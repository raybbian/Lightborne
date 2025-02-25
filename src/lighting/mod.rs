use bevy::prelude::*;
use pipeline::DeferredLightingPipelinePlugin;

pub use occluder::Occluder;
pub use pipeline::AmbientLight2d;
pub use pipeline::PointLight2d;

mod occluder;
mod pipeline;

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DeferredLightingPipelinePlugin);
    }
}
