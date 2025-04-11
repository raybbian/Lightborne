use bevy::prelude::*;
use enum_map::{enum_map, EnumMap};

use crate::{
    camera::{setup_camera, MainCamera},
    level::{CurrentLevel, LevelSystems},
    light::LightColor,
    player::PlayerMarker,
};

use super::PlayerLightInventory;

pub struct LightUiPlugin;

impl Plugin for LightUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_light_icons.after(setup_camera))
            .add_systems(
                FixedUpdate,
                update_light_icons.in_set(LevelSystems::Simulation),
            )
            .add_systems(Update, update_light_icons.in_set(LevelSystems::Reset));
    }
}

#[derive(Resource)]
pub struct LightUiIcons {
    icon_entities: EnumMap<LightColor, Entity>,
}

pub fn spawn_light_icons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_main_camera: Query<Entity, With<MainCamera>>,
) {
    let Ok(main_camera) = q_main_camera.get_single() else {
        return;
    };
    let icons: EnumMap<LightColor, Handle<Image>> = enum_map! {
        LightColor::Green => asset_server.load("ui/green_light_icon.png"),
        LightColor::Purple => asset_server.load("ui/purple_light_icon.png"),
        LightColor::Blue => asset_server.load("ui/blue_light_icon.png"),
        LightColor::White => asset_server.load("ui/white_light_icon.png"),
        LightColor::Black => asset_server.load("ui/black_light_icon.png"),
    };

    // better way to get ID of child?
    let mut container: Option<Entity> = None;
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                justify_content: JustifyContent::End,
                ..default()
            },
            Visibility::Visible,
            LightUiMarker,
            // spawn underneath the level select UI
            GlobalZIndex(-1),
            // show underneath screen transitions
            TargetCamera(main_camera),
        ))
        .with_children(|ui| {
            container = Some(
                ui.spawn(Node {
                    width: Val::Auto,
                    height: Val::Percent(100.0),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Vw(1.)),
                    row_gap: Val::Vw(1.),
                    ..default()
                })
                .id(),
            );
        });

    let mut spawn_and_get_icon_id = |val: LightColor| {
        let mut icon: Option<Entity> = None;
        commands
            .entity(container.unwrap())
            .with_children(|container| {
                icon = Some(
                    container
                        .spawn((
                            ImageNode::from(icons[val].clone()),
                            Node {
                                width: Val::Vw(5.),
                                height: Val::Vw(5.),
                                ..default()
                            },
                        ))
                        .id(),
                );
            });
        icon.unwrap()
    };

    let icon_entities = enum_map! {
        val => spawn_and_get_icon_id(val),
    };

    commands.insert_resource(LightUiIcons { icon_entities });
}

#[derive(Component)]
pub struct LightUiMarker;

pub fn update_light_icons(
    light_icons: Res<LightUiIcons>,
    current_level: Res<CurrentLevel>,
    q_player: Query<&PlayerLightInventory, With<PlayerMarker>>,
    mut q_nodes: Query<(&mut Node, &mut ImageNode)>,
) {
    let Ok(player_light_inventory) = q_player.get_single() else {
        return;
    };
    for (color, can_use) in current_level.allowed_colors.iter() {
        let Ok((mut icon_node, mut icon_image)) = q_nodes.get_mut(light_icons.icon_entities[color])
        else {
            return;
        };

        if *can_use {
            icon_node.display = Display::Block;
            if player_light_inventory.sources[color] {
                icon_image.color.set_alpha(1.);
            } else {
                icon_image.color.set_alpha(0.2);
            }
        } else {
            icon_node.display = Display::None;
        }
    }
}
