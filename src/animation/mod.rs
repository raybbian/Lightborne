use std::time::Duration;

use bevy::prelude::*;

use crate::level::LevelSystems;

pub struct SpriteAnimationPlugin;

impl Plugin for SpriteAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, play_animations.in_set(LevelSystems::Simulation));
    }
}

#[derive(Component)]
pub struct AnimationConfig {
    pub cur_index: usize,
    first_index: usize,
    last_index: usize,
    fps: u8,
    timer: Timer,
    repeat: bool,
}

impl AnimationConfig {
    pub fn new(first: usize, last: usize, fps: u8, repeat: bool) -> Self {
        Self {
            cur_index: first,
            first_index: first,
            last_index: last,
            fps,
            timer: Self::timer_from_fps(fps),
            repeat,
        }
    }

    fn timer_from_fps(fps: u8) -> Timer {
        Timer::new(Duration::from_secs_f32(1.0 / (fps as f32)), TimerMode::Once)
    }
}

fn play_animations(time: Res<Time>, mut query: Query<(&mut AnimationConfig, &mut Sprite)>) {
    for (mut config, mut sprite) in &mut query {
        let Some(atlas) = &mut sprite.texture_atlas else {
            continue;
        };

        // if config doesn't sync with atlas, config was changed
        if config.cur_index != atlas.index {
            atlas.index = config.cur_index;
        }

        config.timer.tick(time.delta());

        if !config.timer.just_finished() {
            continue;
        }

        if atlas.index == config.last_index {
            if config.repeat {
                atlas.index = config.first_index;
                config.timer = AnimationConfig::timer_from_fps(config.fps);
            }
        } else {
            atlas.index += 1;
            config.timer = AnimationConfig::timer_from_fps(config.fps);
        }
        config.cur_index = atlas.index;
    }
}
