use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::systems::process_ldtk_levels;

use crate::{
    camera::HIGHRES_LAYER,
    game::{
        animation::SpriteAnimationPlugin,
        bgm::LevelBgmPlugin,
        camera_op::CameraOpPlugin,
        cursor::CursorCoordsPlugin,
        defs::{one_way_platform::OneWayPlatformHooks, LevelPlugin},
        dialogue::DialoguePlugin,
        level_completion::LevelCompletionPlugin,
        light::LightBeamPlugin,
        lighting::DeferredLightingPlugin,
        lyra::LyraPlugin,
        particle::ParticlePlugin,
        setup::LevelSetupPlugin,
        switch::SwitchLevelPlugin,
    },
    shared::{AnimationState, GameState, PlayState},
};

mod animation;
mod bgm;
mod camera_op;
mod cursor;
pub mod defs;
mod dialogue;
pub mod light;
pub mod lyra;
pub mod setup;
mod switch;
// mod input;
// mod level;
// mod light;
pub mod level_completion;
pub mod lighting;
mod particle;
// mod player;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            PhysicsPlugins::default()
                .with_length_unit(8.)
                .with_collision_hooks::<OneWayPlatformHooks>(),
        );
        // app.add_plugins(PhysicsDebugPlugin);
        // app.add_plugins(PhysicsDiagnosticsPlugin);
        app.add_plugins(CursorCoordsPlugin);
        app.add_plugins(LevelBgmPlugin);
        app.add_plugins(SpriteAnimationPlugin);
        app.add_plugins(LevelCompletionPlugin);
        app.add_plugins(SwitchLevelPlugin);
        app.add_plugins(LevelSetupPlugin);
        app.add_plugins(LyraPlugin);
        app.add_plugins(LevelPlugin);
        app.add_plugins(CameraOpPlugin);
        app.add_plugins(ParticlePlugin);
        app.add_plugins(LightBeamPlugin);
        app.add_plugins(DeferredLightingPlugin);
        app.add_plugins(DialoguePlugin);
        app.insert_resource(Gravity::ZERO);
        app.configure_sets(
            PreUpdate,
            LevelSystems::Processing
                .after(process_ldtk_levels)
                .run_if(in_state(GameState::InGame)),
        );
        app.configure_sets(
            Update,
            LevelSystems::Input.run_if(in_state(PlayState::Playing)),
        );
        app.configure_sets(
            FixedUpdate,
            LevelSystems::Input.run_if(in_state(PlayState::Playing)),
        );
        app.configure_sets(
            Update,
            LevelSystems::Simulation
                .run_if(in_state(PlayState::Playing).or(in_state(AnimationState::InputLocked))),
        );
        app.configure_sets(
            FixedUpdate,
            LevelSystems::Simulation
                .run_if(in_state(PlayState::Playing).or(in_state(AnimationState::InputLocked))),
        );
        app.configure_sets(
            FixedPostUpdate,
            LevelSystems::Simulation
                .run_if(in_state(PlayState::Playing).or(in_state(AnimationState::InputLocked))),
        );
        app.insert_gizmo_config(
            PhysicsGizmos::default(),
            GizmoConfig {
                enabled: true,
                render_layers: HIGHRES_LAYER,
                ..Default::default()
            },
        );
    }
}

#[derive(PhysicsLayer, Default)]
pub enum Layers {
    #[default]
    Default,
    PlayerCollider,
    PlayerHurtbox,
    LightRay,
    WhiteRay,
    BlueRay,
    DangerBox,
    Spike,

    // all terrain types, gotta be separate for casters though
    Terrain,
    Platform,
    BlueCrystal,

    // non collision based, just sensors
    LightSensor,
    SensorBox,
    // BlackRay,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LevelSystems {
    /// Systems used to simulate game logic
    Simulation,
    /// Non-trigger systems that are used to process ldtk entities. Prefer to use triggers.
    Processing,
    Input,
}
