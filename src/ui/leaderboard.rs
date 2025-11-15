use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
};

use crate::{
    save::{CurrentUser, SaveParam},
    shared::UiState,
    ui::{UiButton, UiClick, UiFont, UiFontSize},
    utils::hhmmss::Hhmmss,
};

pub const USER_TOOLTIP_OPACITY: f32 = 0.2;

pub struct LeaderboardUiPlugin;

impl Plugin for LeaderboardUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(UiState::Leaderboard), spawn_leaderboard_ui);
        app.add_systems(
            Update,
            handle_leaderboard_input.run_if(in_state(UiState::Leaderboard)),
        );
        app.add_systems(OnExit(UiState::Leaderboard), despawn_leaderboard_ui);
    }
}

#[derive(Component)]
pub struct LeaderboardUi;

#[derive(Component)]
pub struct LeaderboardUserInput;

pub fn spawn_leaderboard_ui(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    current_user: Res<CurrentUser>,
    save_param: SaveParam,
) {
    let container = commands
        .spawn(LeaderboardUi)
        .insert(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(96.0)),
            column_gap: Val::Px(32.),
            row_gap: Val::Px(32.),
            overflow: Overflow::scroll_y(),
            ..default()
        })
        .insert(BackgroundColor(Color::BLACK))
        .id();

    commands
        .spawn(Text::new("Leaderboard"))
        .insert(ui_font.text_font().with_font_size(UiFontSize::HEADER))
        .insert(ChildOf(container));

    let center_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.0),
            row_gap: Val::Px(48.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .insert(ChildOf(container))
        .id();

    let user_input_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Auto,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(64.),
            ..default()
        })
        .insert(ChildOf(center_container))
        .id();

    let input_container = commands
        .spawn(Node {
            width: Val::Auto,
            height: Val::Auto,
            min_width: Val::Px(400.),
            padding: UiRect::new(Val::Px(12.), Val::Px(12.), Val::Px(4.), Val::Px(4.)),
            border: UiRect::all(Val::Px(2.)),
            ..default()
        })
        .insert(ChildOf(user_input_container))
        .insert(BorderColor::all(Color::WHITE))
        .id();

    let mut input = commands.spawn(LeaderboardUserInput);

    input
        .insert(ChildOf(input_container))
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON));

    if current_user.0.is_empty() {
        input
            .insert(Text::new("Enter Username!"))
            .insert(TextColor(Color::WHITE.with_alpha(USER_TOOLTIP_OPACITY)));
    } else {
        input
            .insert(Text::new(current_user.0.clone()))
            .insert(TextColor(Color::WHITE.with_alpha(1.)));
    }

    commands
        .spawn(Text::new("Next"))
        .insert(Button)
        .insert(UiButton)
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .insert(ChildOf(user_input_container))
        .observe(
            |_: On<UiClick>,
             mut next_ui_state: ResMut<NextState<UiState>>,
             current_user: Res<CurrentUser>| {
                if current_user.0.is_empty() {
                    return;
                }
                info!("Entering Level Select as user {}", &current_user.0);
                next_ui_state.set(UiState::LevelSelect);
            },
        );

    let leaderboard_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            max_width: Val::Px(1280.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Start,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.),
            overflow: Overflow::scroll_y(),
            ..default()
        })
        .insert(ChildOf(center_container))
        .id();

    let mut top_entries: Vec<_> = save_param.save_file.data.iter().collect();
    top_entries.sort_by_key(|(_, data)| {
        (
            data.level.solved(),
            -(data.timer.timer.elapsed().as_nanos() as i128),
        )
    });

    let header = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Auto,
            justify_content: JustifyContent::SpaceBetween,
            row_gap: Val::Px(24.),
            padding: UiRect::new(Val::Px(36.), Val::Px(36.), Val::Px(4.), Val::Px(4.)),
            ..default()
        })
        .insert(ChildOf(leaderboard_container))
        .id();

    commands
        .spawn(Node {
            width: Val::Percent(40.),
            ..default()
        })
        .insert(ChildOf(header))
        .with_child((
            Text::new("Username"),
            ui_font.text_font().with_font_size(UiFontSize::BUTTON),
        ));

    commands
        .spawn(Node {
            width: Val::Percent(30.),
            ..default()
        })
        .insert(ChildOf(header))
        .with_child((
            Text::new("Solved"),
            ui_font.text_font().with_font_size(UiFontSize::BUTTON),
        ));

    commands
        .spawn(Node {
            width: Val::Percent(30.),
            ..default()
        })
        .insert(ChildOf(header))
        .with_child((
            Text::new("Time"),
            ui_font.text_font().with_font_size(UiFontSize::BUTTON),
        ));

    for (user, data) in top_entries.iter().rev() {
        let row = commands
            .spawn(Node {
                width: Val::Percent(100.),
                height: Val::Auto,
                justify_content: JustifyContent::SpaceBetween,
                row_gap: Val::Px(24.),
                padding: UiRect::new(Val::Px(36.), Val::Px(36.), Val::Px(4.), Val::Px(4.)),
                ..default()
            })
            .insert(ChildOf(leaderboard_container))
            .id();

        commands
            .spawn(Node {
                width: Val::Percent(40.),
                ..default()
            })
            .insert(ChildOf(row))
            .with_child((
                Text::new((*user).clone()),
                ui_font.text_font().with_font_size(UiFontSize::TEXT),
            ));

        commands
            .spawn(Node {
                width: Val::Percent(30.),
                ..default()
            })
            .insert(ChildOf(row))
            .with_child((
                Text::new(format!("{}", data.level.solved())),
                ui_font.text_font().with_font_size(UiFontSize::TEXT),
            ));

        commands
            .spawn(Node {
                width: Val::Percent(30.),
                ..default()
            })
            .insert(ChildOf(row))
            .with_child((
                Text::new(data.timer.timer.elapsed().hhmmssxxx().to_string()),
                ui_font.text_font().with_font_size(UiFontSize::TEXT),
            ));
    }

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

pub fn handle_leaderboard_input(
    mut commands: Commands,
    mut evr_kbd: MessageReader<KeyboardInput>,
    mut current_user: ResMut<CurrentUser>,
    mut next_ui_state: ResMut<NextState<UiState>>,
    input_entity: Single<Entity, With<LeaderboardUserInput>>,
) {
    for ev in evr_kbd.read() {
        if ev.state == ButtonState::Released {
            continue;
        }
        match &ev.logical_key {
            Key::Enter => {
                if current_user.0.is_empty() {
                    continue;
                }
                info!("Entering Level Select as user {}", &current_user.0);
                next_ui_state.set(UiState::LevelSelect);
            }
            Key::Backspace => {
                current_user.0.pop();
            }
            Key::Character(input) => {
                if input.chars().any(|c| c.is_control()) {
                    continue;
                }
                if current_user.0.len() >= 24 {
                    continue;
                }
                current_user.0.push_str(input);
            }
            _ => {}
        }
        if current_user.0.is_empty() {
            commands
                .entity(*input_entity)
                .insert(Text::new("Enter Username!"))
                .insert(TextColor(Color::WHITE.with_alpha(USER_TOOLTIP_OPACITY)));
        } else {
            commands
                .entity(*input_entity)
                .insert(Text::new(current_user.0.clone()))
                .insert(TextColor(Color::WHITE.with_alpha(1.)));
        }
    }
}

pub fn despawn_leaderboard_ui(
    loading_ui: Single<Entity, With<LeaderboardUi>>,
    mut commands: Commands,
) {
    info!("Despawning Leaderboard UI!");

    commands.entity(*loading_ui).despawn();
}
