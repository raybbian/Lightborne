use animation::{set_animation, PlayerAnimationType};
use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use indicator::{add_light_indicator, update_light_indicator, LightIndicatorData};
use match_player::{
    post_update_match_player_pixel, pre_update_match_player_pixel, update_match_player_z,
};
use strand::{add_player_hair_and_cloth, update_player_strand_offsets, update_strand};

use crate::{
    animation::AnimationConfig,
    camera::move_camera,
    input::update_cursor_world_coords,
    level::{
        entity::{adjust_semisolid_colliders, set_semisolid},
        LevelSystems,
    },
    lighting::LineLight2d,
    shared::GameState,
};

use kill::{
    kill_player_on_hurt_intersection, reset_player_on_kill, reset_player_on_level_switch,
    start_kill_animation, KillAnimationCallbacks, KillPlayerEvent,
};
use light::{
    despawn_angle_indicator, handle_color_switch, preview_light_path, shoot_light,
    should_shoot_light, spawn_angle_indicator, PlayerLightInventory,
};
use movement::{crouch_player, move_player, queue_jump, PlayerMovement};
use spawn::{add_player_sensors, init_player_bundle, PlayerHurtMarker};

mod animation;
mod indicator;
mod kill;
pub mod light;
pub mod match_player;
pub mod movement;
mod spawn;
mod strand;

/// [`Plugin`] for anything player based.
pub struct PlayerManagementPlugin;

impl Plugin for PlayerManagementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KillAnimationCallbacks>()
            .init_resource::<LightIndicatorData>()
            .add_event::<KillPlayerEvent>()
            .add_systems(
                PreUpdate,
                add_player_sensors.in_set(LevelSystems::Processing),
            )
            .add_systems(
                PreUpdate,
                add_light_indicator.in_set(LevelSystems::Processing),
            )
            .add_systems(FixedUpdate, update_light_indicator)
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
                crouch_player
                    .before(move_player)
                    .in_set(LevelSystems::Simulation),
            )
            .add_systems(
                Update,
                (
                    handle_color_switch,
                    should_shoot_light::<true>.run_if(input_just_pressed(MouseButton::Left)),
                    should_shoot_light::<false>.run_if(input_just_pressed(MouseButton::Right)),
                    preview_light_path,
                    spawn_angle_indicator.run_if(input_just_pressed(MouseButton::Left)),
                    despawn_angle_indicator.run_if(
                        input_just_released(MouseButton::Left)
                            .or(input_just_pressed(MouseButton::Right)),
                    ),
                    shoot_light.run_if(input_just_released(MouseButton::Left)),
                )
                    .chain()
                    .in_set(LevelSystems::Simulation)
                    .after(update_cursor_world_coords),
            )
            .add_systems(
                Update,
                (
                    reset_player_on_kill.before(move_camera),
                    // LMAO yeah so to reset the hair to a natural state i just simulate it 3 times
                    update_strand,
                    update_strand,
                    update_strand,
                )
                    .chain()
                    .in_set(LevelSystems::Reset),
            )
            .add_systems(
                Update,
                (
                    quick_reset
                        .run_if(input_just_pressed(KeyCode::KeyR))
                        .run_if(in_state(GameState::Playing)),
                    reset_player_on_level_switch.in_set(LevelSystems::Reset),
                ),
            )
            .add_systems(
                FixedUpdate,
                (kill_player_on_hurt_intersection, set_semisolid).in_set(LevelSystems::Simulation),
            )
            .add_systems(
                Update,
                start_kill_animation.run_if(on_event::<KillPlayerEvent>),
            )
            .add_systems(
                PreUpdate,
                adjust_semisolid_colliders.in_set(LevelSystems::Processing),
            )
            .add_systems(FixedUpdate, update_strand.in_set(LevelSystems::Simulation))
            .add_systems(PreUpdate, pre_update_match_player_pixel)
            .add_systems(PostUpdate, post_update_match_player_pixel)
            .add_systems(Update, update_match_player_z)
            .add_systems(
                PreUpdate,
                add_player_hair_and_cloth.in_set(LevelSystems::Processing),
            )
            .add_systems(
                FixedUpdate,
                (set_animation, update_player_strand_offsets)
                    .chain()
                    .in_set(LevelSystems::Simulation),
            );
    }
}

/// [`Component`] to signal our own code to finish the initialization of the player (adding sensors, etc)
#[derive(Component, Default)]
pub struct PlayerMarker;

/// [`Bundle`] that will be initialized with [`init_player_bundle`] and inserted to the player
/// [`Entity`] by Ldtk.
#[derive(Bundle)]
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
    point_lighting: LineLight2d,
    animation_config: AnimationConfig,
    animation_type: PlayerAnimationType,
}

/// [`Bundle`] registered with Ldtk that will be spawned in with the level.
#[derive(Bundle, LdtkEntity)]
pub struct LdtkPlayerBundle {
    #[default]
    player_marker: PlayerMarker,
    #[with(init_player_bundle)]
    player: PlayerBundle,
    #[worldly]
    worldly: Worldly,
    #[from_entity_instance]
    instance: EntityInstance,
}

/// [`System`] that will kill the player on press of the R key
fn quick_reset(mut ev_kill_player: EventWriter<KillPlayerEvent>) {
    ev_kill_player.send(KillPlayerEvent);
}
