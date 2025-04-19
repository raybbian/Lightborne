use bevy::{input::common_conditions::input_just_pressed, prelude::*, ui::widget::NodeImageMode};

use crate::{
    shared::GameState,
    sound::{BgmTrack, ChangeBgmEvent},
};

use super::{settings::SettingsButton, start_menu::StartMenuButtonMarker};

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
                resume_button,
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

#[derive(Component)]
pub struct PauseMenuResume;

fn spawn_pause(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_pause: Query<Entity, With<PauseMarker>>,
    mut ev_change_bgm: EventWriter<ChangeBgmEvent>,
) {
    if q_pause.get_single().is_ok() {
        return;
    }

    let font = TextFont {
        font: asset_server.load("fonts/Outfit-Medium.ttf"),
        ..default()
    };

    ev_change_bgm.send(ChangeBgmEvent(BgmTrack::None));

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
        .with_children(|container| {
            container
                .spawn((
                    Node {
                        width: Val::Percent(80.),
                        height: Val::Percent(80.),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::all(Val::Percent(20.)),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(32.0),
                        row_gap: Val::Px(32.0),
                        ..default()
                    },
                    ImageNode::from(asset_server.load("ui/pause_menu.png"))
                        .with_mode(NodeImageMode::Stretch),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Paused"),
                        font.clone().with_font_size(48.),
                        Node {
                            margin: UiRect::all(Val::Px(32.)),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        Text::new("Resume"),
                        Button,
                        PauseMenuResume,
                        font.clone().with_font_size(36.),
                    ));
                    parent.spawn((
                        Text::new("Level Select"),
                        Button,
                        StartMenuButtonMarker::Play,
                        font.clone().with_font_size(36.),
                    ));
                    parent.spawn((
                        Text::new("Settings"),
                        Button,
                        StartMenuButtonMarker::Settings,
                        font.clone().with_font_size(36.),
                    ));
                    parent.spawn((
                        Text::new("Main Menu"),
                        Button,
                        SettingsButton::Back,
                        font.clone().with_font_size(36.),
                    ));
                });
        });
}

fn despawn_pause(mut commands: Commands, q_pause: Query<Entity, With<PauseMarker>>) {
    let Ok(pause_entity) = q_pause.get_single() else {
        return;
    };
    commands.entity(pause_entity).despawn_recursive();
}

fn resume_button(
    mut commands: Commands,
    q_button: Query<&Interaction, (With<PauseMenuResume>, Changed<Interaction>)>,
    mut next_game_state: ResMut<NextState<GameState>>,
    asset_server: Res<AssetServer>,
) {
    for interaction in q_button.iter() {
        match *interaction {
            Interaction::Pressed => {
                commands.spawn((
                    AudioPlayer::new(asset_server.load("sfx/click.wav")),
                    PlaybackSettings::DESPAWN,
                ));
                next_game_state.set(GameState::Playing);
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

fn toggle_pause(state: Res<State<GameState>>, mut next_state: ResMut<NextState<GameState>>) {
    match state.get() {
        GameState::Paused => next_state.set(GameState::Playing),
        GameState::Playing => next_state.set(GameState::Paused),
        _ => {}
    }
}
