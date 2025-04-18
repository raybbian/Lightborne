use bevy::{prelude::*, ui::widget::NodeImageMode};

use crate::shared::GameState;

pub struct StartMenuPlugin;

impl Plugin for StartMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_start)
            .add_systems(OnExit(GameState::StartMenu), exit_start)
            .add_systems(Update, start_game)
            .insert_resource(InputReady(false));
    }
}

#[derive(Component)]
pub struct StartMarker;

#[derive(Component)]
pub struct StartButtonMarker;

#[derive(Resource)]
pub struct InputReady(pub bool); 

fn spawn_start(mut commands: Commands, asset_server: Res<AssetServer>, mut input_ready: ResMut<InputReady>) {
    input_ready.0 = false;
    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                ..default()
            },
            ImageNode::from(asset_server.load("ui/start.png")).with_mode(NodeImageMode::Stretch),
            StartMarker,
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
            StartButtonMarker,
        ));
}

fn exit_start(mut commands: Commands, query: Query<Entity, With<StartMarker>>) {
    let Ok(entity) = query.get_single() else {
        return;
    };
    commands.entity(entity).despawn_recursive();
}

fn start_game(
    q_button: Query<&Interaction, (With<StartButtonMarker>, Changed<Interaction>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(interaction) = &q_button.get_single() else {
        return;
    };
    match interaction {
        Interaction::Pressed => {
            next_state.set(GameState::Playing);
        }
        _ => {}
    }
}
