use bevy::{math::vec2, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::animation::AnimationConfig;

use super::{movement::PlayerMovement, PlayerMarker};

pub const ANIMATION_FRAMES: usize = 25;

#[derive(Component, PartialEq, Eq, Clone, Copy, Default)]
pub enum PlayerAnimationType {
    #[default]
    Idle,
    Walk,
    Crouch,
    Jump,
    Fall,
}

// HAIR, LEFT, RIGHT
const OFFSETS: [[Vec2; 3]; ANIMATION_FRAMES] = [
    [vec2(-2.0, 3.0), vec2(-3.0, -6.0), vec2(4.0, -6.0)], // idle 1
    [vec2(-2.0, 4.0), vec2(-3.0, -5.0), vec2(4.0, -5.0)],
    [vec2(-2.0, 4.0), vec2(-3.0, -5.0), vec2(4.0, -5.0)],
    [vec2(-3.0, 4.0), vec2(-4.0, -6.0), vec2(3.0, -6.0)], // walk 1
    [vec2(-2.0, 4.0), vec2(-3.0, -5.0), vec2(4.0, -5.0)],
    [vec2(-2.0, 4.0), vec2(-3.0, -5.0), vec2(4.0, -5.0)],
    [vec2(-3.0, 4.0), vec2(-2.0, -5.0), vec2(3.0, -5.0)],
    [vec2(-3.0, 4.0), vec2(-3.0, -5.0), vec2(4.0, -5.0)],
    [vec2(-2.0, 4.0), vec2(-4.0, -5.0), vec2(3.0, -5.0)],
    [vec2(-2.0, 4.0), vec2(-3.0, -4.0), vec2(3.0, -5.0)],
    [vec2(-3.0, 4.0), vec2(-3.0, -5.0), vec2(3.0, -5.0)],
    [vec2(-2.0, 3.0), vec2(-3.0, -6.0), vec2(4.0, -6.0)], // crouch 1
    [vec2(-2.0, 2.0), vec2(-3.0, -7.0), vec2(4.0, -6.0)],
    [vec2(-2.0, 1.0), vec2(-3.0, -8.0), vec2(4.0, -7.0)],
    [vec2(-2.0, 0.0), vec2(-3.0, -8.0), vec2(4.0, -8.0)],
    [vec2(-2.0, 2.0), vec2(-3.0, -7.0), vec2(4.0, -6.0)], // jump 1
    [vec2(-2.0, 1.0), vec2(-3.0, -8.0), vec2(4.0, -7.0)],
    [vec2(-2.0, 3.0), vec2(-3.0, -6.0), vec2(3.0, -6.0)],
    [vec2(-2.0, 4.0), vec2(-3.0, -5.0), vec2(3.0, -5.0)],
    [vec2(-1.0, 4.0), vec2(-2.0, -5.0), vec2(3.0, -5.0)],
    [vec2(-2.0, 4.0), vec2(-3.0, -5.0), vec2(3.0, -5.0)],
    [vec2(-2.0, 4.0), vec2(-3.0, -4.0), vec2(3.0, -4.0)], // fall 1
    [vec2(-2.0, 4.0), vec2(-4.0, -4.0), vec2(4.0, -4.0)],
    [vec2(-2.0, 4.0), vec2(-4.0, -3.0), vec2(4.0, -3.0)],
    [vec2(-2.0, 4.0), vec2(-5.0, -2.0), vec2(5.0, -2.0)],
];

impl PlayerAnimationType {
    fn get_offset(&self, index: usize, variant: usize) -> Vec2 {
        OFFSETS[index][variant]
    }
    pub fn hair_offset(&self, index: usize) -> Vec2 {
        self.get_offset(index, 0)
    }
    pub fn left_cloth_offset(&self, index: usize) -> Vec2 {
        self.get_offset(index, 1)
    }
    pub fn right_cloth_offset(&self, index: usize) -> Vec2 {
        self.get_offset(index, 2)
    }
}

impl From<PlayerAnimationType> for AnimationConfig {
    fn from(anim_type: PlayerAnimationType) -> Self {
        match anim_type {
            PlayerAnimationType::Walk => AnimationConfig::new(3, 10, 12, true),
            PlayerAnimationType::Idle => AnimationConfig::new(0, 2, 6, true),
            PlayerAnimationType::Crouch => AnimationConfig::new(11, 14, 48, false),
            PlayerAnimationType::Jump => AnimationConfig::new(15, 20, 24, false),
            PlayerAnimationType::Fall => AnimationConfig::new(21, 23, 24, false),
        }
    }
}

pub fn set_animation(
    mut q_player: Query<
        (
            &PlayerMovement,
            &mut AnimationConfig,
            &mut PlayerAnimationType,
            &KinematicCharacterControllerOutput,
        ),
        With<PlayerMarker>,
    >,
) {
    let Ok((movement, mut config, mut animation, output)) = q_player.get_single_mut() else {
        return;
    };

    let new_anim = if !output.grounded && output.effective_translation.y > 0.0 {
        PlayerAnimationType::Jump
    } else if !output.grounded {
        PlayerAnimationType::Fall
    } else if output.grounded && output.effective_translation.x.abs() > 0.05 {
        PlayerAnimationType::Walk
    } else if output.grounded && movement.crouching {
        PlayerAnimationType::Crouch
    } else {
        PlayerAnimationType::Idle
    };

    if new_anim != *animation {
        *animation = new_anim;
        *config = AnimationConfig::from(new_anim);
    }
}
