use bevy::{
    ecs::query::QueryItem,
    math::{vec2, vec3, Affine3},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        mesh::VertexBufferLayout,
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        view::ViewTarget,
        RenderApp,
    },
    sprite::Mesh2dPipeline,
};
use bytemuck::{Pod, Zeroable};

pub struct PointLight2dPlugin;

impl Plugin for PointLight2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<PointLight2d>::default())
            .add_plugins(UniformComponentPlugin::<RenderPointLight2d>::default());
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<PointLight2dPipeline>()
            .init_resource::<PointLight2dBuffers>();
    }
}

#[derive(Component, Default)]
#[require(Transform)]
pub struct PointLight2d {
    pub color: Vec4,
    pub radius: f32,
    pub volumetric_intensity: f32,
}

impl ExtractComponent for PointLight2d {
    type Out = (RenderPointLight2d, PointLight2dBounds);
    type QueryData = (&'static GlobalTransform, &'static PointLight2d);
    type QueryFilter = ();

    fn extract_component(
        (transform, point_light): QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        let affine_a = transform.affine();
        let affine = Affine3::from(&affine_a);
        let (a, b) = affine.inverse_transpose_3x3();

        Some((
            RenderPointLight2d {
                world_from_local: affine.to_transpose(),
                local_from_world_transpose_a: a,
                local_from_world_transpose_b: b,
                color: point_light.color,
                radius: point_light.radius,
                volumetric_intensity: point_light.volumetric_intensity,
            },
            PointLight2dBounds {
                world_pos: affine_a.translation.xy(),
                radius: point_light.radius,
            },
        ))
    }
}

/// Render world version of [`PointLight2d`].  
#[derive(Component, ShaderType, Clone, Copy, Debug)]
pub struct RenderPointLight2d {
    world_from_local: [Vec4; 3],
    local_from_world_transpose_a: [Vec4; 2],
    local_from_world_transpose_b: f32,
    color: Vec4,
    pub radius: f32,
    volumetric_intensity: f32,
}

#[derive(Component, Clone, Copy)]
pub struct PointLight2dBounds {
    pub world_pos: Vec2,
    pub radius: f32,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct PointLight2dVertex {
    position: Vec3,
    uv: Vec2,
}

impl PointLight2dVertex {
    const fn new(position: Vec3, uv: Vec2) -> Self {
        PointLight2dVertex { position, uv }
    }
}

#[derive(Resource)]
pub struct PointLight2dBuffers {
    pub vertices: RawBufferVec<PointLight2dVertex>,
    pub indices: RawBufferVec<u32>,
}

static VERTICES: [PointLight2dVertex; 4] = [
    PointLight2dVertex::new(vec3(-1.0, -1.0, 0.0), vec2(0.0, 0.0)),
    PointLight2dVertex::new(vec3(1.0, -1.0, 0.0), vec2(1.0, 0.0)),
    PointLight2dVertex::new(vec3(1.0, 1.0, 0.0), vec2(1.0, 1.0)),
    PointLight2dVertex::new(vec3(-1.0, 1.0, 0.0), vec2(0.0, 1.0)),
];

static INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

impl FromWorld for PointLight2dBuffers {
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

        PointLight2dBuffers {
            vertices: vbo,
            indices: ibo,
        }
    }
}
pub fn point_light_bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        "point_light_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                //light settings
                uniform_buffer::<RenderPointLight2d>(true),
            ),
        ),
    )
}

#[derive(Resource)]
pub struct PointLight2dPipeline {
    pub bind_layout: BindGroupLayout,
    pub frag_layout: BindGroupLayout,
    pub scene_sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for PointLight2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_layout = point_light_bind_group_layout(render_device);
        let frag_layout = render_device.create_bind_group_layout(
            "point_light_frag_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // unlit scene
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                ),
            ),
        );

        let scene_sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let shader = world.load_asset("shaders/lighting/point_light.wgsl");

        let pos_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<PointLight2dVertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                // Position
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: std::mem::offset_of!(PointLight2dVertex, position) as u64,
                    shader_location: 0,
                },
                // UV
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: std::mem::offset_of!(PointLight2dVertex, uv) as u64,
                    shader_location: 1,
                },
            ],
        };

        let mesh2d_pipeline = Mesh2dPipeline::from_world(world);

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("point_light_pipeline".into()),
                    layout: vec![
                        mesh2d_pipeline.view_layout,
                        bind_layout.clone(),
                        frag_layout.clone(),
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
                            blend: Some(BlendState {
                                color: BlendComponent {
                                    src_factor: BlendFactor::One,
                                    dst_factor: BlendFactor::One,
                                    operation: BlendOperation::Add,
                                },
                                alpha: BlendComponent::OVER,
                            }),
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
                                compare: CompareFunction::Equal,
                                fail_op: StencilOperation::Keep,
                                depth_fail_op: StencilOperation::Keep,
                                pass_op: StencilOperation::Keep,
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

        PointLight2dPipeline {
            bind_layout,
            frag_layout,
            scene_sampler,
            pipeline_id,
        }
    }
}
