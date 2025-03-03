use bevy::prelude::*;
use bevy_rapier2d::plugin::RapierContext;
use enum_map::{enum_map, EnumMap};
use itertools::Itertools;

use crate::{
    input::CursorWorldCoords,
    level::CurrentLevel,
    light::{
        segments::{play_light_beam, PrevLightBeamPlayback},
        LightBeamSource, LightColor, LightSourceZMarker,
    },
    lighting::LineLight2d,
};

use super::PlayerMarker;

/// A [`Component`] used to track Lyra's current shooting color as well as the number of beams of
/// that color remaining.
#[derive(Component, Default, Debug)]
pub struct PlayerLightInventory {
    /// set to true when LMB is clicked, set to false when RMB is clicked/LMB is released
    should_shoot: bool,
    pub current_color: LightColor,
    /// Is true if the color is available
    pub sources: EnumMap<LightColor, bool>,
}

impl PlayerLightInventory {
    pub fn colors(colors: &[LightColor]) -> Self {
        PlayerLightInventory {
            should_shoot: false,
            current_color: colors[0],
            sources: enum_map! {
                LightColor::Green =>true,
                LightColor::Blue => true,
                LightColor::Red => true,
                LightColor::White =>true,
            },
        }
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
    let Ok(angle) = q_angle.get_single() else {
        return;
    };

    commands.entity(angle).despawn_recursive();
}

/// [`System`] to handle the keyboard presses corresponding to color switches.
pub fn handle_color_switch(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_inventory: Query<&mut PlayerLightInventory>,
    current_level: Res<CurrentLevel>,
) {
    let Ok(mut inventory) = q_inventory.get_single_mut() else {
        return;
    };

    let binds: [(KeyCode, LightColor); 4] = [
        (KeyCode::Digit1, LightColor::Green),
        (KeyCode::Digit2, LightColor::Red),
        (KeyCode::Digit3, LightColor::White),
        (KeyCode::Digit4, LightColor::Blue),
    ];

    for (key, color) in binds {
        if keys.just_pressed(key) && current_level.allowed_colors.contains(&color) {
            inventory.current_color = color;
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
    if !player_inventory.should_shoot {
        return;
    }
    if !player_inventory.sources[player_inventory.current_color] {
        return;
    }

    let ray_pos = player_transform.translation.truncate();
    let ray_dir = (cursor_pos.pos - ray_pos).normalize_or_zero();

    if ray_dir == Vec2::ZERO {
        return;
    }

    let mut source_transform =
        Transform::from_translation(ray_pos.extend(light_source_z.translation.z));
    source_transform.rotate_z(ray_dir.to_angle());
    let mut source_sprite = Sprite::from_image(asset_server.load("light/compass.png"));
    source_sprite.color = Color::srgb(2.0, 2.0, 2.0);
    let mut outer_source_sprite = Sprite::from_image(asset_server.load("light/compass-gold.png"));
    outer_source_sprite.color = player_inventory
        .current_color
        .light_beam_color()
        .mix(&Color::BLACK, 0.2);

    commands
        .spawn(LightBeamSource {
            start_pos: ray_pos,
            start_dir: ray_dir,
            time_traveled: 0.0,
            color: player_inventory.current_color,
        })
        .insert(PrevLightBeamPlayback::from_color(
            player_inventory.current_color,
        ))
        .insert(LineLight2d::point(
            player_inventory.current_color.lighting_color().extend(1.0),
            30.0,
            0.0,
        ))
        .insert(source_sprite)
        .insert(source_transform)
        .with_child(outer_source_sprite);

    // Bevy's Mut or ResMut doesn't let you borrow multiple fields of a struct, so sometimes you
    // need to "reborrow" it to turn it into &mut. See https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
    let player_inventory = &mut *player_inventory;
    player_inventory.sources[player_inventory.current_color] = false;
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
    if !inventory.should_shoot {
        return;
    }
    if !inventory.sources[inventory.current_color] {
        return;
    }

    let ray_pos = transform.translation.truncate();
    let ray_dir = (cursor_pos.pos - ray_pos).normalize_or_zero();

    let dummy_source = LightBeamSource {
        start_pos: ray_pos,
        start_dir: ray_dir,
        time_traveled: 10000.0, // LOL
        color: inventory.current_color,
    };
    let playback = play_light_beam(rapier_context.into_inner(), &dummy_source);

    for (a, b) in playback.iter_points(&dummy_source).tuple_windows() {
        gizmos.line_2d(a, b, inventory.current_color.light_beam_color().darker(0.3));
    }
}
