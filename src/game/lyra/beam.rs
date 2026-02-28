use std::{collections::HashSet, f32::consts::PI};

use avian2d::prelude::*;
use bevy::{input::mouse::MouseWheel, prelude::*};
use enum_map::{enum_map, EnumMap};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    // asset::LoadResource,
    camera::HIGHRES_LAYER,
    config::Config,
    game::{
        cursor::CursorWorldCoords,
        defs::mirror::Mirror,
        light::{
            segments::{play_light_beam, LightBeamSourceDespawn, PrevLightBeamPlayback},
            LightBeamSource, LightColor,
        },
        lighting::LineLight2d,
        lyra::{
            indicator::{LYRA_SHARD_HOLD_X, LYRA_SHARD_HOLD_Y},
            Lyra,
        },
        Layers, LevelSystems,
    },
    save::SaveParam,
    shared::{ResetLevels, UiState},
};

const NUM_INCREMENTS: i32 = 16; // The number of angle increments for light beam alignment

pub struct BeamControllerPlugin;

impl Plugin for BeamControllerPlugin {
    fn build(&self, app: &mut App) {
        // app.register_type::<BeamSourceAssets>();
        // app.load_resource::<BeamSourceAssets>();
        app.init_resource::<PlayerLightProgress>();
        app.add_message::<BeamAction>();
        app.add_systems(OnExit(UiState::Leaderboard), init_player_light_save_data);
        app.add_systems(
            Update,
            (handle_color_switch, handle_shoot_inputs, preview_light_path)
                .chain()
                .in_set(LevelSystems::Input),
        );
        app.add_systems(
            Update,
            process_beam_actions
                .after(preview_light_path)
                .in_set(LevelSystems::Simulation),
        );
        app.add_observer(reset_light_inventory);
    }
}

pub fn reset_light_inventory(
    _: On<ResetLevels>,
    mut inventory: Single<&mut PlayerLightInventory, With<Lyra>>,
) {
    for (_, source) in inventory.collectible.iter_mut() {
        *source = None;
    }
}

#[derive(Message)]
pub enum BeamAction {
    SwitchColor(Option<LightColor>),
    Preview,
    Snap(bool),
    Cancel,
    Shoot,
    Collect,
}

#[derive(Resource, Default, Serialize, Deserialize, Clone)]
pub struct PlayerLightProgress {
    pub unlocked: HashSet<LightColor>,
}

pub fn init_player_light_save_data(
    mut light_save_data: ResMut<PlayerLightProgress>,
    config: Res<Config>,
    save_param: SaveParam,
) {
    if let Some(save_data) = save_param.get_save_data() {
        *light_save_data = save_data.light.clone();
        info!(
            "Found save data with light colors {:?}",
            save_data.light.unlocked
        )
    } else {
        *light_save_data = PlayerLightProgress {
            unlocked: HashSet::new(),
        }
    }

    if config.debug_config.unlock_beams {
        light_save_data.unlocked.insert(LightColor::White);
        light_save_data.unlocked.insert(LightColor::Green);
        light_save_data.unlocked.insert(LightColor::Purple);
        light_save_data.unlocked.insert(LightColor::Blue);
    }
}

pub fn on_collide_beam_source(
    event: On<CollisionStart>,
    mut inventory: Single<&mut PlayerLightInventory, With<Lyra>>,
    q_beam_source: Query<&LightBeamSource>,
) {
    let Ok(beam_source) = q_beam_source.get(event.collider2) else {
        return;
    };
    inventory.collectible[beam_source.color]
        .as_mut()
        .unwrap()
        .in_reach = true;
}

pub fn on_leave_beam_source(
    event: On<CollisionEnd>,
    mut inventory: Single<&mut PlayerLightInventory, With<Lyra>>,
    q_beam_source: Query<&LightBeamSource>,
) {
    let Ok(beam_source) = q_beam_source.get(event.collider2) else {
        return;
    };
    inventory.collectible[beam_source.color]
        .as_mut()
        .unwrap()
        .in_reach = false;
}

#[derive(Debug)]
pub struct LightInventorySource {
    in_reach: bool,
    entity: Entity,
}

#[derive(Component, Default, Debug)]
pub struct PlayerLightInventory {
    pub previewing: bool,
    pub snapping: bool,
    pub should_shoot: bool,
    pub current_color: Option<LightColor>,
    /// Is true if the color is unlocked
    pub allowed: EnumMap<LightColor, bool>,
    pub collectible: EnumMap<LightColor, Option<LightInventorySource>>,
}

impl PlayerLightInventory {
    pub fn new() -> Self {
        PlayerLightInventory {
            previewing: false,
            snapping: false,
            should_shoot: false,
            current_color: None,
            allowed: enum_map! {
                _ => false,
            },
            collectible: enum_map! {
                _ => None,
            },
        }
    }

    pub fn can_shoot(&self) -> bool {
        self.should_shoot
            && self
                .current_color
                .is_some_and(|color| self.can_shoot_color(color))
    }

    pub fn can_shoot_color(&self, color: LightColor) -> bool {
        self.allowed[color] && self.collectible[color].is_none()
    }
}

impl From<&PlayerLightProgress> for PlayerLightInventory {
    fn from(value: &PlayerLightProgress) -> Self {
        let mut inventory = PlayerLightInventory::new();
        for (color, allowed) in inventory.allowed.iter_mut() {
            *allowed = value.unlocked.contains(&color);
        }
        inventory
    }
}

/// [`System`] to handle the keyboard presses corresponding to color switches.
pub fn handle_color_switch(
    keys: Res<ButtonInput<KeyCode>>,
    mut ev_scroll: MessageReader<MouseWheel>,
    mut beam_actions: MessageWriter<BeamAction>,
    inventory: Single<&PlayerLightInventory, With<Lyra>>,
) {
    static COLOR_BINDS: [(KeyCode, LightColor); 4] = [
        (KeyCode::Digit1, LightColor::Green),
        (KeyCode::Digit2, LightColor::Purple),
        (KeyCode::Digit3, LightColor::White),
        (KeyCode::Digit4, LightColor::Blue),
        // (KeyCode::Digit5, LightColor::Black),
    ];

    let mut cur_index = match inventory.current_color {
        None => -1,
        Some(LightColor::Green) => 0,
        Some(LightColor::Purple) => 1,
        Some(LightColor::White) => 2,
        Some(LightColor::Blue) => 3,
        // Some(LightColor::Black) => 4,
    };

    for scroll in ev_scroll.read() {
        let sign = -(scroll.y.signum() as i32);
        let mut new_index = cur_index + sign;

        // suspicious algorithm to cycle through available colors with the scroll wheel
        // basically skips disallowed colors until you find the next one
        let mut count = 0;
        while !inventory.allowed[COLOR_BINDS[new_index.rem_euclid(4) as usize].1]
            && count < COLOR_BINDS.len()
        {
            new_index += sign;
            count += 1;
        }
        cur_index = new_index;
        if inventory.allowed[COLOR_BINDS[new_index.rem_euclid(4) as usize].1] {
            beam_actions.write(BeamAction::SwitchColor(Some(
                COLOR_BINDS[cur_index.rem_euclid(4) as usize].1,
            )));
        }
    }

    for (key, color) in COLOR_BINDS {
        if keys.just_pressed(key) && inventory.allowed[color] {
            beam_actions.write(BeamAction::SwitchColor(Some(color)));
        }
    }
}

pub fn handle_shoot_inputs(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut beam_actions: MessageWriter<BeamAction>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        beam_actions.write(BeamAction::Preview);
    }
    if mouse.just_pressed(MouseButton::Right) {
        beam_actions.write(BeamAction::Cancel);
    }
    if mouse.just_released(MouseButton::Left) {
        beam_actions.write(BeamAction::Shoot);
    }
    if keys.just_pressed(KeyCode::ShiftLeft) {
        beam_actions.write(BeamAction::Snap(true));
    }
    if keys.just_released(KeyCode::ShiftLeft) {
        beam_actions.write(BeamAction::Snap(false));
    }
    if keys.just_pressed(KeyCode::KeyE) {
        beam_actions.write(BeamAction::Collect);
    }
}

// #[derive(Resource, Reflect, Asset, Clone)]
// #[reflect(Resource)]
// pub struct BeamSourceAssets {
//     #[dependency]
//     compass: Handle<Image>,
//     #[dependency]
//     compass_gold: Handle<Image>,
// }
//
// impl FromWorld for BeamSourceAssets {
//     fn from_world(world: &mut World) -> Self {
//         let asset_server = world.resource::<AssetServer>();
//
//         Self {
//             compass: asset_server.load("light/compass.png"),
//             compass_gold: asset_server.load("light/compass-gold.png"),
//         }
//     }
// }

pub fn process_beam_actions(
    mut commands: Commands,
    mut beam_actions: MessageReader<BeamAction>,
    lyra: Single<(&Transform, &mut PlayerLightInventory, &Sprite), With<Lyra>>,
    cursor: Single<&CursorWorldCoords>,
    // beam_assets: Res<BeamSourceAssets>,
) {
    let (player_transform, player_inventory, lyra_sprite) = lyra.into_inner();
    let player_inventory = player_inventory.into_inner();
    for action in beam_actions.read() {
        match action {
            BeamAction::Shoot => {
                if !player_inventory.can_shoot() {
                    continue;
                }
                let mult = if lyra_sprite.flip_x { -1. } else { 1. };
                let desired_shard_pos = Vec3::new(mult * LYRA_SHARD_HOLD_X, LYRA_SHARD_HOLD_Y, 0.);

                let ray_pos = (player_transform.translation + desired_shard_pos).truncate();
                let mut ray_dir = (cursor.pos - ray_pos).normalize_or_zero();
                if player_inventory.snapping {
                    ray_dir = snap_ray(ray_dir);
                }
                if ray_dir == Vec2::ZERO {
                    continue;
                }
                let ray_dir = Dir2::new_unchecked(ray_dir);

                let shoot_color = player_inventory.current_color.unwrap();

                let mut source_transform = Transform::from_translation(
                    ray_pos.extend(player_transform.translation.z - 0.2),
                );
                source_transform.rotate_z(ray_dir.to_angle());
                // let mut source_sprite = Sprite::from_image(beam_assets.compass.clone());
                // source_sprite.color = Color::srgb(2.0, 2.0, 2.0);
                // let mut outer_source_sprite = Sprite::from_image(beam_assets.compass_gold.clone());
                // outer_source_sprite.color = shoot_color.light_beam_color().mix(&Color::BLACK, 0.4);

                let source = commands
                    .spawn(LightBeamSource::new(ray_pos, ray_dir, shoot_color))
                    .insert(Collider::rectangle(12., 12.))
                    .insert(Sensor)
                    .insert(CollisionLayers::new(
                        Layers::SensorBox,
                        [Layers::PlayerHurtbox],
                    ))
                    .insert(PrevLightBeamPlayback::default())
                    .insert(HIGHRES_LAYER)
                    .insert(source_transform)
                    // .with_child((outer_source_sprite, HIGHRES_LAYER))
                    .with_child(LineLight2d::point(
                        shoot_color.lighting_color().extend(1.0),
                        30.0,
                        0.02,
                    ))
                    .id();

                player_inventory.collectible[shoot_color] = Some(LightInventorySource {
                    in_reach: false,
                    entity: source,
                });
                player_inventory.should_shoot = false;
                player_inventory.previewing = false;
            }
            BeamAction::Snap(val) => {
                player_inventory.snapping = *val;
            }
            BeamAction::Preview => {
                if player_inventory.current_color == None
                    || !player_inventory.can_shoot_color(player_inventory.current_color.unwrap())
                {
                    continue;
                }
                player_inventory.previewing = true;
                player_inventory.should_shoot = true;
            }
            BeamAction::Cancel => {
                player_inventory.previewing = false;
                player_inventory.should_shoot = false;
            }
            BeamAction::SwitchColor(color) => {
                player_inventory.current_color = *color;
            }
            BeamAction::Collect => {
                for (_, source) in player_inventory.collectible.iter_mut() {
                    if let Some(s) = source {
                        if s.in_reach {
                            commands.entity(s.entity).insert(LightBeamSourceDespawn);
                            // set source to none once actually despawned
                        }
                    }
                }
            }
        }
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct LightPreviewGizmos;

pub fn preview_light_path(
    spatial_query: SpatialQuery,
    lyra: Single<(&Transform, &PlayerLightInventory, &Sprite), With<Lyra>>,
    cursor: Single<&CursorWorldCoords>,
    mut gizmos: Gizmos,
    q_mirror: Query<&Mirror>,
    // q_black_ray: Query<(Entity, &BlackRayComponent)>,
) {
    let (transform, inventory, lyra_sprite) = lyra.into_inner();
    if !inventory.can_shoot() || !inventory.previewing {
        return;
    }

    let shoot_color = inventory.current_color.unwrap();

    let mult = if lyra_sprite.flip_x { -1. } else { 1. };
    let desired_shard_pos = Vec3::new(mult * LYRA_SHARD_HOLD_X, LYRA_SHARD_HOLD_Y, 0.);

    let ray_pos = (transform.translation + desired_shard_pos).truncate();
    let mut ray_dir = (cursor.pos - ray_pos).normalize_or_zero();

    if inventory.snapping {
        ray_dir = snap_ray(ray_dir);
    }
    if ray_dir == Vec2::ZERO {
        return;
    }
    let ray_dir = Dir2::new_unchecked(ray_dir);

    let dummy_source =
        LightBeamSource::new(ray_pos, ray_dir, shoot_color).with_time_traveled(10000.);

    let playback = play_light_beam(
        &spatial_query,
        &dummy_source,
        // &q_black_ray,
        &q_mirror,
    );

    for (a, b) in playback.iter_points(&dummy_source).tuple_windows() {
        gizmos.line_2d(a, b, shoot_color.light_beam_color().darker(0.3));
    }
}

fn snap_ray(ray_vec: Vec2) -> Vec2 {
    let ray_angle = (ray_vec.y.atan2(ray_vec.x) + (2.0 * PI)) % (2.0 * PI);
    let increment_angle = (2.0 * PI) / NUM_INCREMENTS as f32;
    let snapped_angle = (ray_angle / increment_angle).round() * increment_angle;
    Vec2::from_angle(snapped_angle)
}
