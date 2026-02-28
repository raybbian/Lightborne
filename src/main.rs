#[cfg(feature = "dev_mode")]
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};

use bevy::prelude::*;
use bevy::window::{PresentMode, WindowMode};
use bevy::{asset::AssetMetaCheck, diagnostic::LogDiagnosticsPlugin};

use camera::{CameraPlugin, HIGHRES_LAYER};
use config::ConfigPlugin;
use shared::{AnimationState, GameState, UiState};
use sound::SoundPlugin;
use ui::UiPlugin;

use crate::asset::{AssetLoadPlugin, ResourceLoaded};
use crate::game::GamePlugin;
use crate::save::SavePlugin;
use crate::shared::{LeaderboardState, PlayState};

mod asset;
mod callback;
mod camera;
mod config;
mod game;
mod ldtk;
pub mod save;
mod shared;
mod sound;
mod ui;
mod utils;

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Lightborne".into(),
                    name: Some("lightborne".into()),
                    present_mode: PresentMode::AutoNoVsync,
                    canvas: Some("#bevy-container".into()),
                    fit_canvas_to_parent: true,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
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
    );
    app.insert_gizmo_config::<DefaultGizmoConfigGroup>(
        DefaultGizmoConfigGroup,
        GizmoConfig {
            enabled: true,
            render_layers: HIGHRES_LAYER,
            ..Default::default()
        },
    );
    app.add_plugins(AssetLoadPlugin);
    app.add_plugins(ConfigPlugin);
    app.add_plugins(LogDiagnosticsPlugin::default());
    app.add_plugins(SoundPlugin);
    app.add_plugins(CameraPlugin);
    app.add_plugins(UiPlugin);
    app.add_plugins(GamePlugin);
    app.add_plugins(SavePlugin);
    app.insert_state(GameState::Loading);
    app.add_sub_state::<UiState>();
    app.add_sub_state::<PlayState>();
    app.add_sub_state::<AnimationState>();
    app.add_sub_state::<LeaderboardState>();

    #[cfg(feature = "dev_mode")]
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            enabled: true, // Enable the main FPS overlay
            frame_time_graph_config: FrameTimeGraphConfig {
                enabled: true,     // Enable the frame time graph
                min_fps: 120.0,    // Minimum acceptable FPS (shows red below this)
                target_fps: 360.0, // Target FPS (shows green above this)
                ..default()
            },
            ..default()
        },
    });

    app.add_observer(
        |event: On<ResourceLoaded>,
         mut next_game_state: ResMut<NextState<GameState>>,
         mut next_ui_state: ResMut<NextState<UiState>>| match *event {
            ResourceLoaded::Finished => {
                info!("Resources Loaded");
                next_game_state.set(GameState::Ui);
                next_ui_state.set(UiState::StartMenu);
            }
            ResourceLoaded::InProgress { finished, waiting } => {
                info!(
                    "Resources Loading... Waiting: {} Finished: {}",
                    waiting, finished
                );
            }
        },
    );

    app.run();
}
