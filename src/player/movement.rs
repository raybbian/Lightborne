use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::level::LevelSystems;

use super::{not_input_locked, InputLocked, PlayerMarker};

/// The number of [`FixedUpdate`] steps the player can jump for after pressing the spacebar.
const SHOULD_JUMP_TICKS: isize = 8;
/// The number of [`FixedUpdate`] steps the player can jump for after falling off an edge.
const COYOTE_TIME_TICKS: isize = 5;
/// The number of [`FixedUpdate`] steps the player should receive upward velocity for.
const JUMP_BOOST_TICKS: isize = 2;

/// Max player horizontal velocity.
const PLAYER_MAX_H_VEL: f32 = 1.5;
/// Max player vertical velocity.
const PLAYER_MAX_Y_VEL: f32 = 5.;
/// The positive y velocity added to the player every jump boost tick.
const PLAYER_JUMP_VEL: f32 = 2.2;
/// The x velocity added to the player when A/D is held.
const PLAYER_MOVE_VEL: f32 = 0.6;
/// The y velocity subtracted from the player due to gravity.
const PLAYER_GRAVITY: f32 = 0.15;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            move_player
                .before(PhysicsSet::SyncBackend)
                .in_set(LevelSystems::Simulation),
        )
        .add_systems(
            Update,
            queue_jump
                .run_if(not_input_locked)
                .run_if(input_just_pressed(KeyCode::Space).or(input_just_pressed(KeyCode::KeyW)))
                .before(move_player)
                .in_set(LevelSystems::Simulation),
        )
        .add_systems(
            Update,
            crouch_player
                .run_if(not_input_locked)
                .before(move_player)
                .in_set(LevelSystems::Simulation),
        );
    }
}

/// [`Component`] that stores information about the player's movement state.
#[derive(Component, Default)]
pub struct PlayerMovement {
    /// Holds information that is passed into the rapier character controller's translation
    pub velocity: Vec2,
    pub crouching: bool,
    pub sneaking: bool,
    should_jump_ticks_remaining: isize,
    coyote_time_ticks_remaining: isize,
    jump_boost_ticks_remaining: isize,
}

/// [`System`] that is run the frame the space bar is pressed. Allows the player to jump for the
/// next couple of frames.
pub fn queue_jump(mut q_player: Query<&mut PlayerMovement, With<PlayerMarker>>) {
    let Ok(mut player) = q_player.get_single_mut() else {
        return;
    };
    player.should_jump_ticks_remaining = SHOULD_JUMP_TICKS;
}

/// [`System`] that is run on [`Update`] to crouch player
pub fn crouch_player(
    // query transform
    mut q_player: Query<(&mut PlayerMovement, &mut Collider), With<PlayerMarker>>,
    //ButtonInput<KeyCode> resource (access resource)
    keys: Res<ButtonInput<KeyCode>>,
) {
    // ensure only 1 candidate to match query; let Ok = pattern matching
    let Ok((mut player, mut _collider)) = q_player.get_single_mut() else {
        return;
    };

    // TODO: fix colliders (both player and hurtbox)
    if keys.just_pressed(KeyCode::KeyS) && !player.crouching {
        // decrease size by half
        player.crouching = true;
    }
    if keys.just_released(KeyCode::KeyS) && player.crouching {
        player.crouching = false;
    }
}

/// [`System`] that is run on [`Update`] to move the player around.
pub fn move_player(
    mut q_player: Query<
        (
            &mut KinematicCharacterController,
            &KinematicCharacterControllerOutput,
            &mut PlayerMovement,
            Option<&InputLocked>,
        ),
        With<PlayerMarker>,
    >,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((mut controller, output, mut player, movement_locked)) = q_player.get_single_mut()
    else {
        return;
    };

    let check_pressed = |key: KeyCode| {
        if movement_locked.is_some() {
            return false;
        }
        keys.pressed(key)
    };

    if output.grounded {
        player.coyote_time_ticks_remaining = COYOTE_TIME_TICKS;
    }

    // Can only jump if they've pressed space within the past SHOULD_JUMP_TICKS, and they have been
    // grounded in the past COYOTE_TIME_TICKS
    if player.should_jump_ticks_remaining > 0 && player.coyote_time_ticks_remaining > 0 {
        player.jump_boost_ticks_remaining = JUMP_BOOST_TICKS;
    } else if !check_pressed(KeyCode::Space)
        && !check_pressed(KeyCode::KeyW)
        && player.velocity.y > 0.
    {
        // Jump was cut
        player.velocity.y = PLAYER_GRAVITY;
        player.jump_boost_ticks_remaining = 0;
    } else if output.desired_translation.y > 0. && output.effective_translation.y < 0.05 {
        // Bonked head onto wall
        player.velocity.y = 0.;
        player.jump_boost_ticks_remaining = 0;
    } else if output.grounded {
        player.velocity.y = 0.;
    }

    if player.jump_boost_ticks_remaining > 0 {
        player.velocity.y = PLAYER_JUMP_VEL;
    } else {
        player.velocity.y -= PLAYER_GRAVITY;
    }

    player.velocity.y = player.velocity.y.clamp(-PLAYER_MAX_Y_VEL, PLAYER_MAX_Y_VEL);

    let mut moved = false;
    if check_pressed(KeyCode::KeyA) {
        player.velocity.x -= PLAYER_MOVE_VEL;
        moved = true;
    }
    if check_pressed(KeyCode::KeyD) {
        player.velocity.x += PLAYER_MOVE_VEL;
        moved = true;
    }

    player.sneaking = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let temp_max_h_vel = if player.sneaking {
        PLAYER_MAX_H_VEL / 2.
    } else {
        PLAYER_MAX_H_VEL
    };
    player.velocity.x = player.velocity.x.clamp(-temp_max_h_vel, temp_max_h_vel);
    if !moved {
        // slow player down when not moving horizontally
        // NOTE: why not using rapier friction?
        player.velocity.x *= 0.6;
        if player.velocity.x.abs() < 0.1 {
            player.velocity.x = 0.;
        }
    }

    player.should_jump_ticks_remaining -= 1;
    player.jump_boost_ticks_remaining -= 1;
    player.coyote_time_ticks_remaining -= 1;

    controller.translation = Some(player.velocity);
}
