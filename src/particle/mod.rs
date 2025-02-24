use bevy::prelude::*;
use noise::{NoiseFn, Simplex};
use rand::Rng;

use crate::camera::MainCamera;
pub struct ParticlePlugin;
impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(DustSpawnTimer::new())
            .insert_resource(Wind::new())
            .add_systems(Startup, setup)
            .add_systems(Update, spawn_particles)
            .add_systems(Update, update_particles)
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

#[derive(Component)]
#[require(Transform)]
pub struct Particle {
    timer: Timer,
    velocity: Vec2,

    friction: f32,
}

#[derive(Resource)]
pub struct DustSpawnTimer(Timer);

impl DustSpawnTimer {
    fn new() -> Self {
        Self(Timer::from_seconds(0.01, TimerMode::Repeating))
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
        commands.spawn((
            Particle {
                timer: Timer::from_seconds(8.0, TimerMode::Once),
                velocity: Vec2::default(),

                friction: 1.0,
            },
            Sprite {
                image: asset_server.load("particle/dust.png"),
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
        ));
    }
}

fn delete_particles(mut commands: Commands, particles: Query<(Entity, &Particle)>) {
    for (entity, particle) in particles.iter() {
        if particle.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn update_particles(
    mut particles: Query<(&mut Transform, &mut Particle)>,
    time: Res<Time>,
    wind: Res<Wind>,
) {
    for (mut transform, mut particle) in particles.iter_mut() {
        particle.timer.tick(time.delta());

        let mut accel = Vec2::ZERO;
        accel += Vec2::new(0.0, -0.5) * time.delta_secs();

        let pos = transform.translation.truncate();
        let wind_vec = wind.force_at(time.elapsed_secs(), pos);
        accel += wind_vec * time.delta_secs() * 8.0;

        particle.velocity += accel;

        let friction = particle.friction;
        particle.velocity *= friction;

        transform.translation += particle.velocity.extend(0.0);
    }
}
