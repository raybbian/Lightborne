use bevy::{input::common_conditions::input_just_pressed, prelude::*, ui::widget::NodeImageMode};

use crate::shared::GameState;

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_pause.run_if(in_state(GameState::Paused)),
                despawn_pause.run_if(not(in_state(GameState::Paused))),
                spawn_cover.run_if(in_state(GameState::Ui)),
                despawn_cover.run_if(not(in_state(GameState::Ui))),
            ),
        )
        .add_systems(
            Update,
            toggle_pause.run_if(input_just_pressed(KeyCode::Escape)),
        );
    }
}

// Covers level when not in playing
#[derive(Component)]
pub struct CoverMarker;

fn spawn_cover(mut commands: Commands, q_cover: Query<Entity, With<CoverMarker>>) {
    if q_cover.get_single().is_ok() {
        return;
    }
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(-1),
        BackgroundColor(Color::BLACK),
        CoverMarker,
    ));
}

fn despawn_cover(mut commands: Commands, q_cover: Query<Entity, With<CoverMarker>>) {
    let Ok(cover_entity) = q_cover.get_single() else {
        return;
    };
    commands.entity(cover_entity).despawn_recursive();
}

#[derive(Component)]
pub struct PauseMarker;

fn spawn_pause(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_pause: Query<Entity, With<PauseMarker>>,
) {
    if q_pause.get_single().is_ok() {
        return;
    }
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            PauseMarker,
        ))
        .with_child((
            Node {
                width: Val::Percent(80.),
                height: Val::Percent(80.),
                ..default()
            },
            ImageNode::from(asset_server.load("ui/pause_menu.png"))
                .with_mode(NodeImageMode::Stretch),
        ));
}

fn despawn_pause(mut commands: Commands, q_pause: Query<Entity, With<PauseMarker>>) {
    let Ok(pause_entity) = q_pause.get_single() else {
        return;
    };
    commands.entity(pause_entity).despawn_recursive();
}

fn toggle_pause(state: Res<State<GameState>>, mut next_state: ResMut<NextState<GameState>>) {
    match state.get() {
        GameState::Paused => next_state.set(GameState::Playing),
        GameState::Playing => next_state.set(GameState::Paused),
        _ => {}
    }
}
