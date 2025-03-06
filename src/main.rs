use animation::SpriteAnimationPlugin;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy::{asset::AssetMetaCheck, diagnostic::LogDiagnosticsPlugin};
use bevy_rapier2d::prelude::*;

use camera::CameraPlugin;
use config::ConfigPlugin;
use debug::DebugPlugin;
use input::{init_cursor_world_coords, update_cursor_world_coords};
use level::LevelManagementPlugin;
use level_select::LevelSelectPlugin;
use light::LightManagementPlugin;
use lighting::DeferredLightingPlugin;
use particle::ParticlePlugin;
use pause::PausePlugin;
use player::PlayerManagementPlugin;
use shared::{GameState, ResetLevel, UiState};

mod animation;
mod camera;
mod config;
mod debug;
mod input;
mod level;
mod level_select;
mod light;
mod lighting;
mod particle;
mod pause;
mod player;
mod shared;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Lightborne".into(),
                        name: Some("lightborne".into()),
                        present_mode: PresentMode::AutoNoVsync,
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    //https://github.com/bevyengine/bevy_github_ci_template/issues/48
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(bevy_mod_debugdump::CommandLineArgs)
        .add_plugins(ConfigPlugin)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(8.0).in_fixed_schedule())
        .add_plugins(SpriteAnimationPlugin)
        .add_plugins(PlayerManagementPlugin)
        .add_plugins(LevelManagementPlugin)
        .add_plugins(LightManagementPlugin)
        .add_plugins(ParticlePlugin)
        .add_plugins(PausePlugin)
        .add_plugins(LevelSelectPlugin)
        .add_plugins(CameraPlugin)
        .insert_state(GameState::Ui)
        .add_sub_state::<UiState>()
        .insert_state(UiState::LevelSelect)
        .add_plugins(DeferredLightingPlugin)
        .add_plugins(DebugPlugin {
            // physics: true,
            // ambiguity: true,
            ..default()
        })
        .add_event::<ResetLevel>()
        .add_systems(Startup, init_cursor_world_coords)
        .add_systems(Update, update_cursor_world_coords)
        .run();
}
