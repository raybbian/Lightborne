use bevy::{input::common_conditions::input_just_pressed, prelude::*, ui::widget::NodeImageMode};

use crate::shared::GameState;

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pause)
            .add_systems(OnEnter(GameState::Paused), show_pause::<true>)
            .add_systems(OnExit(GameState::Paused), show_pause::<false>)
            .add_systems(
                Update,
                toggle_pause.run_if(input_just_pressed(KeyCode::Escape)),
            );
    }
}

#[derive(Component)]
pub struct PauseMarker;

fn spawn_pause(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Visibility::Hidden,
            PauseMarker,
        ))
        .with_child((
            ImageNode::from(asset_server.load("ui/m2_pause.png")).with_mode(NodeImageMode::Stretch),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ));
}

fn show_pause<const SHOW: bool>(mut query: Query<&mut Visibility, With<PauseMarker>>) {
    let Ok(mut pause_visibility) = query.get_single_mut() else {
        return;
    };
    *pause_visibility = if SHOW {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

fn toggle_pause(state: Res<State<GameState>>, mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(match state.get() {
        GameState::Paused => GameState::Playing,
        GameState::Playing => GameState::Paused,
        _ => state.get().clone(),
    })
}
