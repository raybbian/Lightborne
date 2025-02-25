use bevy::{
    math::{vec3, Affine3},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_component::UniformComponentPlugin,
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        sync_world::{RenderEntity, SyncToRenderWorld},
        texture::TextureCache,
        view::{ViewDepthTexture, ViewTarget},
        Extract, Render, RenderApp, RenderSet,
    },
    sprite::Mesh2dPipeline,
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

use super::{
    pipeline::{point_light_bind_group_layout, PointLight2dBounds},
    AmbientLight2d,
};

pub struct OccluderPipelinePlugin;

impl Plugin for OccluderPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UniformComponentPlugin::<RenderOccluder>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(ExtractSchedule, extract_occluders)
            .add_systems(
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

#[derive(Component, Default)]
#[require(Transform, SyncToRenderWorld)]
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

pub fn extract_occluders(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(&RenderEntity, &GlobalTransform, &Occluder)>>,
) {
    let mut values = Vec::with_capacity(*previous_len);

    for (render_entity, transform, occluder) in query.iter() {
        let affine_a = transform.affine();
        let affine = Affine3::from(&affine_a);
        let (a, b) = affine.inverse_transpose_3x3();
        values.push((
            render_entity.id(),
            (
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
            ),
        ))
    }

    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
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
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for OccluderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let occluder_layout = render_device.create_bind_group_layout(
            "point_light_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX,
                (
                    // occluder settings
                    uniform_buffer::<RenderOccluder>(true),
                ),
            ),
        );
        let point_light_layout = point_light_bind_group_layout(render_device);

        let shader = world.load_asset("shaders/lighting/occluder.wgsl");

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

        let mesh2d_pipeline = Mesh2dPipeline::from_world(world);

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("occluder_pipeline".into()),
                    layout: vec![
                        mesh2d_pipeline.view_layout,
                        point_light_layout,
                        occluder_layout.clone(),
                    ],
                    vertex: VertexState {
                        shader: shader.clone(),
                        shader_defs: vec![],
                        entry_point: "vertex".into(),
                        buffers: vec![pos_buffer_layout],
                    },
                    fragment: Some(FragmentState {
                        shader,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: ViewTarget::TEXTURE_FORMAT_HDR,
                            // blend: Some(BlendState {
                            //     color: BlendComponent {
                            //         src_factor: BlendFactor::One,
                            //         dst_factor: BlendFactor::Zero,
                            //         operation: BlendOperation::Add,
                            //     },
                            //     alpha: BlendComponent {
                            //         src_factor: BlendFactor::One,
                            //         dst_factor: BlendFactor::Zero,
                            //         operation: BlendOperation::Add,
                            //     },
                            // }),
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    // below needs changing?
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
                                pass_op: StencilOperation::IncrementClamp,
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
            pipeline_id,
        }
    }
}
