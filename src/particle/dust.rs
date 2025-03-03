use std::time::Duration;

use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy_rapier2d::prelude::*;
use rand::{self, seq::IndexedRandom};

use crate::player::{movement::PlayerMovement, PlayerMarker};

use super::{ParticleBundle, ParticleOptions, ParticlePhysicsOptions};

#[derive(Component)]
pub enum DustSurface {
    Wall,
    Crystal,
}

impl DustSurface {
    fn new_particle_options(
        &self,
        starting_velocity: Vec2,
        asset_server: &Res<AssetServer>,
    ) -> ParticleOptions {
        let mut rng = rand::rng();

        match self {
            Self::Wall => {
                let gravity_mult = 200.0;
                let life_time = (2.0 * starting_velocity.y) / gravity_mult;
                ParticleOptions {
                    life_time: Duration::from_secs_f32(life_time),
                    physics: Some(ParticlePhysicsOptions {
                        wind_mult: 0.0,
                        gravity_mult,
                        starting_velocity,
                    }),
                    animation: None,
                    image: asset_server.load(
                        *(["particle/wall_dust_1.png", "particle/wall_dust_2.png"]
                            .choose(&mut rng)
                            .unwrap()),
                    ),
                }
            }
            Self::Crystal => {
                let gravity_mult = 200.0;
                let life_time = (2.0 * starting_velocity.y) / gravity_mult;
                ParticleOptions {
                    life_time: Duration::from_secs_f32(life_time),
                    physics: Some(ParticlePhysicsOptions {
                        wind_mult: 0.0,
                        gravity_mult,
                        starting_velocity,
                    }),
                    animation: None,
                    image: asset_server.load(
                        *([
                            "particle/crystal_dust_1.png",
                            "particle/crystal_dust_2.png",
                            "particle/crystal_dust_3.png",
                        ]
                        .choose(&mut rng)
                        .unwrap()),
                    ),
                }
            }
        }
    }

    fn spawn_interval(&self) -> Duration {
        Duration::from_secs_f32(match self {
            Self::Wall => 0.05,
            Self::Crystal => 0.06,
        })
    }

    fn splash_amount(&self) -> usize {
        match self {
            Self::Wall => 7,
            Self::Crystal => 2,
        }
    }

    fn new_spawn_pos_from_player_pos(&self, player_pos: Vec2) -> Vec2 {
        player_pos
            + Vec2::new(0.0, -10.0)
            + Vec2::new(
                rand::random_range(-4.0..4.0),
                match self {
                    Self::Wall => 0.0,
                    Self::Crystal => rand::random_range(-4.0..4.0),
                },
            )
    }

    fn new_starting_velocity(&self) -> Vec2 {
        match self {
            Self::Wall => Vec2::new(
                rand::random_range(-1.0..1.0) * 20.0,
                rand::random_range(10.0..40.0),
            ),
            Self::Crystal => Vec2::new(
                rand::random_range(-1.0..1.0) * 10.0,
                rand::random_range(8.0..30.0),
            ),
        }
    }
}

#[derive(Resource, Default)]
pub struct DustSpawnStopwatch {
    pub walking: Stopwatch,
    pub landing: Stopwatch,
}

pub fn spawn_player_walking_dust(
    mut commands: Commands,
    player: Query<
        (
            &Transform,
            &KinematicCharacterControllerOutput,
            &PlayerMovement,
        ),
        With<PlayerMarker>,
    >,
    asset_server: Res<AssetServer>,
    dust_surfaces: Query<&DustSurface>,
    mut dust_spawn_stopwatch: ResMut<DustSpawnStopwatch>,
    time: Res<Time>,
) {
    dust_spawn_stopwatch.walking.tick(time.delta());
    dust_spawn_stopwatch.landing.tick(time.delta());
    let Ok((player_t, output, movement)) = player.get_single() else {
        return;
    };

    if !output.grounded {
        return;
    }

    let Some(dust_surface) = output
        .collisions
        .iter()
        .find_map(|collision| dust_surfaces.get(collision.entity).ok())
    else {
        return;
    };

    let (particle_spawn_amount, velocity_mult) = match movement.velocity.length() {
        // if at walking speed, spawn one
        1.25..2.0 => (
            if dust_spawn_stopwatch.walking.elapsed() > dust_surface.spawn_interval() {
                dust_spawn_stopwatch.walking.reset();
                1
            } else {
                0
            },
            1.0,
        ),
        // if at landing after jumping speed, spawn many
        2.0.. => (
            if dust_spawn_stopwatch.landing.elapsed() > Duration::from_secs_f32(0.1) {
                dust_spawn_stopwatch.landing.reset();
                dust_surface.splash_amount()
            } else {
                0
            },
            2.0,
        ),
        _ => (0, 0.0),
    };
    for _ in 0..particle_spawn_amount {
        let pos = dust_surface.new_spawn_pos_from_player_pos(player_t.translation.truncate());

        let starting_velocity = dust_surface.new_starting_velocity() * velocity_mult;

        commands.spawn(ParticleBundle::new(
            dust_surface.new_particle_options(starting_velocity, &asset_server),
            pos,
        ));
    }
}
