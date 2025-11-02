use std::f32::consts::PI;

use avian2d::prelude::SpatialQuery;
use bevy::{input::mouse::MouseWheel, prelude::*};
use enum_map::{enum_map, EnumMap};
use itertools::Itertools;

use crate::{
    asset::LoadResource,
    camera::HIGHRES_LAYER,
    game::{
        cursor::CursorWorldCoords,
        defs::{mirror::Mirror, shard::CrystalShardMods},
        light::{
            segments::{play_light_beam, PrevLightBeamPlayback},
            LightBeamSource, LightColor,
        },
        lighting::LineLight2d,
        lyra::Lyra,
        LevelSystems,
    },
    ldtk::{LdtkLevelParam, LevelExt},
    shared::ResetLevels,
};

const NUM_INCREMENTS: i32 = 16; // The number of angle increments for light beam alignment

pub struct BeamControllerPlugin;

impl Plugin for BeamControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BeamSourceAssets>();
        app.load_resource::<BeamSourceAssets>();
        app.add_message::<BeamAction>();
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
    ldtk_level_param: LdtkLevelParam,
) {
    let allowed_cols = ldtk_level_param
        .cur_level()
        .expect("Cur level should exist")
        .raw()
        .allowed_colors();
    let old_color = inventory.current_color;

    **inventory = PlayerLightInventory::new();

    // if the new level has the current color as an allowed color, preserve it
    if let Some(color) = old_color {
        if allowed_cols[color] {
            inventory.current_color = old_color;
        }
    }
    // if select none, make sure to select some
    if old_color.is_none() && allowed_cols[LightColor::Green] {
        inventory.current_color = Some(LightColor::Green);
    }
}

#[derive(Message)]
pub enum BeamAction {
    SwitchColor(Option<LightColor>),
    Preview,
    Snap(bool),
    Cancel,
    Shoot,
}

#[derive(Component, Default, Debug)]
pub struct PlayerLightInventory {
    pub previewing: bool,
    pub snapping: bool,
    pub should_shoot: bool,
    pub current_color: Option<LightColor>,
    /// Is true if the color is available
    pub sources: EnumMap<LightColor, bool>,
}

impl PlayerLightInventory {
    pub fn new() -> Self {
        PlayerLightInventory {
            previewing: false,
            snapping: false,
            should_shoot: false,
            current_color: None,
            sources: enum_map! {
                LightColor::Green =>true,
                LightColor::Blue => true,
                LightColor::Purple => true,
                LightColor::White =>true,
                // LightColor::Black => true,
            },
        }
    }

    pub fn can_shoot(&self) -> bool {
        self.should_shoot && self.current_color.is_some_and(|color| self.sources[color])
    }
}

/// [`System`] to handle the keyboard presses corresponding to color switches.
pub fn handle_color_switch(
    keys: Res<ButtonInput<KeyCode>>,
    mut ev_scroll: MessageReader<MouseWheel>,
    mut beam_actions: MessageWriter<BeamAction>,
    inventory: Single<&PlayerLightInventory, With<Lyra>>,
    shard_modifs: Res<CrystalShardMods>,
    ldtk_level_param: LdtkLevelParam,
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

    let mut allowed_colors = ldtk_level_param
        .cur_level()
        .expect("Cur level should exist")
        .raw()
        .allowed_colors();

    for (color, allowed) in shard_modifs.0.iter() {
        if *allowed {
            allowed_colors[color] = true;
        }
    }

    for scroll in ev_scroll.read() {
        let sign = -(scroll.y.signum() as i32);
        let mut new_index = cur_index + sign;

        // suspicious algorithm to cycle through available colors with the scroll wheel
        // basically skips disallowed colors until you find the next one
        let mut count = 0;
        while !allowed_colors[COLOR_BINDS[new_index.rem_euclid(4) as usize].1]
            && count < COLOR_BINDS.len()
        {
            new_index += sign;
            count += 1;
        }
        cur_index = new_index;
        if allowed_colors[COLOR_BINDS[new_index.rem_euclid(4) as usize].1] {
            beam_actions.write(BeamAction::SwitchColor(Some(
                COLOR_BINDS[cur_index.rem_euclid(4) as usize].1,
            )));
        }
    }

    for (key, color) in COLOR_BINDS {
        if keys.just_pressed(key) && allowed_colors[color] {
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
}

#[derive(Resource, Reflect, Asset, Clone)]
#[reflect(Resource)]
pub struct BeamSourceAssets {
    #[dependency]
    compass: Handle<Image>,
    #[dependency]
    compass_gold: Handle<Image>,
}

impl FromWorld for BeamSourceAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            compass: asset_server.load("light/compass.png"),
            compass_gold: asset_server.load("light/compass-gold.png"),
        }
    }
}

pub fn process_beam_actions(
    mut commands: Commands,
    mut beam_actions: MessageReader<BeamAction>,
    lyra: Single<(&Transform, &mut PlayerLightInventory), With<Lyra>>,
    cursor: Single<&CursorWorldCoords>,
    beam_assets: Res<BeamSourceAssets>,
) {
    let (player_transform, mut player_inventory) = lyra.into_inner();
    for action in beam_actions.read() {
        match action {
            BeamAction::Shoot => {
                if !player_inventory.can_shoot() {
                    continue;
                }

                let ray_pos = player_transform.translation.truncate();
                let mut ray_dir = (cursor.pos - ray_pos).normalize_or_zero();
                if player_inventory.snapping {
                    ray_dir = snap_ray(ray_dir);
                }
                if ray_dir == Vec2::ZERO {
                    continue;
                }
                let ray_dir = Dir2::new_unchecked(ray_dir);

                let shoot_color = player_inventory.current_color.unwrap();

                // NOTE: hardcode here should be okay
                let mut source_transform = Transform::from_translation(ray_pos.extend(3.));
                source_transform.rotate_z(ray_dir.to_angle());
                let mut source_sprite = Sprite::from_image(beam_assets.compass.clone());
                source_sprite.color = Color::srgb(2.0, 2.0, 2.0);
                let mut outer_source_sprite = Sprite::from_image(beam_assets.compass_gold.clone());
                outer_source_sprite.color = shoot_color.light_beam_color().mix(&Color::BLACK, 0.4);

                commands
                    .spawn(LightBeamSource {
                        start_pos: ray_pos,
                        start_dir: ray_dir,
                        time_traveled: 0.0,
                        color: shoot_color,
                    })
                    .insert(PrevLightBeamPlayback::default())
                    .insert(HIGHRES_LAYER)
                    .insert(source_sprite)
                    .insert(source_transform)
                    .with_child((outer_source_sprite, HIGHRES_LAYER))
                    .with_child(LineLight2d::point(
                        shoot_color.lighting_color().extend(1.0),
                        30.0,
                        0.02,
                    ));

                player_inventory.sources[shoot_color] = false;
                player_inventory.should_shoot = false;
                player_inventory.previewing = false;
            }
            BeamAction::Snap(val) => {
                player_inventory.snapping = *val;
            }
            BeamAction::Preview => {
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
        }
    }
}

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct LightPreviewGizmos;

pub fn preview_light_path(
    spatial_query: SpatialQuery,
    lyra: Single<(&Transform, &PlayerLightInventory), With<Lyra>>,
    cursor: Single<&CursorWorldCoords>,
    mut gizmos: Gizmos,
    q_mirror: Query<&Mirror>,
    // q_black_ray: Query<(Entity, &BlackRayComponent)>,
) {
    let (transform, inventory) = lyra.into_inner();
    if !inventory.can_shoot() || !inventory.previewing {
        return;
    }

    let shoot_color = inventory.current_color.unwrap();

    let ray_pos = transform.translation.truncate();
    let mut ray_dir = (cursor.pos - ray_pos).normalize_or_zero();
    if inventory.snapping {
        ray_dir = snap_ray(ray_dir);
    }
    if ray_dir == Vec2::ZERO {
        return;
    }
    let ray_dir = Dir2::new_unchecked(ray_dir);

    let dummy_source = LightBeamSource {
        start_pos: ray_pos,
        start_dir: ray_dir,
        time_traveled: 10000.0, // LOL
        color: shoot_color,
    };
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
