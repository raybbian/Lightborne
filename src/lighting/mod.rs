use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_phase::{
            sort_phase_system, AddRenderCommand, DrawFunctions, ViewSortedRenderPhases,
        },
        Render, RenderApp, RenderSet,
    },
};

pub use ambient_light::AmbientLight2d;
pub use line_light::LineLight2d;
pub use occluder::Occluder2d;

use ambient_light::AmbientLight2dPlugin;
use line_light::LineLight2dPlugin;
use occluder::Occluder2dPipelinePlugin;
use render::{
    extract_deferred_lighting_2d_camera_phases, queue_deferred_lighting, DeferredLighting2d,
    DeferredLightingLabel, DeferredLightingNode, PostProcessRes, PrepareDeferredLighting,
    PrepareLineLight2d, RenderAmbientLight2d, RenderLineLight2d, RenderOccluder,
    ResetOccluderStencil,
};

mod ambient_light;
mod line_light;
mod occluder;
mod render;

pub struct DeferredLightingPlugin;

impl Plugin for DeferredLightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Occluder2dPipelinePlugin)
            .add_plugins(AmbientLight2dPlugin)
            .add_plugins(LineLight2dPlugin);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DrawFunctions<DeferredLighting2d>>()
            .init_resource::<ViewSortedRenderPhases<DeferredLighting2d>>()
            .add_render_command::<DeferredLighting2d, PrepareDeferredLighting>()
            .add_render_command::<DeferredLighting2d, RenderAmbientLight2d>()
            .add_render_command::<DeferredLighting2d, PrepareLineLight2d>()
            .add_render_command::<DeferredLighting2d, RenderOccluder>()
            .add_render_command::<DeferredLighting2d, RenderLineLight2d>()
            .add_render_command::<DeferredLighting2d, ResetOccluderStencil>()
            .add_systems(ExtractSchedule, extract_deferred_lighting_2d_camera_phases)
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
            )
            .add_systems(
                Render,
                (
                    sort_phase_system::<DeferredLighting2d>.in_set(RenderSet::PhaseSort),
                    queue_deferred_lighting.in_set(RenderSet::QueueMeshes),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<PostProcessRes>();
    }
}
