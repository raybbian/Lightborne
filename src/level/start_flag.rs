use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

/// [`Component`] to mark start flags in the level. Used to query for when start flags are loaded
/// by Ldtk.
#[derive(Default, Component)]
pub struct StartMarker;

/// [`Component`] to hold information about start flags in the level. Initialized after Ldtk spawns
/// the entity, as the [`level_iid`](StartFlag::level_iid) needs to be retrieved from the level entity.
#[derive(Default, Component)]
pub struct StartFlag {
    /// The `level_iid` of the `StartFlag`'s level.
    pub level_iid: LevelIid,
}

/// [`Bundle`] spawned in by Ldtk corresponding to start flags.
#[derive(Default, Bundle, LdtkEntity)]
pub struct StartFlagBundle {
    flag: StartFlag,
    marker: StartMarker,
    #[from_entity_instance]
    instance: EntityInstance,
}

/// Initializes the start maker with the `level_iid`, which must be gotten with queries.
///
/// We can consider inserting the [`StartFlag`] directly into the entity that represents the level in
/// ldtk, allowing us to query for the [`StartFlag`] through the
/// [`CurrentLevel`](super::CurrentLevel) [`Resource`].
pub fn init_start_marker(
    mut commands: Commands,
    q_start_flag: Query<(Entity, &Parent), Added<StartMarker>>,
    q_parent: Query<&Parent, Without<StartMarker>>,
    q_level: Query<&LevelIid>,
) {
    for (entity, parent) in q_start_flag.iter() {
        let Ok(level_entity) = q_parent.get(parent.get()) else {
            continue;
        };
        let Ok(level_iid) = q_level.get(level_entity.get()) else {
            continue;
        };
        commands.entity(entity).insert(StartFlag {
            level_iid: level_iid.clone(),
        });
    }
}
