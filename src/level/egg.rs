use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{player::PlayerHurtMarker, shared::GroupLabel};

use super::LevelSystems;

pub struct EggPlugin;

impl Plugin for EggPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<LdtkEgg>("CLANG")
            .add_systems(FixedUpdate, on_egg.in_set(LevelSystems::Simulation));
    }
}

pub struct EggSounds([Handle<AudioSource>; 3]);

impl FromWorld for EggSounds {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self([
            asset_server.load("sfx/egg/egg_1.wav"),
            asset_server.load("sfx/egg/egg_2.wav"),
            asset_server.load("sfx/egg/egg_3.wav"),
        ])
    }
}

pub fn on_egg(
    mut commands: Commands,
    rapier_context: Query<&RapierContext>,
    q_player: Query<Entity, With<PlayerHurtMarker>>,
    q_egg: Query<Entity, (With<EggEgg>, Without<PlayerHurtMarker>)>,
    egg_sounds: Local<EggSounds>,
    mut was_intersecting: Local<bool>,
) {
    let Ok(player_entity) = q_player.get_single() else {
        return;
    };
    let Ok(egg) = q_egg.get_single() else {
        return;
    };
    let Ok(rapier_context) = rapier_context.get_single() else {
        return;
    };
    if let Some(true) = rapier_context.intersection_pair(egg, player_entity) {
        if !*was_intersecting {
            commands.entity(egg).with_child((
                AudioPlayer::new(egg_sounds.0[rand::random_range(0..3)].clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
        *was_intersecting = true;
    } else {
        *was_intersecting = false;
    }
}

#[derive(Component, Default)]
pub struct EggEgg;

#[derive(Bundle, LdtkEntity)]
pub struct LdtkEgg {
    #[sprite("sfx/egg/flower.png")]
    sprite: Sprite,
    #[with(make_egg)]
    egg: Egg,
    #[default]
    egg_egg: EggEgg,
}

#[derive(Bundle)]
pub struct Egg {
    collider: Collider,
    sensor: Sensor,
    groups: CollisionGroups,
}

pub fn make_egg(_: &EntityInstance) -> Egg {
    Egg {
        collider: Collider::cuboid(4.0, 4.0),
        sensor: Sensor,
        groups: CollisionGroups::new(GroupLabel::ALL, GroupLabel::ALL),
    }
}
