use bevy::prelude::*;
use enum_map::{enum_map, EnumMap};

use crate::{
    asset::LoadResource,
    game::{
        light::LightColor,
        lyra::{beam::PlayerLightInventory, Lyra},
    },
    shared::GameState,
    ui::UiFont,
};

pub struct LightUiPlugin;

impl Plugin for LightUiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LightUiAssets>();
        app.load_resource::<LightUiAssets>();
        app.add_systems(OnEnter(GameState::InGame), spawn_light_icons);
        app.add_systems(OnExit(GameState::InGame), despawn_light_ui);
        app.add_systems(
            Update,
            update_light_icons.run_if(in_state(GameState::InGame)),
        );
    }
}

#[derive(Resource)]
pub struct LightUiIcons {
    icon_entities: EnumMap<LightColor, Entity>,
}

#[derive(Resource, Asset, Reflect, Clone)]
#[reflect(Resource)]
pub struct LightUiAssets {
    #[dependency]
    icons: [Handle<Image>; 4],
}

impl FromWorld for LightUiAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            icons: [
                asset_server.load("ui/green_light_icon.png"),
                asset_server.load("ui/purple_light_icon.png"),
                asset_server.load("ui/blue_light_icon.png"),
                asset_server.load("ui/white_light_icon.png"),
            ],
        }
    }
}

pub fn spawn_light_icons(
    mut commands: Commands,
    light_ui_assets: Res<LightUiAssets>,
    ui_font: Res<UiFont>,
) {
    let icons: EnumMap<LightColor, Handle<Image>> = enum_map! {
        LightColor::Green => light_ui_assets.icons[0].clone(),
        LightColor::Purple => light_ui_assets.icons[1].clone(),
        LightColor::Blue => light_ui_assets.icons[2].clone(),
        LightColor::White => light_ui_assets.icons[3].clone(),
    };

    let overlay = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            justify_content: JustifyContent::End,
            ..default()
        })
        .insert(LightUiMarker)
        .insert(GlobalZIndex(-1))
        .id();

    let container = commands
        .spawn(Node {
            width: Val::Auto,
            height: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(16.)),
            row_gap: Val::Px(16.),
            ..default()
        })
        .insert(ChildOf(overlay))
        .id();

    let mut spawn_and_get_icon_id = |val: LightColor| {
        let text = match val {
            LightColor::Green => "1",
            LightColor::Purple => "2",
            LightColor::White => "3",
            LightColor::Blue => "4",
        };
        let icon = commands
            .spawn(ImageNode::from(icons[val].clone()))
            .insert(Node {
                width: Val::Px(64.),
                height: Val::Px(64.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .insert(ChildOf(container))
            .id();

        commands
            .spawn(Node {
                width: Val::Px(64.),
                height: Val::Px(64.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .insert(UiTransform::from_translation(Val2::new(
                Val::Percent(-120.),
                Val::ZERO,
            )))
            .insert(TextLayout::new_with_justify(Justify::Right))
            .insert(Text::new(text))
            .insert(ui_font.text_font().with_font_size(32.))
            .insert(ChildOf(icon));

        icon
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
    inventory: Single<&PlayerLightInventory, With<Lyra>>,
    mut q_nodes: Query<(&mut Node, &mut ImageNode)>,
) {
    for (color, can_use) in inventory.allowed.iter() {
        let Ok((mut icon_node, mut icon_image)) = q_nodes.get_mut(light_icons.icon_entities[color])
        else {
            return;
        };

        let is_color = inventory.current_color.is_some_and(|c| color == c);
        if *can_use {
            icon_node.display = Display::Block;
            if inventory.collectible[color].is_none() {
                if is_color {
                    icon_image.color.set_alpha(1.0);
                } else {
                    icon_image.color.set_alpha(0.4);
                }
            } else {
                icon_image.color.set_alpha(0.1);
            }
        } else {
            icon_node.display = Display::None;
        }
    }
}

pub fn despawn_light_ui(mut commands: Commands, light_ui: Single<Entity, With<LightUiMarker>>) {
    commands.remove_resource::<LightUiIcons>();
    commands.entity(*light_ui).despawn();
}
