use std::time::Duration;

use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy_rapier2d::prelude::*;
use rand::{self, seq::IndexedRandom};

use crate::{
    level::crystal::{CrystalColor, CrystalGroup},
    player::{movement::PlayerMovement, PlayerMarker},
};

use super::{ParticleBundle, ParticleOptions, ParticlePhysicsOptions};

#[derive(Component, Debug)]
pub enum DustSurface {
    Wall,
    Wood,
    Crystal(CrystalColor),
}

impl DustSurface {
    fn new_particle_options(
        &self,
        starting_velocity: Vec2,
        asset_server: &Res<AssetServer>,
    ) -> ParticleOptions {
        let mut rng = rand::rng();
        let gravity_mult = match self {
            Self::Wall => 200.0,
            Self::Wood => 120.0,
            Self::Crystal(_) => 200.0,
        };

        let images: &[&str] = match self {
            Self::Wall => &["particle/wall_dust_1.png", "particle/wall_dust_2.png"],
            Self::Wood => &[
                "particle/wood_dust_1.png",
                "particle/wood_dust_2.png",
                "particle/wood_dust_3.png",
            ],
            Self::Crystal(_) => &[
                "particle/crystal_dust_1.png",
                "particle/crystal_dust_2.png",
                "particle/crystal_dust_3.png",
                "particle/crystal_dust_4.png",
            ],
        };
        let color = if let Self::Crystal(color) = self {
            color.button_color()
        } else {
            Color::default()
        };
        let life_time = (2.0 * starting_velocity.y) / gravity_mult;
        ParticleOptions {
            life_time: Duration::from_secs_f32(life_time),
            physics: Some(ParticlePhysicsOptions {
                wind_mult: 0.0,
                gravity_mult,
                starting_velocity,
            }),
            animation: None,
            sprite: Sprite {
                image: asset_server.load(*(images.choose(&mut rng).unwrap())),
                color,
                ..default()
            },
            ..default()
        }
    }

    fn spawn_interval(&self) -> Duration {
        Duration::from_secs_f32(match self {
            Self::Wall => 0.05,
            Self::Wood => 0.05,
            Self::Crystal(_) => 0.06,
        })
    }

    fn splash_amount(&self) -> usize {
        match self {
            Self::Wall => 7,
            Self::Wood => 6,
            Self::Crystal(_) => 4,
        }
    }

    fn new_spawn_pos_from_player_pos(&self, player_pos: Vec2) -> Vec2 {
        player_pos + Vec2::new(0.0, -10.0) + Vec2::new(rand::random_range(-4.0..4.0), 0.0)
    }

    fn new_starting_velocity(&self) -> Vec2 {
        match self {
            Self::Wall => Vec2::new(
                rand::random_range(-1.0..1.0) * 20.0,
                rand::random_range(10.0..40.0),
            ),
            Self::Wood => Vec2::new(
                rand::random_range(-1.0..1.0) * 20.0,
                rand::random_range(10.0..30.0),
            ),
            Self::Crystal(_) => Vec2::new(
                rand::random_range(-1.0..1.0) * 20.0,
                rand::random_range(10.0..40.0),
            ),
        }
    }
}

#[derive(Resource, Default)]
pub struct DustSpawnStopwatch {
    pub walking: Stopwatch,
    pub landing: Stopwatch,
}

pub fn add_crystal_dust(
    mut commands: Commands,
    crystals: Query<(Entity, &CrystalGroup), Added<CrystalGroup>>,
) {
    for (entity, crystal) in crystals.iter() {
        commands
            .entity(entity)
            .insert(DustSurface::Crystal(crystal.representative.ident.color));
    }
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

    let Some(dust_surface) = output.collisions.iter().find_map(|collision| {
        let is_below = collision
            .hit
            .details
            .is_some_and(|detail| detail.normal2.y < 0.0);
        if !is_below {
            return None;
        }
        dust_surfaces.get(collision.entity).ok()
    }) else {
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
