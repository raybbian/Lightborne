use bevy::{prelude::*, time::Stopwatch};

use crate::{shared::GameState, utils::hhmmss::Hhmmss};

pub struct SpeedrunTimerPlugin;

impl Plugin for SpeedrunTimerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, tick_speedrun_timer)
            .init_resource::<SpeedrunTimer>();
    }
}

#[derive(Default, Resource)]
pub struct SpeedrunTimer {
    pub enabled: bool,
    timer: Stopwatch,
}

#[derive(Component)]
pub struct SpeedrunUi;

pub fn tick_speedrun_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut speedrun_timer: ResMut<SpeedrunTimer>,
    q_speedrun_timer: Query<Entity, With<SpeedrunUi>>,
    game_state: Res<State<GameState>>,
    asset_server: Res<AssetServer>,
) {
    if *game_state == GameState::Playing {
        speedrun_timer.timer.tick(time.delta());
    }
    if !speedrun_timer.enabled {
        return;
    };

    let font = TextFont {
        font: asset_server.load("fonts/Outfit-Medium.ttf"),
        ..default()
    };

    let timer = q_speedrun_timer.get_single();
    match timer {
        Ok(timer) => {
            commands
                .entity(timer)
                .insert(Text::new(speedrun_timer.timer.elapsed().hhmmssxxx()));
        }
        Err(_) => {
            commands
                .spawn(Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    padding: UiRect::all(Val::Px(32.)),
                    ..default()
                })
                .with_child((
                    Text::new(speedrun_timer.timer.elapsed().hhmmssxxx()),
                    SpeedrunUi,
                    font.with_font_size(36.),
                ));
        }
    }
}
