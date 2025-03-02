use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released, input_pressed},
    prelude::*,
};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use match_player::{
    post_update_match_player_pixel, pre_update_match_player_pixel, update_match_player_z,
};
use strand::{add_player_hair_and_cloth, update_player_strand_offsets, update_strand};
use indicator::{add_light_indicator, update_light_indicator, LightIndicatorData};

use crate::{
    input::update_cursor_world_coords,
    level::LevelSystems,
    shared::{GameState, ResetLevel},
};

use kill::{kill_player_on_spike, reset_player_on_level_switch, reset_player_position};
use light::{handle_color_switch, preview_light_path, shoot_light, PlayerLightInventory};
use movement::{move_player, queue_jump, PlayerMovement};
use spawn::{add_player_sensors, init_player_bundle, PlayerHurtMarker};

mod kill;
pub mod light;
mod match_player;
pub mod movement;
mod spawn;
mod strand;
mod indicator;

/// [`Plugin`] for anything player based.
pub struct PlayerManagementPlugin;

impl Plugin for PlayerManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            add_player_sensors.in_set(LevelSystems::Processing),
        )
        .add_systems(
            FixedUpdate,
            move_player
                .before(PhysicsSet::SyncBackend)
                .in_set(LevelSystems::Simulation),
        )
        .add_systems(
            Update,
            queue_jump
                .run_if(input_just_pressed(KeyCode::Space))
                .before(move_player)
                .in_set(LevelSystems::Simulation),
        )
        .add_systems(
            Update,
            (
                handle_color_switch,
                preview_light_path.run_if(input_pressed(MouseButton::Left)),
                shoot_light.run_if(input_just_released(MouseButton::Left)),
            )
                .chain()
                .in_set(LevelSystems::Simulation)
                .after(update_cursor_world_coords),
        )
        .add_systems(
            FixedUpdate,
            reset_player_on_level_switch.run_if(on_event::<ResetLevel>),
        )
        .add_systems(FixedUpdate, reset_player_position)
        .add_systems(
            Update,
            quick_reset
                .run_if(input_just_pressed(KeyCode::KeyR))
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, kill_player_on_spike)
        .add_systems(FixedUpdate, update_strand)
        .add_systems(FixedPreUpdate, pre_update_match_player_pixel)
        .add_systems(FixedPostUpdate, post_update_match_player_pixel)
        .add_systems(FixedUpdate, update_match_player_z)
        .add_systems(
            PreUpdate,
            add_player_hair_and_cloth.in_set(LevelSystems::Processing),
        )
        .add_systems(FixedUpdate, update_player_strand_offsets)
        .init_resource::<LightIndicatorData>()
        .add_systems(
            PreUpdate,
            add_light_indicator.in_set(LevelSystems::Processing)
        )
        .add_systems(Update, update_light_indicator);
    }
}

/// [`Component`] to signal our own code to finish the initialization of the player (adding sensors, etc)
#[derive(Component, Default)]
pub struct PlayerMarker;

/// [`Bundle`] that will be initialized with [`init_player_bundle`] and inserted to the player
/// [`Entity`] by Ldtk.
#[derive(Default, Bundle)]
pub struct PlayerBundle {
    body: RigidBody,
    controller: KinematicCharacterController,
    controller_output: KinematicCharacterControllerOutput,
    collider: Collider,
    collision_groups: CollisionGroups,
    friction: Friction,
    restitution: Restitution,
    player_movement: PlayerMovement,
    light_inventory: PlayerLightInventory,
}

/// [`Bundle`] registered with Ldtk that will be spawned in with the level.
#[derive(Default, Bundle, LdtkEntity)]
pub struct LdtkPlayerBundle {
    player_marker: PlayerMarker,
    #[with(init_player_bundle)]
    player: PlayerBundle,
    #[sprite("lyra_bald.png")]
    sprite: Sprite,
    #[worldly]
    worldly: Worldly,
    #[from_entity_instance]
    instance: EntityInstance,
}

/// [`System`] that will cause a state switch to [`GameState::Respawning`] when the "R" key is pressed.
fn quick_reset(mut ev_reset_level: EventWriter<ResetLevel>) {
    ev_reset_level.send(ResetLevel::Respawn);
}
