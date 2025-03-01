use std::{f32::consts::PI, ops::Range, time::Duration};

use bevy::{asset, prelude::*};
use noise::{NoiseFn, Simplex};
use rand::Rng;

use crate::{camera::MainCamera, level::crystal::Crystal};
pub struct ParticlePlugin;
impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(DustSpawnTimer::new())
            .insert_resource(Wind::new())
            .add_systems(Startup, setup)
            .add_systems(Update, spawn_particles)
            .add_systems(Update, update_particles)
            .add_systems(Update, update_particle_emitters)
            .add_systems(Update, add_crystal_sparkles)
            .add_systems(Update, delete_particles)
        // end
        ;
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

#[derive(Default, Clone)]
pub struct ParticleAnimationOptions {
    pub frame_time: Duration,
    pub frame_count: usize,
    pub frame_size: Vec2,
    pub repeat: bool,
}

#[derive(Default, Clone)]
pub struct ParticlePhysicsOptions {
    pub wind_mult: f32,
    pub gravity_mult: f32,
}

#[derive(Default, Clone)]
pub struct ParticleOptions {
    pub life_time: Duration,
    pub physics: Option<ParticlePhysicsOptions>,
    pub animation: Option<ParticleAnimationOptions>,
    pub image: Handle<Image>,
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
            velocity: Vec2::ZERO,

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
        let image = options.image.clone();
        Self {
            particle: Particle::new(options, start_pos),
            transform: Transform::from_translation(start_pos.extend(2.0)),
            sprite: Sprite {
                image,
                rect,
                ..default()
            },
        }
    }
}

#[derive(Clone)]

pub enum ParticleEmitterArea {
    Point,
    Cuboid { half_x: f32, half_y: f32 },
    Circle { radius: f32 },
}

#[derive(Clone)]
pub struct ParticleEmitterOptions {
    pub area: ParticleEmitterArea,
    pub particle: ParticleOptions,
    pub delay_range: Range<Duration>,
}

#[derive(Component, Clone)]
#[require(Transform)]
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

#[derive(Resource)]
pub struct DustSpawnTimer(Timer);

impl DustSpawnTimer {
    fn new() -> Self {
        Self(Timer::from_seconds(0.1, TimerMode::Repeating))
    }
}

fn setup() {}

fn spawn_particles(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    camera_t: Query<&Transform, With<MainCamera>>,
    mut dust_timer: ResMut<DustSpawnTimer>,
) {
    let Ok(camera_t) = camera_t.get_single() else {
        return;
    };
    dust_timer.0.tick(time.delta());
    return;
    if dust_timer.0.finished() {
        let mut rng = rand::rng();

        let pos = camera_t.translation.truncate()
            + Vec2::new(320.0 * rng.random::<f32>(), 180.0 * rng.random::<f32>())
            - Vec2::new(320.0 * 0.5, 180.0 * 0.5);
        commands.spawn(ParticleBundle::new(
            ParticleOptions {
                life_time: Duration::from_secs_f32(1.0),
                physics: None,
                animation: Some(ParticleAnimationOptions {
                    frame_count: 9,
                    frame_size: Vec2::new(5.0, 5.0),
                    frame_time: Duration::from_secs_f32(0.05),
                    repeat: false,
                }),
                image: asset_server.load("particle/shine.png"),
            },
            pos,
        ));
    }
}

fn update_particle_emitters(
    mut commands: Commands,
    time: Res<Time>,
    mut emitters: Query<(&mut ParticleEmitter, &GlobalTransform)>,
) {
    for (mut emitter, transform) in emitters.iter_mut() {
        emitter.timer.tick(time.delta());
        if !emitter.timer.finished() {
            continue;
        }
        emitter.timer = Timer::new(
            rand::random_range(emitter.options.delay_range.clone()),
            TimerMode::Once,
        );
        let emitter_pos = transform.translation().truncate();
        let start_pos = emitter_pos
            + match emitter.options.area {
                ParticleEmitterArea::Point => Vec2::ZERO,
                ParticleEmitterArea::Cuboid { half_x, half_y } => Vec2::new(
                    half_x * rand::random_range(-1.0..1.0),
                    half_y * rand::random_range(-1.0..1.0),
                ),
                ParticleEmitterArea::Circle { radius } => {
                    let angle = rand::random_range(0.0..(2.0 * PI));
                    let dist = rand::random_range(0.0..radius);
                    Vec2::new(angle.cos() * dist, angle.sin() * dist)
                }
            };
        commands.spawn(ParticleBundle::new(
            emitter.options.particle.clone(),
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

            accel += Vec2::new(0.0, -0.5) * time.delta_secs() * physics.gravity_mult;

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

fn add_crystal_sparkles(
    mut commands: Commands,
    crystal: Query<(Entity, &Crystal), Changed<Crystal>>,
    asset_server: Res<AssetServer>,
) {
    const FRAME_TIME: f32 = 0.05;
    const FRAME_COUNT: usize = 9;
    for (entity, crystal) in crystal.iter() {
        if crystal.active {
            commands
                .entity(entity)
                .insert(ParticleEmitter::new(ParticleEmitterOptions {
                    area: ParticleEmitterArea::Cuboid {
                        half_x: 4.0,
                        half_y: 4.0,
                    },
                    delay_range: Duration::from_secs_f32(0.0)..Duration::from_secs_f32(30.0),
                    particle: ParticleOptions {
                        life_time: Duration::from_secs_f32(FRAME_TIME * FRAME_COUNT as f32),
                        physics: None,
                        animation: Some(ParticleAnimationOptions {
                            frame_count: FRAME_COUNT,
                            frame_size: Vec2::new(5.0, 5.0),
                            frame_time: Duration::from_secs_f32(FRAME_TIME),
                            repeat: false,
                        }),
                        image: asset_server.load("particle/shine.png"),
                    },
                }));
        } else {
            commands.entity(entity).remove::<ParticleEmitter>();
        }
    }
}
