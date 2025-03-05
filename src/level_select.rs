use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::audio::PlaybackMode;
use bevy::image::{BevyDefault, TextureFormatPixelInfo};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_ecs_ldtk::ldtk::{FieldValue, Type};
use bevy_ecs_ldtk::prelude::LdtkFields;
use bevy_ecs_ldtk::LevelIid;
use bevy_ecs_ldtk::{prelude::LdtkProject, LdtkProjectHandle};

use crate::camera::{camera_position_from_level, handle_move_camera, CameraMoveEvent};
use crate::level::start_flag::StartFlag;
use crate::level::{get_ldtk_level_data, level_box_from_level, CurrentLevel};
use crate::player::PlayerMarker;
use crate::shared::{GameState, UiState, LYRA_RESPAWN_EPSILON};

pub struct LevelSelectPlugin;

const START_FLAG_IDENT: &str = "Start";
const TERRAIN_LAYER_IDENT: &str = "Terrain";
const ENTITY_LAYER_IDENT: &str = "Entities";
const SENSOR_ENTITY_IDENT: &str = "Sensor";
const SENSOR_COLOR_IDENT: &str = "light_color";

// [R, G, B, A] colors for level preview
const LEVEL_PREVIEW_COLORS: [[u8; 4]; 16] = [
    [0, 0, 0, 255],       // intgrid 0
    [41, 54, 78, 255],    // intgrid 1
    [117, 158, 202, 255], // intgrid 2
    [255, 0, 68, 255],    // intgrid 3
    [71, 1, 19, 255],     // intgrid 4
    [99, 199, 77, 255],   // intgrid 5
    [30, 61, 23, 255],    // intgrid 6
    [192, 203, 220, 255], // intgrid 7
    [55, 58, 62, 255],    // intgrid 8
    [80, 183, 56, 255],   // intgrid 9
    [43, 85, 136, 255],   // intgrid 10
    [0, 0, 0, 255],       // intgrid 11
    [0, 0, 0, 255],       // intgrid 12
    [0, 0, 0, 255],       // intgrid 13
    [0, 0, 0, 255],       // intgrid 14
    [115, 62, 57, 255],   // intgrid 15
];

fn sensor_color_to_rgba(sensor_color: &str) -> [u8; 4] {
    match sensor_color {
        "Red" => [255, 143, 212, 255],
        "Green" => [157, 253, 148, 255],
        "White" => [229, 229, 229, 255],
        "Blue" => [143, 225, 255, 255],
        _ => [0, 0, 0, 255],
    }
}

#[derive(Component)]
struct LevelSelectUiMarker;

#[derive(Component)]
pub struct LevelPreviewMarker;

#[derive(Resource)]
pub struct LevelPreviewStore(HashMap<String, Handle<Image>>);

#[derive(Component)]
pub struct LevelSelectButtonIndex(usize);

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LevelPreviewStore(HashMap::new()))
            .add_systems(
                FixedUpdate,
                (
                    spawn_level_select.run_if(in_state(UiState::LevelSelect)),
                    despawn_level_select
                        .after(handle_move_camera)
                        .run_if(not(in_state(UiState::LevelSelect))),
                    handle_level_selection.run_if(in_state(UiState::LevelSelect)),
                ),
            );
    }
}

fn spawn_level_select(
    mut commands: Commands,
    ldtk_assets: Res<Assets<LdtkProject>>,
    query_ldtk: Query<&LdtkProjectHandle>,
    level_select_ui_query: Query<Entity, With<LevelSelectUiMarker>>,
    asset_server: Res<AssetServer>,
) {
    if level_select_ui_query.get_single().is_ok() {
        return;
    }
    let Ok(levels) = get_ldtk_level_data(ldtk_assets, query_ldtk) else {
        return;
    };
    let mut sorted_levels = Vec::with_capacity(levels.len());
    for (i, level) in levels.iter().enumerate() {
        let level_id = level
            .get_string_field("LevelId")
            .expect("Levels should always have a level id!");
        if level_id.is_empty() {
            panic!("Level id for a level should not be empty!");
        }
        sorted_levels.push((level_id, i));
    }
    sorted_levels.sort();

    commands
        .spawn((
            LevelSelectUiMarker,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            AudioPlayer::new(asset_server.load("music/main_menu.wav")),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(Text::new("Level Select"));
            parent
                .spawn(Node {
                    width: Val::Percent(100.),
                    height: Val::Auto,
                    ..default()
                })
                .with_children(|parent| {
                    for (level_id, index) in sorted_levels.iter() {
                        parent
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Percent(4.),
                                    height: Val::Auto,
                                    border: UiRect::all(Val::Percent(0.2)),
                                    margin: UiRect::all(Val::Percent(0.5)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BorderColor(Color::WHITE),
                                LevelSelectButtonIndex(*index),
                            ))
                            .with_child(Text::new(level_id.to_string()));
                    }
                });
            parent
                .spawn((Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|parent| {
                    parent.spawn((
                        Node {
                            width: Val::Percent(30.),
                            height: Val::Auto,
                            ..default()
                        },
                        LevelPreviewMarker,
                    ));
                });
        });
}

fn despawn_level_select(
    mut commands: Commands,
    mut level_select_ui_query: Query<Entity, With<LevelSelectUiMarker>>,
) {
    let Ok(entity) = level_select_ui_query.get_single_mut() else {
        return;
    };

    commands.entity(entity).despawn_recursive();
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn handle_level_selection(
    mut interaction_query: Query<
        (&Interaction, &LevelSelectButtonIndex),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_ui_state: ResMut<NextState<UiState>>,
    ldtk_assets: Res<Assets<LdtkProject>>,
    query_ldtk: Query<&LdtkProjectHandle>,
    mut query_player: Query<&mut Transform, (With<PlayerMarker>, Without<StartFlag>)>,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
    mut current_level: ResMut<CurrentLevel>,
    mut level_preview_store: ResMut<LevelPreviewStore>,
    mut assets: ResMut<Assets<Image>>,
    mut query_level_preview: Query<(Entity, Option<&mut ImageNode>), With<LevelPreviewMarker>>,
    mut commands: Commands,
) {
    // We expect there to be only one interaction
    if let Some((interaction, index)) = (&mut interaction_query).into_iter().next() {
        let Ok(ldtk_levels) = get_ldtk_level_data(ldtk_assets, query_ldtk) else {
            return;
        };
        if index.0 >= ldtk_levels.len() {
            panic!("Selected level index is out of bounds!")
        }
        // <<<<<<< HEAD
        let level = &ldtk_levels[index.0];
        match *interaction {
            Interaction::Pressed => {
                let Some(layers) = level.layer_instances.as_ref() else {
                    panic!("Layers not found! (This is probably because you are using the \"Separate level files\" option.)")
                };
                'loop_layers: for layer in layers {
                    if layer.layer_instance_type == Type::Entities {
                        for entity in &layer.entity_instances {
                            if entity.identifier == START_FLAG_IDENT {
                                let (Some(player_x), Some(player_y)) =
                                    (entity.world_x, entity.world_y)
                                else {
                                    panic!("Start flag entity has no coordinates! (This is probably because your LDTK world is not in free layout mode.)");
                                };
                                let Ok(mut player_transform) = query_player.get_single_mut() else {
                                    panic!("Could not find player!");
                                };
                                player_transform.translation.x = player_x as f32;
                                player_transform.translation.y =
                                    -player_y as f32 + LYRA_RESPAWN_EPSILON;

                                // Send a camera transition event to tp the camera immediately
                                let camera_pos = camera_position_from_level(
                                    level_box_from_level(&ldtk_levels[index.0]),
                                    player_transform.translation.xy(),
                                );
                                ev_move_camera.send(CameraMoveEvent::Instant { to: camera_pos });

                                break 'loop_layers;
                            }
                        }
                    }
                }

                next_game_state.set(GameState::Playing);
                next_ui_state.set(UiState::None);
                // Set the current level_iid to an empty string so we don't trigger the camera transition (skull emoji)
                current_level.level_iid = LevelIid::new("");
            }
            Interaction::Hovered => {
                let level_id = level
                    .get_string_field("LevelId")
                    .expect("Levels should always have a level id!");
                let level_preview = match level_preview_store.0.get(level_id) {
                    Some(handle) => handle.clone(),
                    None => {
                        let level_layers =
                            level.layer_instances.as_ref().expect("Layers not found!");
                        let Some((layer_w, layer_h, layer_data)) =
                            level_layers.iter().find_map(|layer| {
                                if layer.identifier == TERRAIN_LAYER_IDENT {
                                    Some((
                                        layer.c_wid as usize,
                                        layer.c_hei as usize,
                                        &layer.int_grid_csv,
                                    ))
                                } else {
                                    None
                                }
                            })
                        else {
                            panic!("Terrain layer data not found!");
                        };
                        let Some(level_entities) = level_layers.iter().find_map(|layer| {
                            if layer.identifier == ENTITY_LAYER_IDENT {
                                Some(&layer.entity_instances)
                            } else {
                                None
                            }
                        }) else {
                            panic!("Entity layer data not found!");
                        };
                        let mut level_preview_data = Vec::with_capacity(layer_w * layer_h);
                        let pixel_size = TextureFormat::bevy_default().pixel_size();
                        for tile in layer_data {
                            for i in 0..pixel_size {
                                level_preview_data.push(LEVEL_PREVIEW_COLORS[*tile as usize][i]);
                            }
                        }
                        for entity in level_entities {
                            if entity.identifier != SENSOR_ENTITY_IDENT {
                                continue;
                            }
                            let entity_coords = entity.grid;
                            let Some(entity_color) =
                                entity.field_instances.iter().find_map(|instance| {
                                    if instance.identifier == SENSOR_COLOR_IDENT {
                                        let FieldValue::Enum(Some(ref color)) = instance.value
                                        else {
                                            panic!("Sensor color should be an enum!");
                                        };
                                        Some(color)
                                    } else {
                                        None
                                    }
                                })
                            else {
                                panic!("Could not find sensor color field!");
                            };
                            let rgba = sensor_color_to_rgba(entity_color);
                            let image_data_index = (entity_coords.y as usize * layer_w
                                + entity_coords.x as usize)
                                * pixel_size;
                            level_preview_data[image_data_index..(pixel_size + image_data_index)]
                                .copy_from_slice(&rgba[..pixel_size]);
                        }
                        let preview = Image::new(
                            Extent3d {
                                width: layer_w as u32,
                                height: layer_h as u32,
                                depth_or_array_layers: 1,
                            },
                            TextureDimension::D2,
                            level_preview_data,
                            TextureFormat::bevy_default(),
                            RenderAssetUsages::RENDER_WORLD,
                        );
                        let new_handle = assets.add(preview);
                        level_preview_store
                            .0
                            .insert(level_id.into(), new_handle.clone());
                        new_handle
                    }
                };
                let Ok((level_preview_entity, level_preview_image_node)) =
                    query_level_preview.get_single_mut()
                else {
                    panic!("Could not find level preview");
                };
                if let Some(mut level_preview_image_node) = level_preview_image_node {
                    level_preview_image_node.image = level_preview;
                } else {
                    let image_node = ImageNode::new(level_preview);
                    commands.entity(level_preview_entity).insert(image_node);
                }
            }
            _ => {}
        }
    }
}
