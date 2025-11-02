use avian2d::prelude::*;
use bevy::{
    ecs::{
        entity::EntityHashSet,
        system::{lifetimeless::Read, SystemParam},
    },
    prelude::*,
};
use bevy_ecs_ldtk::prelude::*;

use crate::game::{
    defs::merge_tile::{spawn_merged_tiles, MergedTile},
    lyra::Lyra,
    particle::dust::DustSurface,
    Layers, LevelSystems,
};

pub struct OneWayPlatformPlugin;

impl Plugin for OneWayPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_int_cell_for_layer::<OneWayMarker>("Terrain", 15);
        app.add_systems(
            PreUpdate,
            spawn_merged_tiles::<OneWayMarker>.in_set(LevelSystems::Processing),
        );
    }
}
#[derive(Default, Component, LdtkIntCell)]
pub struct OneWayMarker {}

#[derive(Default, Component)]
pub struct OneWayPlatform {
    intersecting: EntityHashSet,
}

impl MergedTile for OneWayMarker {
    type CompareData = ();

    fn bundle(
        commands: &mut EntityCommands,
        center: Vec2,
        extent: Vec2,
        _compare_data: &Self::CompareData,
    ) {
        commands
            .insert(OneWayPlatform::default())
            .insert(Transform::from_xyz(center.x, center.y, 0.))
            .insert(DustSurface::Wood)
            .insert(Collider::compound(vec![(
                Vec2::new(0.0, 3.0),
                Rotation::default(),
                Collider::rectangle(extent.x, 2.0),
            )]))
            .insert(Friction::new(0.))
            .insert(CollisionLayers::new(
                Layers::Platform,
                [
                    Layers::PlayerCollider,
                    Layers::LightRay,
                    Layers::BlueRay,
                    Layers::WhiteRay,
                    Layers::PlayerHurtbox,
                ],
            ))
            .insert(ActiveCollisionHooks::MODIFY_CONTACTS);
    }

    fn compare_data(&self) -> Self::CompareData {}
}

/// A component to control how an actor interacts with a one-way platform.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Component, Reflect)]
pub enum PassThroughOneWayPlatform {
    #[default]
    /// Passes through a `OneWayPlatform` if the contact normal is in line with the platform's local-space up vector.
    ByNormal,
    /// Always passes through a `OneWayPlatform`, temporarily set this to allow an actor to jump down through a platform.
    Always,
    /// Never passes through a `OneWayPlatform`.
    Never,
}

#[allow(clippy::type_complexity)]
// Define a custom `SystemParam` for our collision hooks.
// It can have read-only access to queries, resources, and other system parameters.
#[derive(SystemParam)]
pub struct OneWayPlatformHooks<'w, 's> {
    one_way_platforms_query: Query<'w, 's, (Read<OneWayPlatform>, Read<GlobalTransform>)>,
    lyra_query: Query<
        'w,
        's,
        (Read<GlobalTransform>, Read<PassThroughOneWayPlatform>),
        (With<Collider>, With<Lyra>, Without<OneWayPlatform>),
    >,
}

// Implement the `CollisionHooks` trait for our custom system parameter.
impl CollisionHooks for OneWayPlatformHooks<'_, '_> {
    fn modify_contacts(&self, contacts: &mut ContactPair, commands: &mut Commands) -> bool {
        // This is the contact modification hook, called after collision detection,
        // but before constraints are created for the solver. Mutable access to the ECS
        // is not allowed, but we can queue commands to perform deferred changes.

        // First, figure out which entity is the one-way platform, and which is the other.
        // Choose the appropriate normal for pass-through depending on which is which.
        let (platform_entity, one_way_platform, platform_transform, other_entity) =
            if let Ok((one_way_platform, platform_transform)) =
                self.one_way_platforms_query.get(contacts.collider1)
            {
                (
                    contacts.collider1,
                    one_way_platform,
                    platform_transform,
                    contacts.collider2,
                )
            } else if let Ok((one_way_platform, platform_transform)) =
                self.one_way_platforms_query.get(contacts.collider2)
            {
                (
                    contacts.collider2,
                    one_way_platform,
                    platform_transform,
                    contacts.collider1,
                )
            } else {
                // Neither is a one-way-platform, so accept the collision:
                // we're done here.
                return true;
            };

        if one_way_platform.intersecting.contains(&other_entity) {
            let any_penetrating = contacts.manifolds.iter().any(|manifold| {
                manifold
                    .points
                    .iter()
                    .any(|contact| contact.penetration > 0.0)
            });

            if any_penetrating {
                // If we were already allowing a collision for a particular entity,
                // and if it is penetrating us still, continue to allow it to do so.
                return false;
            } else {
                // If it's no longer penetrating us, forget it.
                commands.queue(OneWayPlatformCommand::Remove {
                    platform_entity,
                    entity: other_entity,
                });
            }
        }

        match self.lyra_query.get(other_entity) {
            // Pass-through is set to never, so accept the collision.
            Ok((_, PassThroughOneWayPlatform::Never)) => true,
            // Pass-through is set to always, so always ignore this collision
            // and register it as an entity that's currently penetrating.
            Ok((_, PassThroughOneWayPlatform::Always)) => {
                commands.queue(OneWayPlatformCommand::Add {
                    platform_entity,
                    entity: other_entity,
                });
                false
            }
            // Default behaviour is "by normal".
            Ok((other_transform, PassThroughOneWayPlatform::ByNormal)) => {
                const PLAYER_HALF_HEIGHT: f32 = 10.0;
                if other_transform.compute_transform().translation.y
                    - platform_transform.compute_transform().translation.y
                    > PLAYER_HALF_HEIGHT
                {
                    true
                } else {
                    // Otherwise, ignore the collision and register
                    // the other entity as one that's currently penetrating.
                    commands.queue(OneWayPlatformCommand::Add {
                        platform_entity,
                        entity: other_entity,
                    });
                    false
                }
            }
            _ => true,
        }
    }
}

/// A command to add/remove entities to/from the set of entities
/// that are currently in contact with a one-way platform.
enum OneWayPlatformCommand {
    Add {
        platform_entity: Entity,
        entity: Entity,
    },
    Remove {
        platform_entity: Entity,
        entity: Entity,
    },
}

impl Command for OneWayPlatformCommand {
    fn apply(self, world: &mut World) {
        match self {
            OneWayPlatformCommand::Add {
                platform_entity,
                entity,
            } => {
                if let Some(mut platform) = world.get_mut::<OneWayPlatform>(platform_entity) {
                    platform.intersecting.insert(entity);
                }
            }

            OneWayPlatformCommand::Remove {
                platform_entity,
                entity,
            } => {
                if let Some(mut platform) = world.get_mut::<OneWayPlatform>(platform_entity) {
                    platform.intersecting.remove(&entity);
                }
            }
        }
    }
}
