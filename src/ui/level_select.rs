use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::{BevyDefault, TextureFormatPixelInfo};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_ecs_ldtk::ldtk::FieldValue;
use bevy_ecs_ldtk::prelude::LdtkFields;
use bevy_ecs_ldtk::LevelSelection;
use serde::{Deserialize, Serialize};

use crate::asset::LoadResource;
use crate::config::Config;
use crate::game::light::LightColor;
use crate::ldtk::LdtkParam;
use crate::save::SaveParam;
use crate::shared::{GameState, UiState};
use crate::sound::{BgmTrack, ChangeBgmEvent};
use crate::ui::{UiButton, UiClick, UiFont, UiFontSize};

pub struct LevelSelectPlugin;

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LevelSelectAssets>();
        app.load_resource::<LevelSelectAssets>();
        app.insert_resource(LevelPreviewStore(HashMap::new()));
        app.insert_resource(LevelProgress(Vec::new()));
        app.add_systems(OnExit(UiState::Leaderboard), init_levels);
        app.add_systems(OnEnter(UiState::LevelSelect), spawn_level_select);
        app.add_systems(
            Update,
            handle_level_selection.run_if(in_state(UiState::LevelSelect)),
        );
        app.add_systems(OnExit(UiState::LevelSelect), despawn_level_select);
    }
}

const TERRAIN_LAYER_IDENT: &str = "Terrain";
const ENTITY_LAYER_IDENT: &str = "Entities";
const SENSOR_ENTITY_IDENT: &str = "Sensor";
const SENSOR_COLOR_IDENT: &str = "toggle_color";

// [R, G, B, A] colors for level preview
const LEVEL_PREVIEW_COLORS: [[u8; 4]; 17] = [
    [0, 0, 0, 255],       // intgrid 0
    [41, 54, 78, 255],    // intgrid 1
    [117, 158, 202, 255], // intgrid 2
    [255, 143, 212, 255], // intgrid 3
    [128, 0, 64, 255],    // intgrid 4
    [255, 0, 0, 255],     // intgrid 5
    [80, 20, 15, 255],    // intgrid 6
    [192, 203, 220, 255], // intgrid 7
    [55, 58, 62, 255],    // intgrid 8
    [80, 150, 230, 255],  // intgrid 9
    [43, 85, 136, 255],   // intgrid 10
    [0, 0, 0, 255],       // intgrid 11
    [0, 0, 0, 255],       // intgrid 12
    [0, 0, 0, 255],       // intgrid 13
    [0, 0, 0, 255],       // intgrid 14
    [115, 62, 57, 255],   // intgrid 15
    [200, 200, 200, 255], // intgrid 16
];

fn sensor_color_to_rgba(sensor_color: &str) -> [u8; 4] {
    match sensor_color {
        "Pink" => [255, 143, 212, 255],
        "Red" => [255, 0, 0, 255],
        "White" => [229, 229, 229, 255],
        "Blue" => [143, 225, 255, 255],
        _ => [0, 0, 0, 255],
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelSelectAssets {
    lock: Handle<Image>,
}

impl FromWorld for LevelSelectAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            lock: asset_server.load("lock.png"),
        }
    }
}

#[derive(Component)]
struct LevelSelectUiMarker;

#[derive(Component)]
pub struct LevelPreviewMarker;

#[derive(Component)]
pub struct LevelPreviewLockedMarker;

#[derive(Resource)]
pub struct LevelPreviewStore(HashMap<String, (Vec2, Handle<Image>)>);
// FIXME .0 is ldtk level index, .1 is index into the Levels.0 vector
#[derive(Component)]
pub struct LevelSelectButtonIndex(usize, usize);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LevelSolution {
    pub light_order: Vec<LightColor>,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct LevelSaveData {
    pub level_id: String,
    pub level_iid: String,
    level_index: usize,
    pub sorted_level_index: usize,
    pub complete: bool,
    pub locked: bool,
    pub solutions_used: Vec<LevelSolution>,
}

impl Ord for LevelSaveData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.level_id.cmp(&other.level_id)
    }
}

impl PartialOrd for LevelSaveData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct LevelProgress(pub Vec<LevelSaveData>);

impl LevelProgress {
    pub fn solved(&self) -> usize {
        let mut count = 0;
        for level in self.0.iter() {
            if level.complete {
                count += 1;
            }
        }
        count
    }
}

fn init_levels(
    mut res_levels: ResMut<LevelProgress>,
    ldtk_param: LdtkParam,
    config: Res<Config>,
    save_param: SaveParam,
) {
    let Some(project) = ldtk_param.project() else {
        return;
    };
    if let Some(save_data) = save_param.get_save_data() {
        *res_levels = save_data.level.clone();
    } else {
        *res_levels = LevelProgress(Vec::new());
        for (i, level) in project.json_data().levels.iter().enumerate() {
            let level_id = level
                .get_string_field("LevelId")
                .expect("Levels should always have a level id!");

            if level_id.is_empty() {
                panic!("Level id for a level should not be empty!");
            }

            let should_show = level
                .get_bool_field("Selectable")
                .expect("Levels should have property Selectable");

            if !should_show {
                continue;
            }
            res_levels.0.push(LevelSaveData {
                level_id: level_id.to_string(),
                level_iid: level.iid.clone(),
                level_index: i,
                sorted_level_index: 0,
                complete: false,
                locked: true,
                solutions_used: Vec::new(),
            });
        }
        res_levels.0.sort();
        for (i, level) in res_levels.0.iter_mut().enumerate() {
            level.sorted_level_index = i;
        }
        res_levels.0[0].locked = false;
    }

    if config.debug_config.unlock_levels {
        for data in res_levels.0.iter_mut() {
            data.complete = true;
            data.locked = false;
        }
    }
}

fn spawn_level_select(
    mut commands: Commands,
    sorted_levels: Res<LevelProgress>,
    ui_font: Res<UiFont>,
    level_select_assets: Res<LevelSelectAssets>,
) {
    info!("Spawning Level Select!");

    commands.trigger(ChangeBgmEvent(BgmTrack::LevelSelect));

    let container = commands
        .spawn(LevelSelectUiMarker)
        .insert(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            max_width: Val::Percent(100.),
            max_height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(96.0)),
            column_gap: Val::Px(32.),
            row_gap: Val::Px(32.),
            ..default()
        })
        .insert(GlobalZIndex(1000))
        .insert(BackgroundColor(Color::BLACK))
        .id();

    commands
        .spawn(Text::new("Level Select"))
        .insert(ui_font.text_font().with_font_size(UiFontSize::HEADER))
        .insert(ChildOf(container));

    let center_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Auto,
            flex_grow: 1.0,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(32.),
            row_gap: Val::Px(32.),
            ..default()
        })
        .insert(ChildOf(container))
        .id();

    let level_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Auto,
            flex_grow: 0.,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .insert(ChildOf(center_container))
        .id();

    for (
        i,
        LevelSaveData {
            level_id,
            level_iid: _,
            level_index: index,
            complete,
            locked,
            solutions_used: _,
            sorted_level_index: _,
        },
    ) in sorted_levels.0.iter().enumerate()
    {
        let level_box = commands
            .spawn(Button)
            .insert(UiButton)
            .insert(Node {
                width: Val::Px(96.0),
                height: Val::Px(96.0),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::all(Val::Px(4.0)),
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .insert(BorderColor::all(if *complete {
                Color::srgb(0.0, 1.0, 0.0)
            } else if !*locked {
                Color::WHITE
            } else {
                Color::srgb(1.0, 0.0, 0.0)
            }))
            .insert(LevelSelectButtonIndex(*index, i))
            .insert(ChildOf(level_container))
            .id();

        commands
            .spawn(if *locked {
                Text::new("-")
            } else {
                Text::new(level_id.to_string())
            })
            .insert(ui_font.text_font().with_font_size(24.))
            .insert(ChildOf(level_box));
    }

    let level_preview_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_grow: 3.,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .insert(ChildOf(center_container))
        .id();

    let level_preview = commands
        .spawn(LevelPreviewMarker)
        .insert(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            position_type: PositionType::Relative,
            ..default()
        })
        .insert(ChildOf(level_preview_container))
        .id();

    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            top: Val::Percent(50.),
            left: Val::Percent(50.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .insert(UiTransform::from_translation(Val2::new(
            Val::Percent(-50.),
            Val::Percent(-50.),
        )))
        .insert(ChildOf(level_preview))
        .insert(LevelPreviewLockedMarker)
        .insert(
            ImageNode::new(level_select_assets.lock.clone())
                .with_color(Color::srgba(1., 1., 1., 0.)),
        );

    commands
        .spawn(Text::new("Back"))
        .insert(Button)
        .insert(UiButton)
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .insert(ChildOf(container))
        .observe(
            |_: On<UiClick>, mut next_ui_state: ResMut<NextState<UiState>>| {
                next_ui_state.set(UiState::Leaderboard);
            },
        );
}

fn despawn_level_select(
    mut commands: Commands,
    level_select_ui: Single<Entity, With<LevelSelectUiMarker>>,
) {
    info!("Despawning Level Select!");

    commands.entity(*level_select_ui).despawn();
}

fn ensure_level_preview_image(
    level: &bevy_ecs_ldtk::ldtk::Level,
    level_preview_store: &mut LevelPreviewStore,
    assets: &mut Assets<Image>,
) -> (Vec2, Handle<Image>) {
    let level_id = level
        .get_string_field("LevelId")
        .expect("Levels should always have a level id!");
    if let Some((dims, handle)) = level_preview_store.0.get(level_id) {
        return (*dims, handle.clone());
    }

    let level_layers = level.layer_instances.as_ref().expect("Layers not found!");
    let (layer_w, layer_h, layer_data) = level_layers
        .iter()
        .find_map(|layer| {
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
        .expect("Terrain layer data not found!");
    let level_entities = level_layers
        .iter()
        .find_map(|layer| {
            if layer.identifier == ENTITY_LAYER_IDENT {
                Some(&layer.entity_instances)
            } else {
                None
            }
        })
        .expect("Entity layer data not found!");

    let pixel_size = TextureFormat::bevy_default()
        .pixel_size()
        .expect("Should be 4 (RGBA8)");
    let mut level_preview_data = Vec::with_capacity(layer_w * layer_h * pixel_size);

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
        let entity_color = entity
            .field_instances
            .iter()
            .find_map(|instance| {
                if instance.identifier == SENSOR_COLOR_IDENT {
                    if let FieldValue::Enum(Some(ref color)) = instance.value {
                        Some(color)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .expect("Could not find sensor color field!");
        let rgba = sensor_color_to_rgba(entity_color);
        let idx = (entity_coords.y as usize * layer_w + entity_coords.x as usize) * pixel_size;
        level_preview_data[idx..idx + pixel_size].copy_from_slice(&rgba[..pixel_size]);
    }

    let preview = Image::new(
        Extent3d {
            width: layer_w as u32,
            height: layer_h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        level_preview_data,
        TextureFormat::bevy_default(), // RGBA8 sRGB
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    let handle = assets.add(preview);
    let dims = Vec2::new(layer_w as f32, layer_h as f32);
    level_preview_store
        .0
        .insert(level_id.to_string(), (dims, handle.clone()));
    (dims, handle)
}

// #[allow(dead_code)]
// fn dump_all_level_previews_to_png(
//     mut level_preview_store: ResMut<LevelPreviewStore>,
//     ldtk_assets: Res<Assets<LdtkProject>>,
//     query_ldtk: Query<&LdtkProjectHandle>,
//     res_levels: Res<LevelProgress>,
//     mut assets: ResMut<Assets<Image>>,
// ) {
//     let Ok(ldtk_handle) = query_ldtk.get_single() else {
//         return;
//     };
//     let Ok(ldtk_levels) = get_ldtk_level_data(ldtk_assets.into_inner(), ldtk_handle) else {
//         return;
//     };
//
//     // Create output folder
//     let _ = fs::create_dir_all("level_previews");
//
//     for save in &res_levels.0 {
//         let level = &ldtk_levels[save.level_index];
//
//         let (_dims, handle) =
//             ensure_level_preview_image(level, &mut level_preview_store, &mut assets);
//         if let Some(img_asset) = assets.get(&handle) {
//             let size = img_asset.texture_descriptor.size;
//             let (w, h) = (size.width, size.height);
//             let path = format!("level_previews/{}.png", save.level_id);
//
//             let _ = img::save_buffer_with_format(
//                 Path::new(&path),
//                 &img_asset.data,
//                 w,
//                 h,
//                 img::ColorType::Rgba8,
//                 img::ImageFormat::Png,
//             );
//         }
//     }
// }

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn handle_level_selection(
    mut interaction_query: Query<
        (&Interaction, &LevelSelectButtonIndex),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_game_state: ResMut<NextState<GameState>>,
    ldtk_param: LdtkParam,
    // mut query_player: Query<&mut Transform, (With<PlayerMarker>, Without<StartFlag>)>,
    // mut ev_move_camera: EventWriter<CameraMoveEvent>,
    // mut current_level: ResMut<CurrentLevel>,
    mut level_preview_store: ResMut<LevelPreviewStore>,
    mut assets: ResMut<Assets<Image>>,
    mut level_preview: Single<
        (Entity, Option<(&mut ImageNode, &mut Node)>),
        With<LevelPreviewMarker>,
    >,
    mut level_preview_locked: Single<
        &mut ImageNode,
        (With<LevelPreviewLockedMarker>, Without<LevelPreviewMarker>),
    >,
    mut commands: Commands,
    level_progress: Res<LevelProgress>,
) {
    let Some(project) = ldtk_param.project() else {
        return;
    };
    let ldtk_levels = &project.json_data().levels;

    for (interaction, index) in interaction_query.iter_mut() {
        if index.0 >= ldtk_levels.len() {
            panic!("Selected level index is out of bounds!")
        }
        let level = &ldtk_levels[index.0];
        match *interaction {
            Interaction::Pressed => {
                if level_progress.0[index.1].locked {
                    return;
                }
                // NOTE: inserting the correct resource here ensures lyra spawned in the right
                // position and camera moved to the correct location on level spawn
                commands.insert_resource(LevelSelection::Iid(level.iid.clone().into()));
                next_game_state.set(GameState::InGame);
                break;
            }
            Interaction::Hovered => {
                let (level_dims, level_preview_img) =
                    ensure_level_preview_image(level, &mut level_preview_store, &mut assets);
                let (level_preview_entity, ref mut level_preview_nodes) = *level_preview;
                let locked = level_progress.0[index.1].locked;

                const LOCKED_LEVEL_PREVIEW_SCALE: f32 = 0.3;
                let scaled_color = Color::srgba(
                    LOCKED_LEVEL_PREVIEW_SCALE,
                    LOCKED_LEVEL_PREVIEW_SCALE,
                    LOCKED_LEVEL_PREVIEW_SCALE,
                    1.0,
                );
                if let Some((ref mut level_preview_image_node, ref mut level_preview_node)) =
                    level_preview_nodes
                {
                    level_preview_image_node.image = level_preview_img;
                    if locked {
                        level_preview_image_node.color = scaled_color
                    } else {
                        level_preview_image_node.color = Color::WHITE;
                    }
                    level_preview_node.aspect_ratio = Some(level_dims.x / level_dims.y);
                } else {
                    let mut image_node = ImageNode::new(level_preview_img);
                    if locked {
                        image_node.color = scaled_color;
                    } else {
                        image_node.color = Color::WHITE;
                    }
                    commands.entity(level_preview_entity).insert((
                        image_node,
                        Node {
                            max_width: Val::Percent(100.),
                            height: Val::Percent(100.),
                            aspect_ratio: Some(level_dims.x / level_dims.y),
                            ..default()
                        },
                    ));
                }

                if locked {
                    level_preview_locked.color = Color::WHITE;
                } else {
                    level_preview_locked.color = Color::srgba(1., 1., 1., 0.);
                }
            }
            _ => {}
        }
    }
}
