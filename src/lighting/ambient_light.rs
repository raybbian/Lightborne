use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{binding_types::uniform_buffer, *},
        renderer::RenderDevice,
        view::ViewTarget,
        Render, RenderApp, RenderSet,
    },
    sprite::Mesh2dPipeline,
};

use super::render::PostProcessRes;

pub struct AmbientLight2dPlugin;

impl Plugin for AmbientLight2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<AmbientLight2d>::default())
            .add_plugins(UniformComponentPlugin::<AmbientLight2d>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            prepare_ambient_light_2d_bind_group.in_set(RenderSet::PrepareBindGroups),
        );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<AmbientLight2dPipeline>();
    }
}

/// Despite its poor name, cameras must have this component to enable deferred lighting.
#[derive(Component, Debug, ExtractComponent, Clone, Copy, ShaderType)]
pub struct AmbientLight2d {
    pub color: Vec4,
}

#[derive(Resource)]
pub struct AmbientLight2dBindGroup {
    value: BindGroup,
}

pub fn prepare_ambient_light_2d_bind_group(
    mut commands: Commands,
    uniforms: Res<ComponentUniforms<AmbientLight2d>>,
    pipeline: Res<AmbientLight2dPipeline>,
    render_device: Res<RenderDevice>,
) {
    if let Some(binding) = uniforms.uniforms().binding() {
        commands.insert_resource(AmbientLight2dBindGroup {
            value: render_device.create_bind_group(
                "ambient_light_2d_bind_group",
                &pipeline.layout,
                &BindGroupEntries::single(binding),
            ),
        })
    }
}

pub struct SetAmbientLight2dBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetAmbientLight2dBindGroup<I> {
    type Param = SRes<AmbientLight2dBindGroup>;
    type ViewQuery = Read<DynamicUniformIndex<AmbientLight2d>>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &param.into_inner().value, &[view.index()]);
        RenderCommandResult::Success
    }
}

#[derive(Resource)]
pub struct AmbientLight2dPipeline {
    pub layout: BindGroupLayout,
    pub pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for AmbientLight2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let post_process_res = world.resource::<PostProcessRes>();
        let post_process_layout = post_process_res.layout.clone();

        let layout = render_device.create_bind_group_layout(
            "ambient_light_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                uniform_buffer::<AmbientLight2d>(true),
            ),
        );

        let shader = world.load_asset("shaders/lighting/ambient_light.wgsl");

        let mesh2d_pipeline = Mesh2dPipeline::from_world(world);

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("ambient_light_pipeline".into()),
                    layout: vec![
                        post_process_layout,
                        mesh2d_pipeline.view_layout,
                        layout.clone(),
                    ],
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
                                alpha: BlendComponent {
                                    src_factor: BlendFactor::One,
                                    dst_factor: BlendFactor::Zero,
                                    operation: BlendOperation::Add,
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
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                });

        AmbientLight2dPipeline {
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
    fn ambient_light_2d_alignment() {
        assert_eq!(mem::size_of::<AmbientLight2d>() % 16, 0);
    }
}
