use std::collections::HashMap;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::tiles::TileTextureIndex;
use bevy_rapier2d::prelude::*;

use crate::{light::LightColor, lighting::Occluder2d, shared::GroupLabel};

use super::{
    entity::HurtMarker,
    merge_tile::{spawn_merged_tiles, MergedTile},
    sensor::update_light_sensors,
    CurrentLevel, LevelSystems,
};

/// [`Plugin`] for managing all things related to [`Crystal`]s. This plugin responds to the
/// addition and removal of [`Activated`] [`Component`]s and updates the sprite and collider of
/// each crystal entity, in addition to handling initialization and cleanup on a [`LevelSwitchEvent`].
pub struct CrystalPlugin;

impl Plugin for CrystalPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CrystalToggleEvent>()
            .init_resource::<CrystalCache>()
            .add_systems(
                PreUpdate,
                (
                    update_crystal_cache,
                    (
                        init_crystal_cache_tiles,
                        spawn_merged_tiles::<Crystal>,
                        init_crystal_cache_groups,
                    )
                        .chain(),
                )
                    .in_set(LevelSystems::Processing),
            )
            // Has event reader, so must be on update
            .add_systems(
                Update,
                (
                    on_crystal_changed
                        .in_set(LevelSystems::Simulation)
                        .after(update_light_sensors),
                    reset_crystals.in_set(LevelSystems::Reset),
                ),
            );

        for i in 3..=10 {
            app.register_ldtk_int_cell_for_layer::<CrystalBundle>("Terrain", i);
        }

        for i in 1..=10 {
            app.register_ldtk_int_cell_for_layer::<CrystalIdBundle>("Crystalmap", i);
        }
    }
}

/// Enum that represents the crystals that a [`LightSensor`] should toggle. Differs from the
/// LightColor in that the white color requires an ID field.
#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq)]
pub struct CrystalColor {
    pub color: LightColor,
    pub id: i32,
}

/// Marker [`Component`] used to query for crystals, currently does not contain any information.
#[derive(Default, Component)]
pub struct Crystal {
    color: CrystalColor,
    init_active: bool,
    pub active: bool,
}

impl MergedTile for Crystal {
    type CompareData = (CrystalColor, bool);

    fn bundle(
        commands: &mut EntityCommands,
        center: Vec2,
        half_extent: Vec2,
        compare_data: &Self::CompareData,
    ) {
        let (crystal_color, crystal_active) = compare_data;

        if crystal_color.color == LightColor::Blue {
            commands.insert(CollisionGroups::new(
                GroupLabel::TERRAIN,
                GroupLabel::ALL & !GroupLabel::BLUE_RAY,
            ));
        }

        if *crystal_active {
            commands.insert((
                Collider::cuboid(half_extent.x, half_extent.y),
                Occluder2d::new(half_extent.x, half_extent.y),
            ));
        }

        commands.insert((
            RigidBody::Fixed,
            Transform::from_xyz(center.x, center.y, 0.),
            CrystalGroup {
                representative: Crystal {
                    init_active: compare_data.1,
                    color: compare_data.0,
                    active: compare_data.1,
                },
                half_extent,
            },
            HurtMarker,
        ));
    }

    fn compare_data(&self) -> Self::CompareData {
        (self.color, self.init_active)
    }
}

/// [`Bundle`] registered with [`LdktEntityAppExt::register_ldtk_entity`](LdtkEntityAppExt) to spawn
/// crystals directly from Ldtk.
#[derive(Bundle, LdtkIntCell, Default)]
pub struct CrystalBundle {
    #[from_int_grid_cell]
    crystal: Crystal,
    #[from_int_grid_cell]
    cell: IntGridCell,
}

#[derive(Component)]
pub struct CrystalGroup {
    pub representative: Crystal,
    pub half_extent: Vec2,
}

/// Identifier [`Component`] used to label the ID of white crystals
#[derive(Default, Component, Clone, Copy, PartialEq)]
pub struct CrystalId(i32);

impl From<IntGridCell> for CrystalId {
    fn from(value: IntGridCell) -> Self {
        CrystalId(value.value)
    }
}

/// Bundle registered with LDTK to spawn in white crystal identifiers
#[derive(Default, Bundle, LdtkIntCell)]
pub struct CrystalIdBundle {
    #[from_int_grid_cell]
    id: CrystalId,
}

#[derive(Debug, Default, Resource)]
pub struct CrystalCache {
    tiles: HashMap<LevelIid, HashMap<CrystalColor, Vec<Entity>>>,
    groups: HashMap<LevelIid, HashMap<CrystalColor, Vec<Entity>>>,
}

fn update_crystal_cache(
    mut ev_level: EventReader<LevelEvent>,
    mut crystal_cache: ResMut<CrystalCache>,
) {
    for ev in ev_level.read() {
        let LevelEvent::Despawned(iid) = ev else {
            continue;
        };
        if let Some(mp) = crystal_cache.tiles.get_mut(iid) {
            mp.clear();
        }
        if let Some(mp) = crystal_cache.groups.get_mut(iid) {
            mp.clear();
        }
    }
}

fn init_crystal_cache_groups(
    q_crystal_groups: Query<(Entity, &Parent, &CrystalGroup), Added<CrystalGroup>>,
    q_level_iid: Query<&LevelIid>,
    mut crystal_cache: ResMut<CrystalCache>,
) {
    for (entity, parent, crystal_group) in q_crystal_groups.iter() {
        let Ok(level_iid) = q_level_iid.get(**parent) else {
            continue;
        };
        crystal_cache
            .groups
            .entry(level_iid.clone())
            .or_default()
            .entry(crystal_group.representative.color)
            .or_default()
            .push(entity);
    }
}

/// System that will initialize all the crystals, storing their entities in the appropriate level
/// -> crystal color location in the crystal cache.
#[allow(clippy::type_complexity)]
fn init_crystal_cache_tiles(
    mut commands: Commands,
    q_crystal_id: Query<(&GridCoords, &Parent, &CrystalId), (Added<CrystalId>, Without<Crystal>)>,
    mut q_crystals: Query<(Entity, &GridCoords, &Parent, &mut Crystal), Added<Crystal>>,
    q_level_iid: Query<&LevelIid>,
    q_parent: Query<&Parent, (Without<CrystalId>, Without<Crystal>)>,
    mut crystal_cache: ResMut<CrystalCache>,
) {
    if q_crystals.is_empty() {
        return;
    }

    // Hashmap of coordinates to color ids
    let mut coords_map: HashMap<LevelIid, HashMap<GridCoords, i32>> = HashMap::new();
    for (coords, parent, crystal_id) in q_crystal_id.iter() {
        let Ok(level_entity) = q_parent.get(**parent) else {
            continue;
        };
        let Ok(level_iid) = q_level_iid.get(**level_entity) else {
            continue;
        };
        coords_map
            .entry(level_iid.clone())
            .or_default()
            .insert(*coords, crystal_id.0);

        commands.entity(**parent).insert(Visibility::Hidden);
    }

    for (entity, coord, parent, mut crystal) in q_crystals.iter_mut() {
        let Ok(level_entity) = q_parent.get(**parent) else {
            continue;
        };
        let Ok(level_iid) = q_level_iid.get(**level_entity) else {
            continue;
        };

        // crystal.color is currently CrystalColor::White with id 0, we need to pull the proper ID
        // in if it exists
        let actual_color = CrystalColor {
            color: crystal.color.color,
            id: coords_map
                .get(level_iid)
                .and_then(|mp| mp.get(coord))
                .copied()
                .unwrap_or(0),
        };

        crystal_cache
            .tiles
            .entry(level_iid.clone())
            .or_default()
            .entry(actual_color)
            .or_default()
            .push(entity);

        crystal.color = actual_color;
    }
}

/// Function to determine whether or not a cell value represents an Active Crystal. Does not use
/// the modulo operator as future crystal cell values need not necessarily follow the same pattern
/// in the future.
fn is_crystal_active(cell_value: IntGridCell) -> bool {
    match cell_value.value {
        3 | 5 | 7 | 9 => true,
        4 | 6 | 8 | 10 => false,
        _ => panic!("Cell value does not correspond to crystal!"),
    }
}

/// Function to determine the base color of the crystal.
fn crystal_color(cell_value: IntGridCell) -> LightColor {
    match cell_value.value {
        3 | 4 => LightColor::Red,
        5 | 6 => LightColor::Green,
        7 | 8 => LightColor::White,
        9 | 10 => LightColor::Blue,
        _ => panic!("Cell value does not correspond to crystal!"),
    }
}

impl From<IntGridCell> for Crystal {
    fn from(cell: IntGridCell) -> Self {
        let init_active = is_crystal_active(cell);

        Crystal {
            color: CrystalColor {
                color: crystal_color(cell),
                id: 0,
            },
            active: init_active,
            init_active,
        }
    }
}

/// The horizontal offset between active crystals and inactive crystals in the crystal tilemap
const CRYSTAL_INDEX_OFFSET: u32 = 5;

fn toggle_crystal_group(
    commands: &mut Commands,
    crystal_group_entity: Entity,
    crystal_group: &mut CrystalGroup,
) {
    let crystal = &mut crystal_group.representative;
    if !crystal.active {
        crystal.active = true;
        commands.entity(crystal_group_entity).insert((
            Collider::cuboid(crystal_group.half_extent.x, crystal_group.half_extent.y),
            Occluder2d::new(crystal_group.half_extent.x, crystal_group.half_extent.y),
        ));
    } else {
        crystal.active = false;
        commands
            .entity(crystal_group_entity)
            .remove::<(Collider, Occluder2d)>();
    }
}

fn toggle_crystal(crystal: &mut Crystal, crystal_index: &mut TileTextureIndex) {
    if !crystal.active {
        crystal.active = true;
        crystal_index.0 -= CRYSTAL_INDEX_OFFSET;
    } else {
        crystal.active = false;
        crystal_index.0 += CRYSTAL_INDEX_OFFSET;
    }
}

/// [`System`] that listens to [`LevelSwitchEvent`]s to ensure that [`Crystal`] states are reset
/// when switching between rooms.
pub fn reset_crystals(
    mut commands: Commands,
    mut q_crystals: Query<(&mut Crystal, &mut TileTextureIndex)>,
    mut q_crystal_groups: Query<(Entity, &mut CrystalGroup)>,
) {
    for (entity, mut crystal_group) in q_crystal_groups.iter_mut() {
        let crystal = &crystal_group.representative;
        if crystal.init_active != crystal.active {
            toggle_crystal_group(&mut commands, entity, &mut crystal_group);
        }
    }

    for (mut crystal, mut index) in q_crystals.iter_mut() {
        if crystal.init_active != crystal.active {
            toggle_crystal(&mut crystal, &mut index);
        }
    }
}

/// Event that will toggle all crystals of a certain color.
#[derive(Event)]
pub struct CrystalToggleEvent {
    pub color: CrystalColor,
}

/// [`System`] that listens to when [`Crystal`]s are activated or deactivated, updating the
/// [`Sprite`] and adding/removing [`FixedEntityBundle`] of the [`Entity`].
pub fn on_crystal_changed(
    mut commands: Commands,
    mut q_crystal: Query<(&mut Crystal, &mut TileTextureIndex)>,
    mut q_crystal_groups: Query<&mut CrystalGroup>,
    mut crystal_toggle_ev: EventReader<CrystalToggleEvent>,
    crystal_cache: Res<CrystalCache>,
    current_level: Res<CurrentLevel>,
) {
    if crystal_toggle_ev.is_empty() {
        return;
    }
    let Some(crystal_tile_map) = crystal_cache.tiles.get(&current_level.level_iid) else {
        return;
    };
    let Some(crystal_group_map) = crystal_cache.groups.get(&current_level.level_iid) else {
        return;
    };

    for CrystalToggleEvent { color } in crystal_toggle_ev.read() {
        if let Some(crystals) = crystal_tile_map.get(color) {
            for crystal_entity in crystals.iter() {
                let Ok((mut crystal, mut index)) = q_crystal.get_mut(*crystal_entity) else {
                    continue;
                };
                toggle_crystal(&mut crystal, &mut index);
            }
        };
        if let Some(crystal_groups) = crystal_group_map.get(color) {
            for crystal_group_entity in crystal_groups.iter() {
                let Ok(mut crystal_group) = q_crystal_groups.get_mut(*crystal_group_entity) else {
                    continue;
                };
                toggle_crystal_group(&mut commands, *crystal_group_entity, &mut crystal_group);
            }
        }
    }
}
