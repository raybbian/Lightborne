use bevy::{
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::{vec2, vec3, Affine3, Affine3A},
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        mesh::VertexBufferLayout,
        primitives::Aabb,
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        view::{check_visibility, ViewTarget, VisibilitySystems},
        Render, RenderApp, RenderSet,
    },
    sprite::Mesh2dPipeline,
};
use bytemuck::{Pod, Zeroable};

use super::render::PostProcessRes;

pub struct LineLight2dPlugin;

impl Plugin for LineLight2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<LineLight2d>::default())
            .add_plugins(UniformComponentPlugin::<ExtractLineLight2d>::default())
            .add_systems(
                PostUpdate,
                (
                    calculate_line_light_2d_bounds.in_set(VisibilitySystems::CalculateBounds),
                    check_visibility::<With<LineLight2d>>
                        .in_set(VisibilitySystems::CheckVisibility),
                ),
            );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            prepare_line_light_2d_bind_group.in_set(RenderSet::PrepareBindGroups),
        );
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<LineLight2dPipeline>()
            .init_resource::<LineLight2dBuffers>();
    }
}

#[derive(Component, Default, Clone, Debug)]
#[require(Transform, Visibility)]
pub struct LineLight2d {
    pub color: Vec4,
    pub half_length: f32,
    pub radius: f32,
    pub volumetric_intensity: f32,
}

impl LineLight2d {
    pub fn point(color: Vec4, radius: f32, volumetric_intensity: f32) -> Self {
        Self {
            color,
            half_length: 0.0,
            radius,
            volumetric_intensity,
        }
    }
}

pub fn calculate_line_light_2d_bounds(
    mut commands: Commands,
    q_light_changed: Query<(Entity, &LineLight2d), Changed<LineLight2d>>,
) {
    for (entity, light) in q_light_changed.iter() {
        let aabb = Aabb {
            center: Vec3::ZERO.into(),
            half_extents: Vec2::new(light.half_length + light.radius, light.radius)
                .extend(0.0)
                .into(),
        };
        commands.entity(entity).try_insert(aabb);
    }
}

impl ExtractComponent for LineLight2d {
    type Out = (ExtractLineLight2d, LineLight2dBounds);
    type QueryData = (&'static GlobalTransform, &'static LineLight2d);
    type QueryFilter = ();

    fn extract_component(
        (transform, line_light): QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        // FIXME: don't do computations in extract
        let (scale, rotation, translation) = transform.to_scale_rotation_translation();
        let transform_no_scale =
            Affine3A::from_scale_rotation_translation(scale.signum(), rotation, translation);
        let affine = Affine3::from(&transform_no_scale);
        let (a, b) = affine.inverse_transpose_3x3();

        Some((
            ExtractLineLight2d {
                world_from_local: affine.to_transpose(),
                local_from_world_transpose_a: a,
                local_from_world_transpose_b: b,
                color: line_light.color,
                half_length: line_light.half_length,
                radius: line_light.radius,
                volumetric_intensity: line_light.volumetric_intensity,
            },
            LineLight2dBounds {
                transform: transform.compute_transform(),
                half_length: line_light.half_length,
                radius: line_light.radius,
            },
        ))
    }
}

/// Render world version of [`LineLight2d`].  
#[derive(Component, ShaderType, Clone, Copy, Debug)]
pub struct ExtractLineLight2d {
    world_from_local: [Vec4; 3],
    local_from_world_transpose_a: [Vec4; 2],
    local_from_world_transpose_b: f32,
    color: Vec4,
    pub half_length: f32,
    pub radius: f32,
    volumetric_intensity: f32,
}

#[derive(Component, Clone, Copy)]
pub struct LineLight2dBounds {
    pub transform: Transform,
    pub radius: f32,
    pub half_length: f32,
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct LineLight2dVertex {
    position: Vec3,
    uv: Vec2,
    /// 0 -> inner, 1 -> outer
    variant: u32,
}

impl LineLight2dVertex {
    const fn inner(position: Vec3, uv: Vec2) -> Self {
        LineLight2dVertex {
            position,
            uv,
            variant: 0,
        }
    }
    const fn outer(position: Vec3, uv: Vec2) -> Self {
        LineLight2dVertex {
            position,
            uv,
            variant: 1,
        }
    }
}

#[derive(Resource)]
pub struct LineLight2dBuffers {
    pub vertices: RawBufferVec<LineLight2dVertex>,
    pub indices: RawBufferVec<u32>,
}

pub const LINE_LIGHT_2D_NUM_INDICES: u32 = 18;

static VERTICES: [LineLight2dVertex; 8] = [
    LineLight2dVertex::inner(vec3(-1.0, -1.0, 0.0), vec2(0.5, 0.0)),
    LineLight2dVertex::inner(vec3(1.0, -1.0, 0.0), vec2(0.5, 0.0)),
    LineLight2dVertex::inner(vec3(1.0, 1.0, 0.0), vec2(0.5, 1.0)),
    LineLight2dVertex::inner(vec3(-1.0, 1.0, 0.0), vec2(0.5, 1.0)),
    LineLight2dVertex::outer(vec3(-1.0, -1.0, 0.0), vec2(0.0, 0.0)),
    LineLight2dVertex::outer(vec3(1.0, -1.0, 0.0), vec2(1.0, 0.0)),
    LineLight2dVertex::outer(vec3(1.0, 1.0, 0.0), vec2(1.0, 1.0)),
    LineLight2dVertex::outer(vec3(-1.0, 1.0, 0.0), vec2(0.0, 1.0)),
];

static INDICES: [u32; 18] = [0, 1, 2, 2, 3, 0, 1, 5, 6, 6, 2, 1, 4, 0, 3, 3, 7, 4];

impl FromWorld for LineLight2dBuffers {
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

        LineLight2dBuffers {
            vertices: vbo,
            indices: ibo,
        }
    }
}

pub fn line_light_bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        "line_light_bind_group_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<ExtractLineLight2d>(true),
        ),
    )
}

#[derive(Resource)]
pub struct LineLight2dBindGroup {
    value: BindGroup,
}

pub fn prepare_line_light_2d_bind_group(
    mut commands: Commands,
    uniforms: Res<ComponentUniforms<ExtractLineLight2d>>,
    pipeline: Res<LineLight2dPipeline>,
    render_device: Res<RenderDevice>,
) {
    if let Some(binding) = uniforms.uniforms().binding() {
        commands.insert_resource(LineLight2dBindGroup {
            value: render_device.create_bind_group(
                "line_light_2d_bind_group",
                &pipeline.layout,
                &BindGroupEntries::single(binding),
            ),
        })
    }
}

pub struct SetLineLight2dBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetLineLight2dBindGroup<I> {
    type Param = SRes<LineLight2dBindGroup>;
    type ViewQuery = ();
    type ItemQuery = Read<DynamicUniformIndex<ExtractLineLight2d>>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(index) = entity else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &param.into_inner().value, &[index.index()]);
        RenderCommandResult::Success
    }
}

pub struct DrawLineLight2d;
impl<P: PhaseItem> RenderCommand<P> for DrawLineLight2d {
    type Param = SRes<LineLight2dBuffers>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let buffers = param.into_inner();

        pass.set_stencil_reference(0); // only render if no occluders here

        pass.set_vertex_buffer(0, buffers.vertices.buffer().unwrap().slice(..));
        pass.set_index_buffer(
            buffers.indices.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.draw_indexed(0..LINE_LIGHT_2D_NUM_INDICES, 0, 0..1);

        RenderCommandResult::Success
    }
}

#[derive(Resource)]
pub struct LineLight2dPipeline {
    pub layout: BindGroupLayout,
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for LineLight2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let post_process_res = world.resource::<PostProcessRes>();
        let post_process_layout = post_process_res.layout.clone();

        let layout = line_light_bind_group_layout(render_device);

        let shader = world.load_asset("shaders/lighting/line_light.wgsl");

        let pos_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<LineLight2dVertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                // Position
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: std::mem::offset_of!(LineLight2dVertex, position) as u64,
                    shader_location: 0,
                },
                // UV
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: std::mem::offset_of!(LineLight2dVertex, uv) as u64,
                    shader_location: 1,
                },
                // Variant (Inner vs Outer vertex)
                VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: std::mem::offset_of!(LineLight2dVertex, variant) as u64,
                    shader_location: 2,
                },
            ],
        };

        let mesh2d_pipeline = Mesh2dPipeline::from_world(world);

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("line_light_pipeline".into()),
                    layout: vec![
                        post_process_layout,
                        mesh2d_pipeline.view_layout,
                        layout.clone(),
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
                                alpha: BlendComponent {
                                    src_factor: BlendFactor::One,
                                    dst_factor: BlendFactor::One,
                                    operation: BlendOperation::Max,
                                },
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

        LineLight2dPipeline {
            layout,
            pipeline_id,
        }
    }
}

// WebGL2 requires thes structs be 16-byte aligned
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn line_light_2d_alignment() {
        assert_eq!(mem::size_of::<ExtractLineLight2d>() % 16, 0);
    }
}
