use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    ecs::query::QueryItem,
    math::{vec3, Affine3},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        texture::TextureCache,
        view::{ViewDepthTexture, ViewTarget},
        Render, RenderApp, RenderSet,
    },
    sprite::Mesh2dPipeline,
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

use super::{
    pipeline::{point_light_bind_group_layout, PointLight2dBounds},
    AmbientLight2d,
};

pub const DEBUG_OCCLUDERS: bool = false;

pub struct OccluderPipelinePlugin;

impl Plugin for OccluderPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UniformComponentPlugin::<RenderOccluder>::default())
            .add_plugins(ExtractComponentPlugin::<Occluder>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            prepare_occluder_count_textures.in_set(RenderSet::PrepareResources),
        );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<OccluderPipeline>()
            .init_resource::<OccluderBuffers>();
    }
}

#[derive(Component)]
#[require(Transform)]
pub struct Occluder {
    pub half_size: Vec2,
}

impl Occluder {
    pub fn new(half_x: f32, half_y: f32) -> Self {
        Self {
            half_size: Vec2::new(half_x, half_y),
        }
    }
}

impl ExtractComponent for Occluder {
    type Out = (RenderOccluder, OccluderBounds);
    type QueryData = (&'static GlobalTransform, &'static Occluder);
    type QueryFilter = ();

    fn extract_component(
        (transform, occluder): QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        let affine_a = transform.affine();
        let affine = Affine3::from(&affine_a);
        let (a, b) = affine.inverse_transpose_3x3();

        Some((
            RenderOccluder {
                world_from_local: affine.to_transpose(),
                local_from_world_transpose_a: a,
                local_from_world_transpose_b: b,
                half_size: occluder.half_size,
            },
            OccluderBounds {
                world_pos: affine_a.translation.xy(),
                half_size: occluder.half_size,
            },
        ))
    }
}

/// Render world version of [`PointLight2d`].
#[derive(Component, ShaderType, Clone, Copy, Debug)]
pub struct RenderOccluder {
    world_from_local: [Vec4; 3],
    local_from_world_transpose_a: [Vec4; 2],
    local_from_world_transpose_b: f32,
    half_size: Vec2,
}

#[derive(Component, Clone, Copy)]
pub struct OccluderBounds {
    pub world_pos: Vec2,
    pub half_size: Vec2,
}

impl OccluderBounds {
    pub fn visible_from_point_light(&self, light: &PointLight2dBounds) -> bool {
        let min_rect = self.world_pos - self.half_size;
        let max_rect = self.world_pos + self.half_size;

        let closest_point = light.world_pos.clamp(min_rect, max_rect);

        light.world_pos.distance_squared(closest_point) <= light.radius * light.radius
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct OccluderVertex {
    position: Vec3,
    normal: Vec3,
}

impl OccluderVertex {
    const fn new(position: Vec3, normal: Vec3) -> Self {
        OccluderVertex { position, normal }
    }
}

#[derive(Resource)]
pub struct OccluderBuffers {
    pub vertices: RawBufferVec<OccluderVertex>,
    pub indices: RawBufferVec<u32>,
}

static VERTICES: [OccluderVertex; 8] = [
    OccluderVertex::new(vec3(-1.0, -1.0, 0.0), vec3(-1.0, 0.0, 0.0)),
    OccluderVertex::new(vec3(-1.0, -1.0, 0.0), vec3(0.0, -1.0, 0.0)),
    OccluderVertex::new(vec3(1.0, -1.0, 0.0), vec3(0.0, -1.0, 0.0)),
    OccluderVertex::new(vec3(1.0, -1.0, 0.0), vec3(1.0, 0.0, 0.0)),
    OccluderVertex::new(vec3(1.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0)),
    OccluderVertex::new(vec3(1.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0)),
    OccluderVertex::new(vec3(-1.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0)),
    OccluderVertex::new(vec3(-1.0, 1.0, 0.0), vec3(-1.0, 0.0, 0.0)),
];

static INDICES: [u32; 18] = [0, 1, 2, 2, 3, 4, 4, 5, 6, 6, 7, 0, 0, 2, 4, 4, 6, 0];

impl FromWorld for OccluderBuffers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let mut vbo = RawBufferVec::new(BufferUsages::VERTEX);
        let mut ibo = RawBufferVec::new(BufferUsages::INDEX);

        for vtx in &VERTICES {
            vbo.push(*vtx);
        }
        for index in &INDICES {
            ibo.push(*index);
        }

        vbo.write_buffer(render_device, render_queue);
        ibo.write_buffer(render_device, render_queue);

        OccluderBuffers {
            vertices: vbo,
            indices: ibo,
        }
    }
}

#[derive(Component)]
pub struct OccluderCountTexture(pub ViewDepthTexture);

/// Prepare my own texture because theirs has funny sample count??
pub fn prepare_occluder_count_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), (With<Camera2d>, With<AmbientLight2d>)>,
) {
    let mut textures = HashMap::default();
    for (view, camera) in &views {
        let Some(physical_target_size) = camera.physical_target_size else {
            continue;
        };

        let cached_texture = textures
            .entry(camera.target.clone())
            .or_insert_with(|| {
                // The size of the depth texture
                let size = Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_target_size.x,
                    height: physical_target_size.y,
                };

                let descriptor = TextureDescriptor {
                    label: Some("occluder_count_texture"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Stencil8,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                };

                texture_cache.get(&render_device, descriptor)
            })
            .clone();

        commands
            .entity(view)
            .insert(OccluderCountTexture(ViewDepthTexture::new(
                cached_texture,
                Some(0.0),
            )));
    }
}

#[derive(Resource)]
pub struct OccluderPipeline {
    pub layout: BindGroupLayout,
    pub shadow_pipeline_id: CachedRenderPipelineId,
    pub cutout_pipeline_id: CachedRenderPipelineId,
    pub reset_pipeline_id: CachedRenderPipelineId,
}

pub fn build_occluder_pipeline_descriptor(
    world: &mut World,
    cutout: bool,
    occluder_layout: &BindGroupLayout,
) -> RenderPipelineDescriptor {
    let render_device = world.resource::<RenderDevice>();

    let point_light_layout = point_light_bind_group_layout(render_device);

    let shader = world.load_asset("shaders/lighting/occluder.wgsl");

    let mesh2d_pipeline = Mesh2dPipeline::from_world(world);

    let pos_buffer_layout = VertexBufferLayout {
        array_stride: std::mem::size_of::<OccluderVertex>() as u64,
        step_mode: VertexStepMode::Vertex,
        attributes: vec![
            // Position
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: std::mem::offset_of!(OccluderVertex, position) as u64,
                shader_location: 0,
            },
            // Normals
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: std::mem::offset_of!(OccluderVertex, normal) as u64,
                shader_location: 1,
            },
        ],
    };

    let mut shader_defs: Vec<ShaderDefVal> = vec![];
    if DEBUG_OCCLUDERS {
        shader_defs.push("DEBUG_OCCLUDERS".into());
    }
    if cutout {
        shader_defs.push("OCCLUDER_CUTOUT".into());
    }

    let label = if cutout {
        Some("occluder_cutout_pipeline".into())
    } else {
        Some("occluder_pipeline".into())
    };

    RenderPipelineDescriptor {
        label,
        layout: vec![
            mesh2d_pipeline.view_layout,
            point_light_layout,
            occluder_layout.clone(),
        ],
        vertex: VertexState {
            shader: shader.clone(),
            shader_defs: shader_defs.clone(),
            entry_point: "vertex".into(),
            buffers: vec![pos_buffer_layout],
        },
        fragment: Some(FragmentState {
            shader,
            shader_defs,
            entry_point: "fragment".into(),
            targets: vec![Some(ColorTargetState {
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                blend: Some(BlendState {
                    color: BlendComponent::REPLACE,
                    alpha: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALPHA,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Stencil8,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Always,
            stencil: StencilState {
                front: StencilFaceState {
                    compare: CompareFunction::Always,
                    fail_op: StencilOperation::Keep,
                    depth_fail_op: StencilOperation::Keep,
                    pass_op: if cutout {
                        StencilOperation::Zero
                    } else {
                        StencilOperation::IncrementClamp
                    },
                },
                back: StencilFaceState::default(),
                read_mask: 0xFF,
                write_mask: 0xFF,
            },
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState::default(),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: false,
    }
}

impl FromWorld for OccluderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let occluder_layout = render_device.create_bind_group_layout(
            "point_light_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // occluder settings
                    uniform_buffer::<RenderOccluder>(true),
                ),
            ),
        );

        let reset_shader = world.load_asset("shaders/lighting/occluder_reset.wgsl");

        let shadow_pipeline_descriptor =
            build_occluder_pipeline_descriptor(world, false, &occluder_layout);
        let cutout_pipeline_descriptor =
            build_occluder_pipeline_descriptor(world, true, &occluder_layout);

        let pipeline_cache = world.resource_mut::<PipelineCache>();
        let shadow_pipeline_id = pipeline_cache.queue_render_pipeline(shadow_pipeline_descriptor);
        let cutout_pipeline_id = pipeline_cache.queue_render_pipeline(cutout_pipeline_descriptor);

        let reset_pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("occluder_reset_pipeline".into()),
            layout: vec![],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: reset_shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: ViewTarget::TEXTURE_FORMAT_HDR,
                    blend: Some(BlendState {
                        color: BlendComponent::REPLACE,
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALPHA,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Stencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Always,
                        fail_op: StencilOperation::Zero,
                        depth_fail_op: StencilOperation::Zero,
                        pass_op: StencilOperation::Zero,
                    },
                    back: StencilFaceState::default(),
                    read_mask: 0xFF,
                    write_mask: 0xFF,
                },
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        });

        OccluderPipeline {
            layout: occluder_layout,
            shadow_pipeline_id,
            cutout_pipeline_id,
            reset_pipeline_id,
        }
    }
}
