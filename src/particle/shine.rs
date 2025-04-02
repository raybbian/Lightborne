use std::time::Duration;

use bevy::prelude::*;

use crate::{
    level::crystal::Crystal,
    lighting::{LineLight2d, Occluder2dGroups},
    particle::{ParticleAnimationOptions, ParticleOptions},
};

use super::{
    emitter::{ParticleEmitter, ParticleEmitterArea, ParticleEmitterOptions},
    Particle,
};

pub fn add_crystal_shine(
    mut commands: Commands,
    crystal: Query<(Entity, &Crystal), Changed<Crystal>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, crystal) in crystal.iter() {
        if crystal.active {
            commands
                .entity(entity)
                .insert_if_new((ParticleEmitter::new(ParticleEmitterOptions {
                    area: ParticleEmitterArea::Cuboid {
                        half_x: 4.0,
                        half_y: 4.0,
                    },
                    delay_range: Duration::from_secs_f32(0.0)..Duration::from_secs_f32(30.0),
                    particles: vec![
                        {
                            const FRAME_TIME: f32 = 0.05;
                            const FRAME_COUNT: usize = 9;
                            ParticleOptions {
                                life_time: Duration::from_secs_f32(FRAME_TIME * FRAME_COUNT as f32),
                                animation: Some(ParticleAnimationOptions {
                                    frame_count: FRAME_COUNT,
                                    frame_size: Vec2::new(5.0, 5.0),
                                    frame_time: Duration::from_secs_f32(FRAME_TIME),
                                    repeat: false,
                                }),
                                sprite: Sprite {
                                    image: asset_server.load("particle/shine_1.png"),
                                    ..default()
                                },
                                light: true,
                                ..default()
                            }
                        },
                        {
                            const FRAME_TIME: f32 = 0.05;
                            const FRAME_COUNT: usize = 5;
                            ParticleOptions {
                                life_time: Duration::from_secs_f32(FRAME_TIME * FRAME_COUNT as f32),
                                animation: Some(ParticleAnimationOptions {
                                    frame_count: FRAME_COUNT,
                                    frame_size: Vec2::new(3.0, 3.0),
                                    frame_time: Duration::from_secs_f32(FRAME_TIME),
                                    repeat: false,
                                }),
                                sprite: Sprite {
                                    image: asset_server.load("particle/shine_2.png"),
                                    ..default()
                                },
                                light: true,
                                ..default()
                            }
                        },
                    ],
                    ..default()
                }),));
        } else {
            commands.entity(entity).remove::<ParticleEmitter>();
        }
    }
}

pub fn adjust_crystal_shine_lights(
    mut commands: Commands,
    mut q_particle: Query<(Entity, &Particle, Option<&mut LineLight2d>)>,
) {
    for (entity, particle, light) in q_particle.iter_mut() {
        if !particle.options.light {
            continue;
        }

        match light {
            None => {
                commands.entity(entity).insert((
                    LineLight2d::point(Vec4::new(1.0, 1.0, 1.0, 0.0), 15.0, 0.005),
                    Occluder2dGroups::NONE,
                ));
            }
            Some(mut light) => {
                let progress = particle.life_timer.elapsed_secs()
                    / particle.life_timer.duration().as_secs_f32();
                // light follows 1-x^2 over [-1, 1]
                let progress_one_one = progress * 2. - 1.;
                light.color.w = 1. - progress_one_one.powi(2);
            }
        }
    }
}
