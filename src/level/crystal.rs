use std::collections::HashMap;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::tiles::TileTextureIndex;
use bevy_rapier2d::prelude::*;

use crate::{light::LightColor, lighting::occluder::ColliderBasedOccluder, shared::GroupLabel};

use super::{entity::HurtMarker, CurrentLevel, LevelSystems};

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
                    init_crystal_cache_and_ids,
                    add_crystal_colliders,
                    update_crystal_cache,
                )
                    .in_set(LevelSystems::Processing),
            )
            // Has event reader, so must be on update
            .add_systems(
                Update,
                (
                    on_crystal_changed.in_set(LevelSystems::Simulation),
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
    levels: HashMap<LevelIid, HashMap<CrystalColor, Vec<Entity>>>,
}

fn update_crystal_cache(
    mut ev_level: EventReader<LevelEvent>,
    mut crystal_cache: ResMut<CrystalCache>,
) {
    for ev in ev_level.read() {
        let LevelEvent::Despawned(iid) = ev else {
            continue;
        };
        if let Some(mp) = crystal_cache.levels.get_mut(iid) {
            mp.clear();
        }
    }
}

/// System that will initialize all the crystals, storing their entities in the appropriate level
/// -> crystal color location in the crystal cache.
#[allow(clippy::type_complexity)]
fn init_crystal_cache_and_ids(
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
            .levels
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

/// [`Bundle`] registered with [`LdktEntityAppExt::register_ldtk_entity`](LdtkEntityAppExt) to spawn
/// crystals directly from Ldtk.
#[derive(Bundle, LdtkIntCell)]
pub struct CrystalBundle {
    #[from_int_grid_cell]
    crystal: Crystal,
    #[from_int_grid_cell]
    cell: IntGridCell,
    collider_based_occluder: ColliderBasedOccluder,
    hurt_marker: HurtMarker,
}

impl Default for CrystalBundle {
    fn default() -> Self {
        Self {
            collider_based_occluder: ColliderBasedOccluder { indent: 2.0 },
            crystal: Crystal::default(),
            cell: IntGridCell::default(),
            hurt_marker: HurtMarker,
        }
    }
}

fn add_crystal_colliders(
    mut commands: Commands,
    q_crystals: Query<(Entity, &IntGridCell), Added<Crystal>>,
) {
    for (entity, cell) in q_crystals.iter() {
        if crystal_color(*cell) == LightColor::Blue {
            let mut collider = commands.entity(entity);
            collider.insert(CollisionGroups::new(
                GroupLabel::TERRAIN,
                GroupLabel::ALL & !GroupLabel::BLUE_RAY,
            ));
        }
        if is_crystal_active(*cell) {
            let mut collider = commands.entity(entity);
            collider.insert(Collider::cuboid(4.0, 4.0));
        }
    }
}

/// The horizontal offset between active crystals and inactive crystals in the crystal tilemap
const CRYSTAL_INDEX_OFFSET: u32 = 5;

/// Switches a crystal from inactive to active. Calling this on an already active crystal will
/// result in weird behavior.
fn activate_crystal(
    commands: &mut Commands,
    crystal_entity: Entity,
    crystal_index: &mut TileTextureIndex,
) {
    commands
        .entity(crystal_entity)
        .insert(Collider::cuboid(4.0, 4.0));
    crystal_index.0 -= CRYSTAL_INDEX_OFFSET;
}

/// Switches a crystal from active to inactive. Calling this on an already inactive crystal will
/// result in weird behavior.
fn deactivate_crystal(
    commands: &mut Commands,
    crystal_entity: Entity,
    crystal_index: &mut TileTextureIndex,
) {
    commands.entity(crystal_entity).remove::<Collider>();
    crystal_index.0 += CRYSTAL_INDEX_OFFSET;
}

/// [`System`] that listens to [`LevelSwitchEvent`]s to ensure that [`Crystal`] states are reset
/// when switching between rooms.
pub fn reset_crystals(
    mut commands: Commands,
    mut q_crystals: Query<(Entity, &mut Crystal, &mut TileTextureIndex)>,
) {
    for (entity, mut crystal, mut index) in q_crystals.iter_mut() {
        if crystal.active == crystal.init_active {
            continue;
        }
        if crystal.init_active {
            activate_crystal(&mut commands, entity, &mut index);
            crystal.active = true;
        } else {
            deactivate_crystal(&mut commands, entity, &mut index);
            crystal.active = false;
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
    mut crystal_toggle_ev: EventReader<CrystalToggleEvent>,
    crystal_cache: Res<CrystalCache>,
    current_level: Res<CurrentLevel>,
) {
    if crystal_toggle_ev.is_empty() {
        return;
    }
    let Some(color_map) = crystal_cache.levels.get(&current_level.level_iid) else {
        return;
    };

    for CrystalToggleEvent { color } in crystal_toggle_ev.read() {
        let Some(crystals) = color_map.get(color) else {
            continue;
        };
        for crystal_entity in crystals.iter() {
            let Ok((mut crystal, mut index)) = q_crystal.get_mut(*crystal_entity) else {
                continue;
            };

            if crystal.active {
                deactivate_crystal(&mut commands, *crystal_entity, &mut index);
                crystal.active = false;
            } else {
                activate_crystal(&mut commands, *crystal_entity, &mut index);
                crystal.active = true;
            }
        }
    }
}
