use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
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
        renderer::{RenderContext, RenderDevice},
        view::{ExtractedView, RenderVisibleEntities, ViewTarget},
        RenderApp,
    },
};

use crate::camera::MainCamera;

pub struct LightingPipelinePlugin;

impl Plugin for LightingPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<DeferredPointLight>::default(),
            UniformComponentPlugin::<DeferredPointLight>::default(),
        ));

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

        render_app.init_resource::<DeferredLightingPipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct DeferredLightingLabel;

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
struct DeferredPointLight {
    color: Vec2,
    radius: f32,
    // WebGL2 structs must be 16 byte aligned.
    _wasm_padding: Vec3,
}

#[derive(Resource)]
pub struct DeferredLightingPipeline {
    layout: BindGroupLayout,
    scene_sampler: Sampler,
    normal_sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for DeferredLightingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "lighting_bind_group_layou",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    //no-lighting scene
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    //normal map
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    //light settings
                    uniform_buffer::<DeferredPointLight>(true),
                ),
            ),
        );

        let scene_sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let normal_sampler = render_device.create_sampler(&SamplerDescriptor::default());

        // TODO: add deferred lighting shader
        let shader = world.load_asset("shaders/deferred_lighting.wgsl");

        let pipeline_id = world
            .resource_mut::<PipelineCache>()
            // This will add the pipeline to the cache and queue its creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("post_process_pipeline".into()),
                layout: vec![layout.clone()],
                // TODO: change to point light only
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader,
                    shader_defs: vec![],
                    entry_point: "fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::bevy_default(),
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                // below needs changing?
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
                zero_initialize_workgroup_memory: false,
            });

        DeferredLightingPipeline {
            layout,
            scene_sampler,
            normal_sampler,
            pipeline_id,
        }
    }
}

pub fn queue_deferred_lighting_pipeline(
    pipeline_cache: Res<PipelineCache>,
    deferred_lighting_pipeline: Res<DeferredLightingPipeline>,
    views: Query<(Entity, &RenderVisibleEntities), With<ExtractedView>>,
) {
    for (view_entity, view_visible_entities) in views.iter() {}
}

#[derive(Default)]
pub struct DeferredLightingNode;

impl ViewNode for DeferredLightingNode {
    type ViewQuery = (&'static ViewTarget, &'static MainCamera);

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, _main_camera): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let deferred_lighting_pipeline = world.resource::<DeferredLightingPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) =
            pipeline_cache.get_render_pipeline(deferred_lighting_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        Ok(())
    }
}
