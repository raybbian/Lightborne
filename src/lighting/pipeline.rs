use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::{query::QueryItem, system::SystemState},
    math::{vec2, vec3, Affine3},
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ViewTarget, ViewUniformOffset},
        RenderApp,
    },
    sprite::{Mesh2dPipeline, Mesh2dViewBindGroup},
};
use bytemuck::{Pod, Zeroable};

use super::occluder::{
    OccluderBounds, OccluderBuffers, OccluderCountTexture, OccluderPipeline,
    OccluderPipelinePlugin, RenderOccluder,
};

pub struct DeferredLightingPipelinePlugin;

impl Plugin for DeferredLightingPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OccluderPipelinePlugin)
            .add_plugins(ExtractComponentPlugin::<AmbientLight2d>::default())
            .add_plugins(UniformComponentPlugin::<AmbientLight2d>::default())
            .add_plugins(ExtractComponentPlugin::<PointLight2d>::default())
            .add_plugins(UniformComponentPlugin::<RenderPointLight2d>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<DeferredLightingNode>>(
                Core2d,
                DeferredLightingLabel,
            )
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::MainTransparentPass,
                    DeferredLightingLabel,
                    Node2d::EndMainPass,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<PointLight2dPipeline>()
            .init_resource::<AmbientLight2dPipeline>()
            .init_resource::<PointLight2dBuffers>();
    }
}

/// Despite its poor name, cameras must have this component to enable deferred lighting.
#[derive(Component, Debug, ExtractComponent, Clone, Copy, ShaderType)]
pub struct AmbientLight2d {
    pub color: Vec4,
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
struct PointLight2dVertex {
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
    vertices: RawBufferVec<PointLight2dVertex>,
    indices: RawBufferVec<u32>,
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct DeferredLightingLabel;

#[derive(Resource)]
pub struct AmbientLight2dPipeline {
    layout: BindGroupLayout,
    scene_sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for AmbientLight2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "ambient_light_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // unlit scene
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    // ambient light settings
                    uniform_buffer::<AmbientLight2d>(true),
                ),
            ),
        );

        let scene_sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let shader = world.load_asset("shaders/lighting/ambient_light.wgsl");

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("ambient_light_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
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
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                });

        AmbientLight2dPipeline {
            layout,
            scene_sampler,
            pipeline_id,
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
    bind_layout: BindGroupLayout,
    frag_layout: BindGroupLayout,
    scene_sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
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

#[derive(Default)]
pub struct DeferredLightingNode {
    point_lights: Vec<(u32, PointLight2dBounds)>,
    occluders: Vec<(u32, OccluderBounds)>,
}

impl ViewNode for DeferredLightingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static OccluderCountTexture,
        &'static ViewUniformOffset,
        &'static Mesh2dViewBindGroup,
        &'static DynamicUniformIndex<AmbientLight2d>,
    );

    fn update(&mut self, world: &mut World) {
        let mut state = SystemState::<(
            Query<(
                &DynamicUniformIndex<RenderPointLight2d>,
                &PointLight2dBounds,
            )>,
            Query<(&DynamicUniformIndex<RenderOccluder>, &OccluderBounds)>,
        )>::new(world);

        let (q_indices, q_occluders) = state.get(world);

        // should this be done every frame?
        self.point_lights.clear();
        for (light_index, light_bounds) in q_indices.iter() {
            self.point_lights.push((light_index.index(), *light_bounds));
        }
        self.occluders.clear();
        for (occluder_index, occluder_bounds) in q_occluders.iter() {
            self.occluders
                .push((occluder_index.index(), *occluder_bounds));
        }
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            view_target,
            occluder_count_texture,
            view_uniform_offset,
            mesh2d_view_bind_group,
            ambient_lighting_index,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let point_light_pipeline_res = world.resource::<PointLight2dPipeline>();
        let ambient_light_pipeline_res = world.resource::<AmbientLight2dPipeline>();
        let occluder_pipeline_res = world.resource::<OccluderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (
            Some(point_light_pipeline),
            Some(ambient_light_pipeline),
            Some(occluder_shadow_pipeline),
            Some(occluder_cutout_pipeline),
            Some(occluder_reset_pipeline),
        ) = (
            pipeline_cache.get_render_pipeline(point_light_pipeline_res.pipeline_id),
            pipeline_cache.get_render_pipeline(ambient_light_pipeline_res.pipeline_id),
            pipeline_cache.get_render_pipeline(occluder_pipeline_res.shadow_pipeline_id),
            pipeline_cache.get_render_pipeline(occluder_pipeline_res.cutout_pipeline_id),
            pipeline_cache.get_render_pipeline(occluder_pipeline_res.reset_pipeline_id),
        )
        else {
            return Ok(());
        };

        let Some(point_light_buffers) = world.get_resource::<PointLight2dBuffers>() else {
            return Ok(());
        };

        let Some(occluder_buffers) = world.get_resource::<OccluderBuffers>() else {
            return Ok(());
        };

        let point_light_2d_uniforms = world.resource::<ComponentUniforms<RenderPointLight2d>>();
        let Some(point_light_uniforms_binding) = point_light_2d_uniforms.uniforms().binding()
        else {
            return Ok(());
        };

        let ambient_light_2d_uniforms = world.resource::<ComponentUniforms<AmbientLight2d>>();
        let Some(ambient_light_2d_uniforms_binding) =
            ambient_light_2d_uniforms.uniforms().binding()
        else {
            return Ok(());
        };

        let occluder_uniforms = world.resource::<ComponentUniforms<RenderOccluder>>();
        let Some(occluder_uniforms_binding) = occluder_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let point_light_bind_group = render_context.render_device().create_bind_group(
            "point_light_bind_group",
            &point_light_pipeline_res.bind_layout,
            &BindGroupEntries::single(point_light_uniforms_binding.clone()),
        );

        let point_light_frag_group = render_context.render_device().create_bind_group(
            "point_light_frag_group",
            &point_light_pipeline_res.frag_layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &point_light_pipeline_res.scene_sampler,
            )),
        );

        let ambient_light_frag_group = render_context.render_device().create_bind_group(
            "ambient_light_frag_group",
            &ambient_light_pipeline_res.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &ambient_light_pipeline_res.scene_sampler,
                ambient_light_2d_uniforms_binding.clone(),
            )),
        );

        let occluder_bind_group = render_context.render_device().create_bind_group(
            "occluder_bind_group",
            &occluder_pipeline_res.layout,
            &BindGroupEntries::single(occluder_uniforms_binding.clone()),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("deferred_lighting_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: occluder_count_texture.0.view(),
                depth_ops: None,
                stencil_ops: Some(Operations {
                    load: LoadOp::Clear(0),
                    store: StoreOp::Discard,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Draw the ambient light first
        render_pass.set_render_pipeline(ambient_light_pipeline);
        render_pass.set_bind_group(
            0,
            &ambient_light_frag_group,
            &[ambient_lighting_index.index()],
        );
        render_pass.draw(0..3, 0..1);

        render_pass.set_bind_group(
            0,
            &mesh2d_view_bind_group.value,
            &[view_uniform_offset.offset],
        );

        for (light_index, light_bounds) in self.point_lights.iter() {
            // render occluders
            render_pass.set_bind_group(1, &point_light_bind_group, &[*light_index]);
            render_pass.set_vertex_buffer(0, occluder_buffers.vertices.buffer().unwrap().slice(..));
            render_pass.set_index_buffer(
                occluder_buffers.indices.buffer().unwrap().slice(..),
                0,
                IndexFormat::Uint32,
            );

            // TODO: instanced rendering

            // render occluders for this light
            for (occluder_index, occluder_bounds) in self.occluders.iter() {
                if !occluder_bounds.visible_from_point_light(light_bounds) {
                    continue;
                }

                render_pass.set_render_pipeline(occluder_shadow_pipeline);
                render_pass.set_bind_group(2, &occluder_bind_group, &[*occluder_index]);
                render_pass.draw_indexed(0..18, 0, 0..1);

                // FIXME: stencil buffers are iffy when cutouts are applied

                // decrement the stencil buffer where the occluders are
                render_pass.set_render_pipeline(occluder_cutout_pipeline);
                render_pass.draw_indexed(0..18, 0, 0..1);
            }

            // render the light itself
            render_pass.set_render_pipeline(point_light_pipeline);

            render_pass.set_bind_group(2, &point_light_frag_group, &[]);

            render_pass.set_stencil_reference(0); // only render if no occluders here

            render_pass
                .set_vertex_buffer(0, point_light_buffers.vertices.buffer().unwrap().slice(..));
            render_pass.set_index_buffer(
                point_light_buffers.indices.buffer().unwrap().slice(..),
                0,
                IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..6, 0, 0..1);

            // reset the stencil buffer for the next light
            render_pass.set_render_pipeline(occluder_reset_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
