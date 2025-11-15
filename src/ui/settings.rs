use bevy::prelude::*;

use crate::shared::UiState;
use crate::sound::{BgmTrack, ChangeBgmEvent};
use crate::ui::{UiButton, UiClick, UiFont, UiFontSize};

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(UiState::Settings), spawn_settings);
        app.add_systems(OnExit(UiState::Settings), despawn_settings);
    }
}

#[derive(Component)]
struct SettingsUiMarker;

const CONTROLS: [(&str, &str); 9] = [
    ("Restart", "R"),
    ("Pick Up Shard", "E"),
    ("Jump", "Space"),
    ("Movement", "WASD"),
    ("Sneak", "Control"),
    ("Snap Angles", "Shift"),
    ("Aim Light", "Left Click (Press)"),
    ("Shoot Light", "Left Click (Release)"),
    ("Cancel Shoot Light", "Right Click"),
];

fn spawn_settings(mut commands: Commands, ui_font: Res<UiFont>) {
    info!("Spawning Settings Menu");

    commands.trigger(ChangeBgmEvent(BgmTrack::None));

    let controls_nodes = CONTROLS.map(|(action, control)| {
        commands
            .spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|parent| {
                parent.spawn((Text::new(action), ui_font.text_font().with_font_size(24.0)));
                parent.spawn((Text::new(control), ui_font.text_font().with_font_size(24.0)));
            })
            .id()
    });

    let container = commands
        .spawn(SettingsUiMarker)
        .insert(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(96.0)),
            overflow: Overflow::scroll_y(),
            column_gap: Val::Px(32.),
            row_gap: Val::Px(32.),
            ..default()
        })
        .insert(GlobalZIndex(1000))
        .insert(BackgroundColor(Color::BLACK))
        .id();

    commands
        .spawn(Text::new("Settings"))
        .insert(ui_font.text_font().with_font_size(UiFontSize::HEADER))
        .insert(ChildOf(container));

    commands
        .spawn(Node {
            width: Val::Percent(50.),
            height: Val::Percent(100.0),
            row_gap: Val::Px(6.),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .insert(ChildOf(container))
        .with_child((
            Node {
                margin: UiRect::bottom(Val::Px(24.)),
                ..default()
            },
            Text::new("Controls (Fixed)"),
            ui_font.text_font().with_font_size(36.),
        ))
        .add_children(&controls_nodes);

    commands
        .spawn(Text::new("Back"))
        .insert(Button)
        .insert(UiButton)
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .insert(ChildOf(container))
        .observe(
            |_: On<UiClick>, mut next_ui_state: ResMut<NextState<UiState>>| {
                next_ui_state.set(UiState::StartMenu);
            },
        );
}

fn despawn_settings(mut commands: Commands, settings_menu: Single<Entity, With<SettingsUiMarker>>) {
    info!("Despawning Settings Menu");

    commands.entity(*settings_menu).despawn();
}
