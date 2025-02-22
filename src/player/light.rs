use bevy::prelude::*;
use bevy_rapier2d::{math::Real, plugin::RapierContext, prelude::CollisionGroups, prelude::*};
use enum_map::{enum_map, EnumMap};

use crate::{
    input::CursorWorldCoords,
    light::{LightColor, LightRaySource},
    shared::GroupLabel,
};

use super::PlayerMarker;

/// A [`Component`] used to track Lyra's current shooting color as well as the number of beams of
/// that color remaining.
#[derive(Component, Default, Debug)]
pub struct PlayerLightInventory {
    current_color: LightColor,
    /// Is true if the color is available
    sources: EnumMap<LightColor, bool>,
}

impl PlayerLightInventory {
    pub fn colors(colors: &Vec<LightColor>) -> Self {
        PlayerLightInventory {
            current_color: colors[0],
            sources: enum_map! {
                LightColor::Green => colors.contains(&LightColor::Green),
                LightColor::Blue => colors.contains(&LightColor::Blue),
                LightColor::Red => colors.contains(&LightColor::Red),
                LightColor::White => colors.contains(&LightColor::White),
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
) {
    let Ok(mut inventory) = q_inventory.get_single_mut() else {
        return;
    };
    if keys.just_pressed(KeyCode::Digit1) {
        inventory.current_color = LightColor::Green;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        inventory.current_color = LightColor::Red;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        inventory.current_color = LightColor::White;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        inventory.current_color = LightColor::Blue;
    }
}

/// [`System`] that spawns a [`LightRaySource`] when the player releases the left mouse button.
/// This system should instead consider sending a `LightRaySpawnEvent` with the needed information
/// to keep all light-related systems in the [`light`](crate::light) module.
pub fn shoot_light(
    mut commands: Commands,
    mut q_player: Query<(&Transform, &mut PlayerLightInventory), With<PlayerMarker>>,
    q_cursor: Query<&CursorWorldCoords>,
    asset_server: Res<AssetServer>,
) {
    let Ok((player_transform, mut player_inventory)) = q_player.get_single_mut() else {
        return;
    };
    let Ok(cursor_pos) = q_cursor.get_single() else {
        return;
    };
    if !player_inventory.sources[player_inventory.current_color] {
        return;
    }

    let ray_pos = player_transform.translation.truncate();
    let ray_dir = (cursor_pos.pos - ray_pos).normalize_or_zero();

    if ray_dir == Vec2::ZERO {
        return;
    }

    let mut source_transform = Transform::from_translation(ray_pos.extend(1.0)); //hardcode hack fix
    source_transform.rotate_z(ray_dir.to_angle());
    let mut source_sprite = Sprite::from_image(asset_server.load("light/compass.png"));
    source_sprite.color = Color::srgb(2.0, 2.0, 2.0);
    let mut outer_source_sprite = Sprite::from_image(asset_server.load("light/compass-gold.png"));
    outer_source_sprite.color = player_inventory
        .current_color
        .light_beam_color()
        .mix(&Color::BLACK, 0.2);

    commands
        .spawn(LightRaySource {
            start_pos: ray_pos,
            start_dir: ray_dir,
            time_traveled: 0.0,
            num_bounces: 0,
            color: player_inventory.current_color,
        })
        .insert(source_sprite)
        .insert(source_transform)
        .with_child(outer_source_sprite);

    // Bevy's Mut or ResMut doesn't let you borrow multiple fields of a struct, so sometimes you
    // need to "reborrow" it to turn it into &mut. See https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
    let player_inventory = &mut *player_inventory;
    player_inventory.sources[player_inventory.current_color] = false;
}

/// [`System`] that uses [`Gizmos`] to preview the light path while the left mouse button is held
/// down. This system needs some work, namely:
///
/// - Not using [`Gizmos`] to render the light segments
/// - Not copying the same code logic as
///    [`simulate_light_sources`](crate::light::segments::simulate_light_sources).
pub fn preview_light_path(
    mut q_rapier: Query<&mut RapierContext>,
    q_player: Query<(&Transform, &PlayerLightInventory), With<PlayerMarker>>,
    q_cursor: Query<&CursorWorldCoords>,
    mut gizmos: Gizmos,
) {
    // FIXME: duplicate code with some of light module, should be made common function
    let Ok(rapier_context) = q_rapier.get_single_mut() else {
        return;
    };
    let Ok((transform, inventory)) = q_player.get_single() else {
        return;
    };
    let Ok(cursor_pos) = q_cursor.get_single() else {
        return;
    };
    if !inventory.sources[inventory.current_color] {
        return;
    }

    let mut ray_pos = transform.translation.truncate();
    let mut ray_dir = (cursor_pos.pos - ray_pos).normalize_or_zero();

    if ray_dir == Vec2::ZERO {
        return;
    }

    let collision_groups = match inventory.current_color {
        LightColor::White => CollisionGroups::new(
            GroupLabel::WHITE_RAY,
            GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR,
        ),
        LightColor::Blue => CollisionGroups::new(
            GroupLabel::BLUE_RAY,
            GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR | GroupLabel::WHITE_RAY,
        ),
        _ => CollisionGroups::new(
            GroupLabel::LIGHT_RAY,
            GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR | GroupLabel::WHITE_RAY,
        ),
    };

    let mut ray_qry = QueryFilter::new().groups(collision_groups);

    for _ in 0..inventory.current_color.num_bounces() + 1 {
        let Some((entity, intersection)) =
            rapier_context.cast_ray_and_get_normal(ray_pos, ray_dir, Real::MAX, true, ray_qry)
        else {
            let final_point = ray_pos + ray_dir * Real::MAX;
            gizmos.line_2d(
                ray_pos,
                final_point,
                inventory.current_color.light_beam_color().darker(0.3),
            );
            break;
        };

        if intersection.time_of_impact < 0.01 {
            break;
        }

        gizmos.line_2d(
            ray_pos,
            intersection.point,
            inventory.current_color.light_beam_color().darker(0.3),
        );

        ray_pos = intersection.point;
        ray_dir = ray_dir.reflect(intersection.normal);
        ray_qry = ray_qry.exclude_collider(entity);
    }
}
