use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub trait MergedTile {
    /// The comparison data used to compute if two tiles are mergeable or not
    type CompareData: PartialEq + Eq + Hash;

    /// This function should spawn the merged tile's components using the given EntityCommands. The
    /// given Entity commands refers to an entity that is a direct child of the level, not the
    /// layer.
    fn bundle(
        commands: &mut EntityCommands,
        center: Vec2,
        half_extent: Vec2,
        compare_data: &Self::CompareData,
    );

    /// This function should return the compare data used to check merge-ability between entities
    /// with the same component
    fn compare_data(&self) -> Self::CompareData;
}

pub fn spawn_merged_tiles<TILE>(
    mut commands: Commands,
    tile_query: Query<(&GridCoords, &Parent, &TILE), Added<TILE>>,
    parent_query: Query<&Parent, Without<TILE>>,
    level_query: Query<(Entity, &LevelIid)>,
    ldtk_projects: Query<&LdtkProjectHandle>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
) where
    TILE: MergedTile + Component,
{
    if tile_query.is_empty() {
        return;
    }
    #[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
    struct Plate {
        left: i32,
        right: i32,
    }

    struct Rect {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }

    let mut level_to_tile_locations: HashMap<
        Entity,
        HashMap<TILE::CompareData, HashSet<GridCoords>>,
    > = HashMap::new();

    tile_query.iter().for_each(|(&grid_coords, parent, tile)| {
        if let Ok(grandparent) = parent_query.get(parent.get()) {
            level_to_tile_locations
                .entry(grandparent.get())
                .or_default()
                .entry(tile.compare_data())
                .or_default()
                .insert(grid_coords);
        }
    });

    let ldtk_project = ldtk_project_assets
        .get(ldtk_projects.single())
        .expect("Project should be loaded if level has spawned");

    level_query.iter().for_each(|(level_entity, level_iid)| {
        let Some(level_tiles) = level_to_tile_locations.get(&level_entity) else {
            return;
        };

        let level = ldtk_project
            .as_standalone()
            .get_loaded_level_by_iid(&level_iid.to_string())
            .expect("Spawned level should exist in LDtk project");

        let LayerInstance {
            c_wid: width,
            c_hei: height,
            grid_size,
            ..
        } = level.layer_instances()[0];

        for (compare_data, tile_coords) in level_tiles.iter() {
            let mut plate_stack: Vec<Vec<Plate>> = Vec::new();

            for y in 0..height {
                let mut row_plates: Vec<Plate> = Vec::new();
                let mut plate_start = None;

                // + 1 to the width so the algorithm "terminates" plates that touch the right edge
                for x in 0..width + 1 {
                    match (plate_start, tile_coords.contains(&GridCoords { x, y })) {
                        (Some(s), false) => {
                            row_plates.push(Plate {
                                left: s,
                                right: x - 1,
                            });
                            plate_start = None;
                        }
                        (None, true) => plate_start = Some(x),
                        _ => (),
                    }
                }

                plate_stack.push(row_plates);
            }

            // combine "plates" into rectangles across multiple rows
            let mut rect_builder: HashMap<Plate, Rect> = HashMap::new();
            let mut prev_row: Vec<Plate> = Vec::new();
            let mut tile_rects: Vec<Rect> = Vec::new();

            // an extra empty row so the algorithm "finishes" the rects that touch the top edge
            plate_stack.push(Vec::new());

            for (y, current_row) in plate_stack.into_iter().enumerate() {
                for prev_plate in &prev_row {
                    if !current_row.contains(prev_plate) {
                        // remove the finished rect so that the same plate in the future starts a new rect
                        if let Some(rect) = rect_builder.remove(prev_plate) {
                            tile_rects.push(rect);
                        }
                    }
                }
                for plate in &current_row {
                    rect_builder
                        .entry(plate.clone())
                        .and_modify(|e| e.top += 1)
                        .or_insert(Rect {
                            bottom: y as i32,
                            top: y as i32,
                            left: plate.left,
                            right: plate.right,
                        });
                }
                prev_row = current_row;
            }

            commands.entity(level_entity).with_children(|level| {
                for tile_rect in tile_rects {
                    let half_extent = Vec2::new(
                        (tile_rect.right as f32 - tile_rect.left as f32 + 1.) * grid_size as f32
                            / 2.,
                        (tile_rect.top as f32 - tile_rect.bottom as f32 + 1.) * grid_size as f32
                            / 2.,
                    );
                    let center = Vec2::new(
                        (tile_rect.left + tile_rect.right + 1) as f32 * grid_size as f32 / 2.,
                        (tile_rect.bottom + tile_rect.top + 1) as f32 * grid_size as f32 / 2.,
                    );
                    TILE::bundle(&mut level.spawn_empty(), center, half_extent, compare_data);
                }
            });
        }
    });
}
