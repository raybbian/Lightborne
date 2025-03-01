use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::RenderDevice,
        view::ViewTarget,
        RenderApp,
    },
};

pub struct AmbientLight2dPlugin;

impl Plugin for AmbientLight2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<AmbientLight2d>::default())
            .add_plugins(UniformComponentPlugin::<AmbientLight2d>::default());
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
pub struct AmbientLight2dPipeline {
    pub layout: BindGroupLayout,
    pub scene_sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
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
