use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::ldtk::LdtkLevelParam;

pub trait MergedTile {
    type CompareData: PartialEq + Eq + Hash;

    fn bundle(
        commands: &mut EntityCommands,
        center: Vec2,
        extent: Vec2,
        compare_data: &Self::CompareData,
    );

    fn compare_data(&self) -> Self::CompareData;
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
struct Plate {
    left: i32,
    right: i32,
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

fn build_rects_by_row(width: i32, height: i32, tiles: &HashSet<GridCoords>) -> Vec<Rect> {
    let mut plate_stack: Vec<Vec<Plate>> = Vec::new();

    for y in 0..height {
        let mut row_plates: Vec<Plate> = Vec::new();
        let mut plate_start: Option<i32> = None;

        // +1 so we "close" any plate that hits the right edge
        for x in 0..(width + 1) {
            match (plate_start, tiles.contains(&GridCoords { x, y })) {
                (Some(s), false) => {
                    row_plates.push(Plate {
                        left: s,
                        right: x - 1,
                    });
                    plate_start = None;
                }
                (None, true) => plate_start = Some(x),
                _ => {}
            }
        }
        plate_stack.push(row_plates);
    }

    // extra empty row so we "close" plates that hit the top edge
    plate_stack.push(Vec::new());

    let mut rect_builder: HashMap<Plate, Rect> = HashMap::new();
    let mut prev_row: Vec<Plate> = Vec::new();
    let mut rects: Vec<Rect> = Vec::new();

    for (y, current_row) in plate_stack.into_iter().enumerate() {
        for prev_plate in &prev_row {
            if !current_row.contains(prev_plate) {
                if let Some(rect) = rect_builder.remove(prev_plate) {
                    rects.push(rect);
                }
            }
        }

        for plate in &current_row {
            rect_builder
                .entry(plate.clone())
                .and_modify(|r| r.top += 1)
                .or_insert(Rect {
                    bottom: y as i32,
                    top: y as i32,
                    left: plate.left,
                    right: plate.right,
                });
        }

        prev_row = current_row;
    }

    rects
}

// fn transpose_coords_set(tiles: &HashSet<GridCoords>) -> HashSet<GridCoords> {
//     tiles
//         .iter()
//         .map(|&GridCoords { x, y }| GridCoords { x: y, y: x })
//         .collect()
// }
//
// fn build_rects_by_column(width: i32, height: i32, tiles: &HashSet<GridCoords>) -> Vec<Rect> {
//     let tset = transpose_coords_set(tiles);
//     let mut rects_t = build_rects_by_row(height, width, &tset);
//
//     for r in &mut rects_t {
//         let left = r.bottom;
//         let right = r.top;
//         let bottom = r.left;
//         let top = r.right;
//         r.left = left;
//         r.right = right;
//         r.bottom = bottom;
//         r.top = top;
//     }
//
//     rects_t
// }

pub fn spawn_merged_tiles<Tile>(
    mut commands: Commands,
    tile_query: Query<(&GridCoords, &ChildOf, &Tile), Added<Tile>>,
    parent_query: Query<&ChildOf, Without<Tile>>,
    level_query: Query<(Entity, &LevelIid)>,
    ldtk_level_param: LdtkLevelParam,
) where
    Tile: MergedTile + Component,
{
    if tile_query.is_empty() {
        return;
    }

    let mut level_to_tile_locations: HashMap<
        Entity,
        HashMap<Tile::CompareData, HashSet<GridCoords>>,
    > = HashMap::new();

    tile_query.iter().for_each(|(&grid_coords, parent, tile)| {
        if let Ok(grandparent) = parent_query.get(parent.0) {
            level_to_tile_locations
                .entry(grandparent.0)
                .or_default()
                .entry(tile.compare_data())
                .or_default()
                .insert(grid_coords);
        }
    });

    level_query.iter().for_each(|(level_entity, level_iid)| {
        let Some(level_tiles) = level_to_tile_locations.get(&level_entity) else {
            return;
        };

        let level = ldtk_level_param
            .level_by_iid(level_iid)
            .expect("Spawned level should exist in LDtk project");

        let LayerInstance {
            c_wid: width,
            c_hei: height,
            grid_size,
            ..
        } = level.layer_instances()[0];

        let grid = grid_size as f32;

        for (compare_data, tile_coords) in level_tiles.iter() {
            let rects_h = build_rects_by_row(width, height, tile_coords);
            // let rects_v = build_rects_by_column(width, height, tile_coords);

            commands.entity(level_entity).with_children(|level| {
                for r in rects_h.iter() {
                    let extent = Vec2::new(
                        (r.right - r.left + 1) as f32 * grid,
                        (r.top - r.bottom + 1) as f32 * grid,
                    )
                    .abs();

                    let center = Vec2::new(
                        (r.left + r.right + 1) as f32 * grid / 2.0,
                        (r.bottom + r.top + 1) as f32 * grid / 2.0,
                    );

                    Tile::bundle(&mut level.spawn_empty(), center, extent, compare_data);
                }
                //cover gaps because sadge
                // for r in rects_v.iter() {
                //     let extent = Vec2::new(
                //         (r.right - r.left + 1) as f32 * grid,
                //         (r.top - r.bottom + 1) as f32 * grid - 0.1,
                //     )
                //     .abs();
                //
                //     let center = Vec2::new(
                //         (r.left + r.right + 1) as f32 * grid / 2.0,
                //         (r.bottom + r.top + 1) as f32 * grid / 2.0,
                //     );
                //
                //     Tile::bundle(&mut level.spawn_empty(), center, extent, compare_data);
                // }
            });
        }
    });
}
