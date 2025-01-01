use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::player::PlayerBundle;

use button::ButtonBundle;
use walls::{spawn_wall_collision, WallBundle};
use door::DoorBundle;

mod button;
mod walls;
mod door;

pub struct LevelManagementPlugin;

impl Plugin for LevelManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            // .insert_resource(LdtkSettings {
            //     level_background: LevelBackground::Nonexistent,
            //     ..default()
            // })
            .add_systems(Startup, setup_level)
            .insert_resource(LevelSelection::index(0))
            .register_ldtk_entity::<PlayerBundle>("Lyra")
            .register_ldtk_entity::<DoorBundle>("Door")
            .register_ldtk_int_cell::<WallBundle>(1)
            .register_ldtk_int_cell::<ButtonBundle>(5)
            .add_systems(Update, spawn_wall_collision);
    }
}

fn setup_level(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("lightborne.ldtk").into(),
        ..Default::default()
    });
}

#[derive(Default, Bundle)]
pub struct CollidableEntity {
    collider: Collider,
    rigid_body: RigidBody,
}

impl From<&EntityInstance> for CollidableEntity {
    fn from(entity_instance: &EntityInstance) -> Self {
        match entity_instance.identifier.as_ref() {
            "Door" => CollidableEntity {
                collider: Collider::cuboid(4., 4.),
                rigid_body: RigidBody::Fixed,
            },
            _ => unreachable!(),
        }
    }
}
