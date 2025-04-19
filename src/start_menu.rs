use bevy::{prelude::*, ui::widget::NodeImageMode};

use crate::shared::UiState;

pub struct StartMenuPlugin;

impl Plugin for StartMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_start.run_if(in_state(UiState::StartMenu)))
            .add_systems(Update, exit_start.run_if(not(in_state(UiState::StartMenu))))
            .add_systems(Update, start_game);
    }
}

#[derive(Component)]
pub struct StartMenuMarker;

#[derive(Component)]
pub struct StartMenuButtonMarker;

fn spawn_start(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_start_menu: Query<Entity, With<StartMenuMarker>>,
) {
    if let Ok(_) = q_start_menu.get_single() {
        return;
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                ..default()
            },
            ImageNode::from(asset_server.load("ui/start.png")).with_mode(NodeImageMode::Stretch),
            StartMenuMarker,
        ))
        .with_child((
            Node {
                width: Val::Px(120.),
                height: Val::Px(60.),
                top: Val::Percent(60.),
                ..default()
            },
            ImageNode::from(asset_server.load("ui/start_button.png")),
            Button,
            StartMenuButtonMarker,
        ));
}

fn exit_start(mut commands: Commands, query: Query<Entity, With<StartMenuMarker>>) {
    let Ok(entity) = query.get_single() else {
        return;
    };
    commands.entity(entity).despawn_recursive();
}

fn start_game(
    q_button: Query<&Interaction, (With<StartMenuButtonMarker>, Changed<Interaction>)>,
    mut next_state: ResMut<NextState<UiState>>,
) {
    let Ok(interaction) = &q_button.get_single() else {
        return;
    };
    match interaction {
        Interaction::Pressed => {
            next_state.set(UiState::LevelSelect);
        }
        _ => {}
    }
}
