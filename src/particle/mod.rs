use std::time::Duration;

use bevy::prelude::*;
use dust::{add_crystal_dust, spawn_player_walking_dust, DustSpawnStopwatch};
use emitter::{
    update_particle_emitters, ParticleEmitter, ParticleEmitterArea, ParticleEmitterOptions,
};
use noise::{NoiseFn, Simplex};
use shine::{add_crystal_shine, adjust_crystal_shine_lights};
use spark::{
    add_segment_sparks, create_spark_explosions, SegmentTransformMap, SparkExplosionEvent,
};

pub mod dust;
pub mod emitter;
pub mod shine;
pub mod spark;
use crate::level::LevelSystems;
pub struct ParticlePlugin;
impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Wind::new())
            .insert_resource(DustSpawnStopwatch::default())
            .insert_resource(SegmentTransformMap::default())
            .add_event::<SparkExplosionEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    (
                        delete_particles,
                        update_particles,
                        adjust_crystal_shine_lights,
                    )
                        .chain(),
                    update_particle_emitters,
                    add_crystal_shine,
                    spawn_player_walking_dust,
                    add_crystal_dust,
                    add_segment_sparks,
                    create_spark_explosions,
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

fn setup() {}

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
    pub fade_away: bool,
    pub light: bool,
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

        if particle.options.fade_away {
            sprite.color = sprite.color.with_alpha(
                (particle.life_timer.remaining_secs() / particle.options.life_time.as_secs_f32())
                    .powi(2),
            );
        }
    }
}
