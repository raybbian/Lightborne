use bevy::{prelude::*, time::Stopwatch};
use serde::{Deserialize, Serialize};

use crate::{
    save::SaveParam,
    shared::{GameState, PlayState, UiState},
    ui::UiFont,
    utils::hhmmss::Hhmmss,
};

pub struct SpeedrunTimerPlugin;

impl Plugin for SpeedrunTimerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpeedrunTimer>();
        app.add_systems(
            OnExit(UiState::Leaderboard),
            init_speedrun_timer.before(spawn_speedrun_timer),
        );
        app.add_systems(OnEnter(GameState::InGame), spawn_speedrun_timer);
        app.add_systems(OnExit(GameState::InGame), despawn_speedrun_timer);
        app.add_systems(
            Update,
            tick_speedrun_timer.run_if(in_state(PlayState::Playing)),
        );
    }
}

#[derive(Default, Resource, Serialize, Deserialize, Clone)]
pub struct SpeedrunTimer {
    pub timer: Stopwatch,
}

pub fn init_speedrun_timer(mut speedrun_timer: ResMut<SpeedrunTimer>, save_param: SaveParam) {
    if let Some(save_data) = save_param.get_save_data() {
        *speedrun_timer = save_data.timer.clone();
    } else {
        *speedrun_timer = SpeedrunTimer {
            timer: Stopwatch::new(),
        }
    }
}

#[derive(Component)]
pub struct SpeedrunUiMarker;

pub fn spawn_speedrun_timer(
    mut commands: Commands,
    speedrun_timer: Res<SpeedrunTimer>,
    ui_font: Res<UiFont>,
) {
    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            padding: UiRect::all(Val::Px(32.)),
            ..default()
        })
        .with_child((
            Text::new(speedrun_timer.timer.elapsed().hhmmssxxx()),
            SpeedrunUiMarker,
            ui_font.text_font().with_font_size(36.),
        ));
}

pub fn tick_speedrun_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut speedrun_timer: ResMut<SpeedrunTimer>,
    speedrun_ui: Single<Entity, With<SpeedrunUiMarker>>,
) {
    speedrun_timer.timer.tick(time.delta());

    commands
        .entity(*speedrun_ui)
        .insert(Text::new(speedrun_timer.timer.elapsed().hhmmssxxx()));
}

pub fn despawn_speedrun_timer(
    mut commands: Commands,
    speedrun_ui: Option<Single<Entity, With<SpeedrunUiMarker>>>,
) {
    let Some(speedrun_ui) = speedrun_ui else {
        return;
    };
    commands.entity(*speedrun_ui).despawn();
}
