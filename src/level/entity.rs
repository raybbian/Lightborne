use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::shared::GroupLabel;
/// Component for things that hurt
#[derive(Default, Component)]
pub struct HurtMarker;

/// Component for spikes
#[derive(Default, Component)]
pub struct Spike;

/// Bundle for spikes
#[derive(Default, Bundle, LdtkIntCell)]
pub struct SpikeBundle {
    #[from_int_grid_cell]
    fixed_entity_bundle: FixedEntityBundle,
    hurt_marker: HurtMarker,
    spike: Spike,
}

/// [`Bundle`] used to group together components commonly used together when initializing physics
/// for fixed [`LdtkEntity`]s.
#[derive(Default, Bundle)]
pub struct FixedEntityBundle {
    pub collider: Collider,
    pub rigid_body: RigidBody,
    pub collision_groups: CollisionGroups,
}

impl From<&EntityInstance> for FixedEntityBundle {
    fn from(entity_instance: &EntityInstance) -> Self {
        match entity_instance.identifier.as_ref() {
            "Sensor" => FixedEntityBundle {
                collider: Collider::cuboid(4., 4.),
                rigid_body: RigidBody::Fixed,
                collision_groups: CollisionGroups::new(
                    GroupLabel::LIGHT_SENSOR,
                    GroupLabel::LIGHT_RAY | GroupLabel::WHITE_RAY | GroupLabel::BLUE_RAY,
                ),
            },
            "CrystalShard" => FixedEntityBundle {
                collider: Collider::cuboid(6., 6.),
                rigid_body: RigidBody::Fixed,
                collision_groups: CollisionGroups::new(
                    GroupLabel::CRYSTAL_SHARD,
                    GroupLabel::PLAYER_SENSOR,
                ),
            },
            _ => unreachable!(),
        }
    }
}

impl From<IntGridCell> for FixedEntityBundle {
    fn from(cell_instance: IntGridCell) -> Self {
        match cell_instance.value {
            2 => FixedEntityBundle {
                collider: Collider::triangle(
                    Vec2::new(-4., -4.),
                    Vec2::new(4., -4.),
                    Vec2::new(0., 4.),
                ),
                rigid_body: RigidBody::Fixed,
                collision_groups: CollisionGroups::new(
                    GroupLabel::TERRAIN,
                    GroupLabel::ALL & !GroupLabel::PLAYER_COLLIDER,
                ),
            },
            15 => FixedEntityBundle {
                collider: Collider::cuboid(4., 1.),
                rigid_body: RigidBody::Fixed,
                collision_groups: CollisionGroups::new(GroupLabel::TERRAIN, GroupLabel::ALL),
            },
            _ => unreachable!(),
        }
    }
}
