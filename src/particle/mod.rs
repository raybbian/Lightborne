use std::{ops::Range, time::Duration};

use bevy::prelude::*;
use dust::{add_crystal_dust, spawn_player_walking_dust, DustSpawnStopwatch};
use noise::{NoiseFn, Simplex};
use rand::prelude::IndexedRandom;

pub mod dust;
use crate::{
    level::{crystal::Crystal, LevelSystems},
    lighting::{LineLight2d, Occluder2dGroups},
};
pub struct ParticlePlugin;
impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Wind::new())
            .insert_resource(DustSpawnStopwatch::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    update_particles,
                    update_particle_emitters,
                    add_crystal_shine,
                    spawn_player_walking_dust,
                    adjust_crystal_shine_lights,
                    add_crystal_dust,
                    delete_particles,
                )
                    .in_set(LevelSystems::Simulation),
            );
    }
}

#[derive(Resource)]
pub struct Wind {
    noise_1: Simplex,
    noise_2: Simplex,
}

impl Wind {
    fn new() -> Self {
        Self {
            noise_1: Simplex::new(0),
            noise_2: Simplex::new(1),
        }
    }

    fn force_at(&self, time: f32, pos: Vec2) -> Vec2 {
        let point = [time * 0.5, pos.x * 0.005, pos.y * 0.005];
        let point = [point[0] as f64, point[1] as f64, point[2] as f64];
        Vec2::new(
            self.noise_1.get(point) as f32,
            self.noise_2.get(point) as f32,
        )
    }
}

#[derive(Default, Clone, Debug)]
pub struct ParticleAnimationOptions {
    pub frame_time: Duration,
    pub frame_count: usize,
    pub frame_size: Vec2,
    pub repeat: bool,
}

#[derive(Default, Clone, Debug)]
pub struct ParticlePhysicsOptions {
    pub wind_mult: f32,
    pub gravity_mult: f32,
    pub starting_velocity: Vec2,
}

#[derive(Default, Clone, Debug)]
pub struct ParticleOptions {
    pub life_time: Duration,
    pub physics: Option<ParticlePhysicsOptions>,
    pub animation: Option<ParticleAnimationOptions>,
    pub sprite: Sprite,
}

#[derive(Component, Default, Clone)]
#[require(Transform)]
pub struct Particle {
    life_timer: Timer,
    velocity: Vec2,
    pos: Vec2,

    frame_index: usize,
    frame_timer: Timer,

    options: ParticleOptions,
}

impl Particle {
    fn new(options: ParticleOptions, start_pos: Vec2) -> Self {
        Self {
            life_timer: Timer::new(options.life_time, TimerMode::Once),
            velocity: options
                .physics
                .clone()
                .map(|p| p.starting_velocity)
                .unwrap_or_default(),

            frame_index: 0,
            frame_timer: Timer::new(
                options
                    .animation
                    .clone()
                    .map(|a| a.frame_time)
                    .unwrap_or(Duration::ZERO),
                TimerMode::Repeating,
            ),
            pos: start_pos,
            options,
        }
    }
}

#[derive(Bundle, Clone)]
pub struct ParticleBundle {
    particle: Particle,
    transform: Transform,
    sprite: Sprite,
}

impl ParticleBundle {
    fn new(options: ParticleOptions, start_pos: Vec2) -> Self {
        let rect = options
            .animation
            .as_ref()
            .map(|a| Rect::new(0.0, 0.0, a.frame_size.x, a.frame_size.y));
        let sprite = options.sprite.clone();
        Self {
            particle: Particle::new(options, start_pos),
            transform: Transform::from_translation(start_pos.extend(2.0)),
            sprite: Sprite { rect, ..sprite },
        }
    }
}

#[derive(Clone, Debug)]
pub enum ParticleEmitterArea {
    // Point,
    Cuboid { half_x: f32, half_y: f32 },
    // Circle { radius: f32 },
    // Capsule { radius: f32 },
}

#[derive(Clone, Debug)]
pub struct ParticleEmitterOptions {
    pub area: ParticleEmitterArea,
    pub particles: Vec<ParticleOptions>,
    pub delay_range: Range<Duration>,
}

#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility)]
pub struct ParticleEmitter {
    pub options: ParticleEmitterOptions,
    pub timer: Timer,
}

impl ParticleEmitter {
    pub fn new(options: ParticleEmitterOptions) -> Self {
        Self {
            timer: Timer::new(
                rand::random_range(options.delay_range.clone()),
                TimerMode::Once,
            ),
            options,
        }
    }
}

fn setup() {}

fn update_particle_emitters(
    mut commands: Commands,
    time: Res<Time>,
    mut emitters: Query<(Entity, &mut ParticleEmitter)>,
) {
    for (emitter_entity, mut emitter) in emitters.iter_mut() {
        emitter.timer.tick(time.delta());
        if !emitter.timer.finished() {
            continue;
        }
        emitter.timer = Timer::new(
            rand::random_range(emitter.options.delay_range.clone()),
            TimerMode::Once,
        );
        let start_pos = match emitter.options.area {
            // ParticleEmitterArea::Point => Vec2::ZERO,
            ParticleEmitterArea::Cuboid { half_x, half_y } => Vec2::new(
                half_x * rand::random_range(-1.0..1.0),
                half_y * rand::random_range(-1.0..1.0),
            ),
            // ParticleEmitterArea::Circle { radius } => {
            //     let angle = rand::random_range(0.0..(2.0 * PI));
            //     let dist = rand::random_range(0.0..radius);
            //     Vec2::new(angle.cos() * dist, angle.sin() * dist)
            // }
            // ParticleEmitterArea::Capsule { radius } => {
            //     let unit_vec = transform
            //         .rotation()
            //         .mul_vec3(Vec3::new(1.0, 0.0, 0.0))
            //         .truncate();
            //     let point_1_offset = unit_vec * transform.scale().x / 2.;
            //     let point_2_offset = -unit_vec * transform.scale().x / 2.;

            //     let weight = rand::random_range(0.0..1.0);
            //     let point_on_line = point_1_offset * weight + point_2_offset * (1.0 - weight);

            //     let angle = rand::random_range(0.0..(2.0 * PI));
            //     let dist = rand::random_range(0.0..radius);
            //     point_on_line + Vec2::new(angle.cos() * dist, angle.sin() * dist)
            // }
        };
        commands
            .entity(emitter_entity)
            .with_child(ParticleBundle::new(
                emitter
                    .options
                    .particles
                    .choose(&mut rand::rng())
                    .expect("ParticleBundle particles were empty")
                    .clone(),
                start_pos,
            ));
    }
}

fn delete_particles(mut commands: Commands, particles: Query<(Entity, &Particle)>) {
    for (entity, particle) in particles.iter() {
        if particle.life_timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_particles(
    mut particles: Query<(&mut Transform, &mut Particle, &mut Sprite)>,
    time: Res<Time>,
    wind: Res<Wind>,
) {
    for (mut transform, mut particle, mut sprite) in particles.iter_mut() {
        particle.life_timer.tick(time.delta());

        if let Some(physics) = particle.options.physics.clone() {
            let pos = transform.translation.truncate();
            let mut velocity = particle.velocity;
            let mut accel = Vec2::ZERO;

            accel += Vec2::new(0.0, -1.0) * time.delta_secs() * physics.gravity_mult;

            let wind_vec = wind.force_at(time.elapsed_secs(), pos);
            accel += wind_vec * time.delta_secs() * 300.0 * physics.wind_mult;

            velocity += accel;
            particle.velocity = velocity;
            particle.pos += velocity * time.delta_secs();
            transform.translation = particle.pos.round().extend(transform.translation.z);
        }

        if let Some(animation) = particle.options.animation.clone() {
            particle.frame_timer.tick(time.delta());
            if particle.frame_timer.just_finished() {
                particle.frame_index += 1;
            }
            if animation.repeat || particle.frame_index < animation.frame_count {
                let frame = particle.frame_index % animation.frame_count;
                sprite.rect = Some(Rect::new(
                    animation.frame_size.x * frame as f32,
                    0.0,
                    animation.frame_size.x * (frame + 1) as f32,
                    animation.frame_size.y,
                ))
            }
        }
    }
}

#[derive(Default, Component, Debug)]
pub struct CrystalEmitterMarker;

fn add_crystal_shine(
    mut commands: Commands,
    crystal: Query<(Entity, &Crystal), Changed<Crystal>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, crystal) in crystal.iter() {
        if crystal.active {
            commands.entity(entity).insert_if_new((
                ParticleEmitter::new(ParticleEmitterOptions {
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
                                physics: None,
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
                            }
                        },
                        {
                            const FRAME_TIME: f32 = 0.05;
                            const FRAME_COUNT: usize = 5;
                            ParticleOptions {
                                life_time: Duration::from_secs_f32(FRAME_TIME * FRAME_COUNT as f32),
                                physics: None,
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
                            }
                        },
                    ],
                }),
                CrystalEmitterMarker,
            ));
        } else {
            commands
                .entity(entity)
                .remove::<(ParticleEmitter, CrystalEmitterMarker)>();
        }
    }
}

fn adjust_crystal_shine_lights(
    mut commands: Commands,
    q_crystal_emitter: Query<&Children, With<CrystalEmitterMarker>>,
    mut q_particle: Query<(&Particle, Option<&mut LineLight2d>)>,
) {
    for crystal_emitter_children in q_crystal_emitter.iter() {
        for crystal_particle in crystal_emitter_children.iter() {
            let Ok((particle, light)) = q_particle.get_mut(*crystal_particle) else {
                continue;
            };
            match light {
                None => {
                    commands.entity(*crystal_particle).insert((
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
}

// fn add_segment_sparks(
//     mut commands: Commands,
//     light_segment: Query<(Entity, &LightSegment), Changed<LightSegment>>,
//     asset_server: Res<AssetServer>,
// ) {
//     // for (entity, light_segment) in light_segment.iter() {
//     //     commands
//     //         .entity(entity)
//     //         .insert(ParticleEmitter::new(ParticleEmitterOptions {
//     //             area: ParticleEmitterArea::Capsule { radius: 2.0 },
//     //             delay_range: Duration::from_secs_f32(0.0)..Duration::from_secs_f32(1.0),
//     //             particle: create_shine_particle_options(&asset_server),
//     //         }));
//     // }
// }
