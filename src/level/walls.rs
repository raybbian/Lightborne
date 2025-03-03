use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{lighting::Occluder2d, particle::dust::DustSurface, shared::GroupLabel};

use super::merge_tile::MergedTile;

/// Marker [`Component`] representing a wall.
#[derive(Default, Component)]
pub struct Wall;

/// Wall [`Bundle`] spawned int by Ldtk.
#[derive(Default, Bundle, LdtkIntCell)]
pub struct WallBundle {
    wall: Wall,
}

impl MergedTile for Wall {
    type CompareData = ();

    fn bundle(
        commands: &mut EntityCommands,
        center: Vec2,
        half_extent: Vec2,
        _compare_data: &Self::CompareData,
    ) {
        commands.insert((
            Collider::cuboid(half_extent.x, half_extent.y),
            Occluder2d::new(half_extent.x, half_extent.y),
            CollisionGroups::new(
                GroupLabel::TERRAIN,
                GroupLabel::PLAYER_COLLIDER
                    | GroupLabel::LIGHT_RAY
                    | GroupLabel::WHITE_RAY
                    | GroupLabel::STRAND
                    | GroupLabel::BLUE_RAY,
            ),
            RigidBody::Fixed,
            Transform::from_xyz(center.x, center.y, 0.),
            DustSurface::Wall,
        ));
    }

    fn compare_data(&self) -> Self::CompareData {
        // all walls are mergable
        ()
    }
}
