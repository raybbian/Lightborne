use bevy::{
    input::{
        common_conditions::{input_just_pressed, input_just_released},
        mouse::MouseWheel,
    },
    prelude::*,
};
use bevy_rapier2d::plugin::RapierContext;
use enum_map::{enum_map, EnumMap};
use itertools::Itertools;
use ui::LightUiPlugin;

use bevy::prelude::ops::{cos, sin};
use std::f32::consts::PI;

use crate::{
    input::{update_cursor_world_coords, CursorWorldCoords},
    level::{CurrentLevel, LevelSystems},
    light::{
        segments::{play_light_beam, PrevLightBeamPlayback},
        LightBeamSource, LightColor, LightSourceZMarker,
    },
    lighting::LineLight2d,
};
use indicator::LightIndicatorPlugin;

mod indicator;
mod ui;

const NUMINCREMENTS: i32 = 16;

use super::{not_input_locked, PlayerMarker};

pub struct PlayerLightPlugin;

impl Plugin for PlayerLightPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LightIndicatorPlugin)
            .add_plugins(LightUiPlugin)
            .add_systems(
                Update,
                (
                    handle_color_switch,
                    should_shoot_light::<true>.run_if(input_just_pressed(MouseButton::Left)),
                    should_shoot_light::<false>.run_if(input_just_pressed(MouseButton::Right)),
                    preview_light_path,
                    spawn_angle_indicator.run_if(input_just_pressed(MouseButton::Left)),
                    despawn_angle_indicator.run_if(
                        input_just_released(MouseButton::Left)
                            .or(input_just_pressed(MouseButton::Right)),
                    ),
                    shoot_light.run_if(input_just_released(MouseButton::Left)),
                )
                    .chain()
                    .run_if(not_input_locked)
                    .in_set(LevelSystems::Simulation)
                    .after(update_cursor_world_coords),
            );
    }
}

/// A [`Component`] used to track Lyra's current shooting color as well as the number of beams of
/// that color remaining.
#[derive(Component, Default, Debug)]
pub struct PlayerLightInventory {
    /// set to true when LMB is clicked, set to false when RMB is clicked/LMB is released
    should_shoot: bool,
    pub current_color: Option<LightColor>,
    /// Is true if the color is available
    pub sources: EnumMap<LightColor, bool>,
}

impl PlayerLightInventory {
    pub fn new() -> Self {
        PlayerLightInventory {
            should_shoot: false,
            current_color: None,
            sources: enum_map! {
                LightColor::Green =>true,
                LightColor::Blue => true,
                LightColor::Purple => true,
                LightColor::White =>true,
            },
        }
    }

    pub fn can_shoot(&self) -> bool {
        self.should_shoot && self.current_color.is_some_and(|color| self.sources[color])
    }
}

#[derive(Component)]
pub struct AngleMarker;

pub fn spawn_angle_indicator(
    mut commands: Commands,
    q_player: Query<Entity, With<PlayerMarker>>,
    asset_server: Res<AssetServer>,
) {
    let Ok(player) = q_player.get_single() else {
        return;
    };

    commands.entity(player).with_child((
        Sprite {
            image: asset_server.load("angle.png"),
            color: Color::srgba(1.0, 1.0, 1.0, 0.1),
            ..default()
        },
        AngleMarker,
    ));
}

pub fn despawn_angle_indicator(mut commands: Commands, q_angle: Query<Entity, With<AngleMarker>>) {
    for angle in q_angle.iter() {
        commands.entity(angle).despawn_recursive();
    }
}

/// [`System`] to handle the keyboard presses corresponding to color switches.
pub fn handle_color_switch(
    keys: Res<ButtonInput<KeyCode>>,
    mut ev_scroll: EventReader<MouseWheel>,
    mut q_inventory: Query<&mut PlayerLightInventory, With<PlayerMarker>>,
    current_level: Res<CurrentLevel>,
) {
    let Ok(mut inventory) = q_inventory.get_single_mut() else {
        return;
    };

    static COLOR_BINDS: [(KeyCode, LightColor); 4] = [
        (KeyCode::Digit1, LightColor::Green),
        (KeyCode::Digit2, LightColor::Purple),
        (KeyCode::Digit3, LightColor::White),
        (KeyCode::Digit4, LightColor::Blue),
    ];

    let mut cur_index = match inventory.current_color {
        None => -1,
        Some(LightColor::Green) => 0,
        Some(LightColor::Purple) => 1,
        Some(LightColor::White) => 2,
        Some(LightColor::Blue) => 3,
    };

    for scroll in ev_scroll.read() {
        let sign = -(scroll.y.signum() as i32);
        let mut new_index = cur_index + sign;

        // suspicious algorithm to cycle through available colors with the scroll wheel
        // basically skips disallowed colors until you find the next one
        let mut count = 0;
        while !current_level.allowed_colors[COLOR_BINDS[new_index.rem_euclid(4) as usize].1]
            && count < COLOR_BINDS.len()
        {
            new_index += sign;
            count += 1;
        }
        cur_index = new_index;
        if current_level.allowed_colors[COLOR_BINDS[new_index.rem_euclid(4) as usize].1] {
            inventory.current_color = Some(COLOR_BINDS[cur_index.rem_euclid(4) as usize].1);
        }
    }

    for (key, color) in COLOR_BINDS {
        if keys.just_pressed(key) && current_level.allowed_colors[color] {
            inventory.current_color = Some(color);
        }
    }
}

pub fn should_shoot_light<const V: bool>(
    mut q_player: Query<&mut PlayerLightInventory, With<PlayerMarker>>,
) {
    let Ok(mut inventory) = q_player.get_single_mut() else {
        return;
    };
    inventory.should_shoot = V;
}

pub fn shoot_light(
    mut commands: Commands,
    mut q_player: Query<(&Transform, &mut PlayerLightInventory), With<PlayerMarker>>,
    q_light_source_z: Query<&Transform, With<LightSourceZMarker>>,
    q_cursor: Query<&CursorWorldCoords>,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
) {
    let Ok((player_transform, mut player_inventory)) = q_player.get_single_mut() else {
        return;
    };
    let Ok(light_source_z) = q_light_source_z.get_single() else {
        return;
    };
    let Ok(cursor_pos) = q_cursor.get_single() else {
        return;
    };
    if !player_inventory.can_shoot() {
        return;
    }

    let ray_pos = player_transform.translation.truncate();
    let mut ray_dir = (cursor_pos.pos - ray_pos).normalize_or_zero();

    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        ray_dir = snap_ray(ray_dir);
    }

    if ray_dir == Vec2::ZERO {
        return;
    }

    let shoot_color = player_inventory.current_color.unwrap();

    let mut source_transform =
        Transform::from_translation(ray_pos.extend(light_source_z.translation.z));
    source_transform.rotate_z(ray_dir.to_angle());
    let mut source_sprite = Sprite::from_image(asset_server.load("light/compass.png"));
    source_sprite.color = Color::srgb(2.0, 2.0, 2.0);
    let mut outer_source_sprite = Sprite::from_image(asset_server.load("light/compass-gold.png"));
    outer_source_sprite.color = shoot_color.light_beam_color().mix(&Color::BLACK, 0.4);

    commands
        .spawn(LightBeamSource {
            start_pos: ray_pos,
            start_dir: ray_dir,
            time_traveled: 0.0,
            color: shoot_color,
        })
        .insert(PrevLightBeamPlayback::from_color(shoot_color))
        .insert(LineLight2d::point(
            shoot_color.lighting_color().extend(1.0),
            30.0,
            0.0,
        ))
        .insert(source_sprite)
        .insert(source_transform)
        .with_child(outer_source_sprite);

    // Bevy's Mut or ResMut doesn't let you borrow multiple fields of a struct, so sometimes you
    // need to "reborrow" it to turn it into &mut. See https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
    let player_inventory = &mut *player_inventory;
    player_inventory.sources[shoot_color] = false;
    player_inventory.should_shoot = false;
}

/// [`System`] that uses [`Gizmos`] to preview the light path while the left mouse button is held
/// down. This system needs some work, namely:
///
/// - Not using [`Gizmos`] to render the light segments
pub fn preview_light_path(
    mut q_rapier: Query<&mut RapierContext>,
    q_player: Query<(&Transform, &PlayerLightInventory), With<PlayerMarker>>,
    q_cursor: Query<&CursorWorldCoords>,
    keys: Res<ButtonInput<KeyCode>>,
    mut gizmos: Gizmos,
) {
    let Ok(rapier_context) = q_rapier.get_single_mut() else {
        return;
    };
    let Ok((transform, inventory)) = q_player.get_single() else {
        return;
    };
    let Ok(cursor_pos) = q_cursor.get_single() else {
        return;
    };
    if !inventory.can_shoot() {
        return;
    }

    let shoot_color = inventory.current_color.unwrap();

    let ray_pos = transform.translation.truncate();
    let mut ray_dir = (cursor_pos.pos - ray_pos).normalize_or_zero();

    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        ray_dir = snap_ray(ray_dir);
    }

    let dummy_source = LightBeamSource {
        start_pos: ray_pos,
        start_dir: ray_dir,
        time_traveled: 10000.0, // LOL
        color: shoot_color,
    };
    let playback = play_light_beam(rapier_context.into_inner(), &dummy_source);

    for (a, b) in playback.iter_points(&dummy_source).tuple_windows() {
        gizmos.line_2d(a, b, shoot_color.light_beam_color().darker(0.3));
    }
}

fn snap_ray(ray_vec: Vec2) -> Vec2 {
    let ray_angle = (ray_vec.y.atan2(ray_vec.x) + (2.0 * PI)) % (2.0 * PI);
    let increment_angle = (2.0 * PI) / NUMINCREMENTS as f32;
    let snapped_angle = (ray_angle / increment_angle).round() * increment_angle;
    let new_ray_vec = Vec2::new(cos(snapped_angle), sin(snapped_angle));
    return new_ray_vec;
}