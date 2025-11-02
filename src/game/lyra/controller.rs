use avian2d::{math::*, prelude::*};
use bevy::{ecs::query::Has, prelude::*};

use crate::{
    game::{
        lyra::{Lyra, LyraWallCaster},
        LevelSystems,
    },
    shared::PlayState,
};

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
const PLAYER_MOVE_VEL: f32 = 0.4;
/// The y velocity subtracted from the player due to gravity.
const PLAYER_GRAVITY: f32 = 0.15;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MovementAction>();
        app.add_systems(Update, keyboard_input.in_set(LevelSystems::Input));
        app.add_systems(
            FixedUpdate,
            update_grounded
                .before(movement)
                .in_set(LevelSystems::Simulation),
        );
        app.add_systems(FixedUpdate, movement.in_set(LevelSystems::Simulation));
        app.add_systems(OnExit(PlayState::Playing), cache_linear_vel);
        app.add_systems(OnEnter(PlayState::Playing), res_linear_vel);
    }
}

#[derive(Component)]
pub struct CachedLinearVelocity(pub Vector);

pub fn cache_linear_vel(
    lyra: Single<(&mut LinearVelocity, &mut CachedLinearVelocity), With<Lyra>>,
) {
    let (mut linvel, mut cache) = lyra.into_inner();
    cache.0 = linvel.0;
    linvel.0 = Vec2::ZERO;
}

pub fn res_linear_vel(lyra: Single<(&mut LinearVelocity, &mut CachedLinearVelocity), With<Lyra>>) {
    let (mut linvel, mut cache) = lyra.into_inner();
    linvel.0 = cache.0;
    cache.0 = Vec2::ZERO;
}

/// A [`Message`] written for a movement input action.
#[derive(Message)]
pub enum MovementAction {
    Move(Scalar),
    Jump,
    JumpCut,
    Crouch,
    Stand,
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

/// A bundle that contains components for character movement.
#[derive(Component, Default)]
pub struct MovementInfo {
    pub should_jump_ticks: isize,
    pub coyote_time_ticks: isize,
    pub jump_boost_ticks: isize,
    pub crouched: bool,
}

pub fn keyboard_input(
    mut movement_writer: MessageWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let horizontal = right as i8 - left as i8;
    let direction = horizontal as Scalar;

    if direction != 0.0 {
        movement_writer.write(MovementAction::Move(direction));
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_writer.write(MovementAction::Jump);
    }
    if keyboard_input.just_released(KeyCode::Space) {
        movement_writer.write(MovementAction::JumpCut);
    }
    if keyboard_input.just_pressed(KeyCode::ControlLeft) {
        movement_writer.write(MovementAction::Crouch);
    }
    if keyboard_input.just_released(KeyCode::ControlLeft) {
        movement_writer.write(MovementAction::Stand);
    }
}

pub fn update_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits), With<CharacterController>>,
) {
    for (entity, hits) in &mut query {
        let is_grounded = hits.iter().next().is_some();
        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

pub fn movement(
    mut movement_reader: MessageReader<MovementAction>,
    lyra: Single<
        (
            &mut MovementInfo,
            &mut LinearVelocity,
            &ShapeHits,
            Has<Grounded>,
        ),
        With<Lyra>,
    >,
    wall_casters: Query<(&ShapeHits, &LyraWallCaster), Without<Lyra>>,
) {
    let (mut movement_info, mut linear_velocity, shape_hits, is_grounded) = lyra.into_inner();
    if is_grounded {
        movement_info.coyote_time_ticks = COYOTE_TIME_TICKS;
    }

    let mut moved = false;
    for event in movement_reader.read() {
        match event {
            MovementAction::Move(direction) => {
                linear_velocity.x += *direction * PLAYER_MOVE_VEL * 64.;
                moved = true;
            }
            MovementAction::Jump => {
                movement_info.should_jump_ticks = SHOULD_JUMP_TICKS;
            }
            MovementAction::JumpCut => {
                if linear_velocity.y > 0. {
                    linear_velocity.y /= 3.;
                    movement_info.jump_boost_ticks = 0;
                    movement_info.should_jump_ticks = 0;
                }
            }
            MovementAction::Crouch => movement_info.crouched = true,
            MovementAction::Stand => movement_info.crouched = false,
        }
    }

    if movement_info.should_jump_ticks > 0 && movement_info.coyote_time_ticks > 0 {
        movement_info.jump_boost_ticks = JUMP_BOOST_TICKS;
    }

    let too_close = shape_hits.iter().any(|hit| hit.distance < 0.25);
    if movement_info.jump_boost_ticks > 0 {
        linear_velocity.y = PLAYER_JUMP_VEL * 64.;
    } else if too_close && linear_velocity.y < 0.5 {
        linear_velocity.y = 0.45;
    } else if is_grounded && linear_velocity.y < 0.5 {
        linear_velocity.y = 0.;
    } else {
        linear_velocity.y -= PLAYER_GRAVITY * 64.;
    }

    linear_velocity.y = linear_velocity
        .y
        .clamp(-PLAYER_MAX_Y_VEL * 64., PLAYER_MAX_Y_VEL * 64.);

    if !moved {
        linear_velocity.x *= 0.6;
        if linear_velocity.x.abs() < 0.1 {
            linear_velocity.x = 0.;
        }
    }

    for (wall_hits, side) in wall_casters.iter() {
        let too_close = wall_hits.iter().any(|hit| hit.distance < 0.25);
        let any_hit = wall_hits.iter().next().is_some();
        match side {
            LyraWallCaster::Left => {
                if too_close && linear_velocity.x < 0.5 {
                    linear_velocity.x = 0.45;
                } else if any_hit && linear_velocity.x < 0.5 {
                    linear_velocity.x = 0.;
                }
            }
            LyraWallCaster::Right => {
                if too_close && linear_velocity.x > -0.5 {
                    linear_velocity.x = -0.45;
                } else if any_hit && linear_velocity.x > -0.5 {
                    linear_velocity.x = 0.;
                }
            }
        }
    }

    let crouch_modif = if movement_info.crouched { 0.5 } else { 1.0 };
    linear_velocity.x = linear_velocity.x.clamp(
        -PLAYER_MAX_H_VEL * 64. * crouch_modif,
        PLAYER_MAX_H_VEL * 64. * crouch_modif,
    );

    movement_info.should_jump_ticks -= 1;
    movement_info.jump_boost_ticks -= 1;
    movement_info.coyote_time_ticks -= 1;
}
