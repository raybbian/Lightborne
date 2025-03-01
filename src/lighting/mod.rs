use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        render_graph::{RenderGraphApp, ViewNodeRunner},
        RenderApp,
    },
};

pub use ambient_light::AmbientLight2d;
pub use occluder::Occluder;
pub use point_light::PointLight2d;

use ambient_light::AmbientLight2dPlugin;
use occluder::OccluderPipelinePlugin;
use point_light::PointLight2dPlugin;
use render::{DeferredLightingLabel, DeferredLightingNode};

mod ambient_light;
mod occluder;
mod point_light;
mod render;

pub struct DeferredLightingPlugin;

impl Plugin for DeferredLightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OccluderPipelinePlugin)
            .add_plugins(AmbientLight2dPlugin)
            .add_plugins(PointLight2dPlugin);

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
}
