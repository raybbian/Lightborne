use bevy::{prelude::*, ui::widget::NodeImageMode};

use crate::{
    shared::{GameState, UiState},
    sound::{BgmTrack, ChangeBgmEvent},
};

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
pub enum StartMenuButtonMarker {
    Play,
    Settings,
    Quit,
}

fn spawn_start(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_start_menu: Query<Entity, With<StartMenuMarker>>,
    mut ev_change_bgm: EventWriter<ChangeBgmEvent>,
) {
    if q_start_menu.get_single().is_ok() {
        return;
    };

    let font = TextFont {
        font: asset_server.load("fonts/Outfit-Medium.ttf"),
        ..default()
    };

    ev_change_bgm.send(ChangeBgmEvent(BgmTrack::LevelSelect));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                ..default()
            },
            ImageNode::from(asset_server.load("ui/start.png")).with_mode(NodeImageMode::Stretch),
            BackgroundColor(Color::BLACK),
            StartMenuMarker,
        ))
        .with_children(|container| {
            container
                .spawn((Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(70.),
                    top: Val::Percent(30.),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(32.0),
                    row_gap: Val::Px(32.0),
                    ..default()
                },))
                .with_child((
                    Node {
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    font.clone().with_font_size(48.),
                    Text::new("Play"),
                    Button,
                    StartMenuButtonMarker::Play,
                ))
                .with_child((
                    Node {
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    font.clone().with_font_size(48.),
                    Text::new("Settings"),
                    Button,
                    StartMenuButtonMarker::Settings,
                ))
                .with_child((
                    Node {
                        width: Val::Auto,
                        height: Val::Auto,
                        ..default()
                    },
                    font.clone().with_font_size(48.),
                    Text::new("Quit"),
                    Button,
                    StartMenuButtonMarker::Quit,
                ));
        });
}

fn exit_start(mut commands: Commands, query: Query<Entity, With<StartMenuMarker>>) {
    let Ok(entity) = query.get_single() else {
        return;
    };
    commands.entity(entity).despawn_recursive();
}

fn start_game(
    mut commands: Commands,
    q_button: Query<(&Interaction, &StartMenuButtonMarker), Changed<Interaction>>,
    mut next_ui_state: ResMut<NextState<UiState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
    asset_server: Res<AssetServer>,
) {
    for (interaction, button_marker) in q_button.iter() {
        match *interaction {
            Interaction::Pressed => {
                commands.spawn((
                    AudioPlayer::new(asset_server.load("sfx/click.wav")),
                    PlaybackSettings::DESPAWN,
                ));

                next_game_state.set(GameState::Ui);
                match button_marker {
                    StartMenuButtonMarker::Play => {
                        next_ui_state.set(UiState::LevelSelect);
                    }
                    StartMenuButtonMarker::Settings => {
                        next_ui_state.set(UiState::Settings);
                    }
                    StartMenuButtonMarker::Quit => {
                        exit.send(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => {
                commands.spawn((
                    AudioPlayer::new(asset_server.load("sfx/hover.wav")),
                    PlaybackSettings::DESPAWN,
                ));
            }
            _ => {}
        }
    }
}
