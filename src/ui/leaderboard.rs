use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
};

use crate::{
    save::{CurrentUser, SaveFile, SaveParam},
    shared::UiState,
    ui::{
        pause::{check_save_state, WreckconAssets},
        UiButton, UiClick, UiFont, UiFontSize,
    },
    utils::hhmmss::Hhmmss,
};

pub const USER_TOOLTIP_OPACITY: f32 = 0.2;

pub struct LeaderboardUiPlugin;

impl Plugin for LeaderboardUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(UiState::Leaderboard), spawn_leaderboard_ui);
        app.add_systems(
            Update,
            (
                handle_leaderboard_input.run_if(in_state(UiState::Leaderboard)),
                update_task_preview.run_if(in_state(UiState::Leaderboard)),
            ),
        );
        app.add_systems(OnExit(UiState::Leaderboard), despawn_leaderboard_ui);
    }
}

#[derive(Component)]
pub struct LeaderboardUi;

#[derive(Component)]
pub struct LeaderboardUserInput;

#[derive(Component)]
pub struct RowFor(pub String);

pub fn spawn_leaderboard_ui(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    mut current_user: ResMut<CurrentUser>,
    save_file: Res<SaveFile>,
    wreckcon_assets: Res<WreckconAssets>,
) {
    current_user.0 = "".to_string();
    let container = commands
        .spawn(LeaderboardUi)
        .insert(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            max_width: Val::Percent(100.),
            max_height: Val::Percent(100.),
            justify_content: JustifyContent::SpaceBetween,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            position_type: PositionType::Relative,
            padding: UiRect::all(Val::Px(96.0)),
            column_gap: Val::Px(32.),
            row_gap: Val::Px(32.),
            overflow: Overflow::scroll_y(),
            ..default()
        })
        .insert(GlobalZIndex(1000))
        // NOTE: required because GameState::InGame spawns levels
        // probably that cover the ui invisibly
        .insert(BackgroundColor(Color::BLACK))
        .id();

    commands
        .spawn(Text::new("Leaderboard"))
        .insert(ui_font.text_font().with_font_size(UiFontSize::HEADER))
        .insert(ChildOf(container));

    let center_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Auto,
            flex_grow: 1.0,
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
            height: Val::Auto,
            flex_grow: 4.,
            justify_content: JustifyContent::Start,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.),
            overflow: Overflow::scroll_y(),
            ..default()
        })
        .insert(ChildOf(center_container))
        .id();

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

    let border = commands
        .spawn(Node {
            width: Val::Auto,
            height: Val::Percent(50.),
            flex_grow: 1.,
            aspect_ratio: Some(1600. / 2022.),
            padding: UiRect::all(Val::Px(4.)),
            border: UiRect::all(Val::Px(2.)),
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.),
            right: Val::Px(0.),
            ..default()
        })
        .insert(ChildOf(container))
        .insert(BorderColor::all(Color::WHITE))
        .id();

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            position_type: PositionType::Relative,
            ..default()
        })
        .insert(ImageNode::from(wreckcon_assets.bg.clone()))
        .insert(TaskPreviewMarker)
        .insert(ChildOf(border));

    let mut top_entries: Vec<_> = save_file.data.iter().collect();
    top_entries.sort_by_key(|(_, data)| {
        (
            data.level.solved(),
            -(data.timer.timer.elapsed().as_nanos() as i128),
        )
    });

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
        let user = *user;
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
            .insert(RowFor(user.clone()))
            .insert(UiButton)
            .observe(
                |e: On<Pointer<Over>>,
                 q_row_for: Query<&RowFor>,
                 mut current_user: ResMut<CurrentUser>| {
                    let Ok(row_for) = q_row_for.get(e.entity) else {
                        return;
                    };
                    current_user.0 = row_for.0.clone();
                },
            )
            .observe(
                |_: On<Pointer<Click>>,
                 current_user: ResMut<CurrentUser>,
                 mut next_ui_state: ResMut<NextState<UiState>>| {
                    next_ui_state.set(UiState::LevelSelect);
                    info!("Entering Level Select as user {}", &current_user.0);
                },
            )
            .id();

        commands
            .spawn(Node {
                width: Val::Percent(40.),
                ..default()
            })
            .insert(ChildOf(row))
            .with_child((
                Text::new(user.clone()),
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

#[derive(Component)]
pub struct TaskPreviewMarker;

pub fn update_task_preview(
    mut commands: Commands,
    task_node: Single<Entity, With<TaskPreviewMarker>>,
    wreckcon_assets: Res<WreckconAssets>,
    save: SaveParam,
) {
    commands.entity(*task_node).despawn_children();
    let Some(save_data) = save.get_save_data() else {
        return;
    };
    let checks = check_save_state(save_data);

    const X_PERCENTAGE: f32 = 87.5;
    const Y_PERCENTAGES: &[f32] = &[
        29.12, 35.39, 41.65, 47.92, 54.19, 60.46, 66.73, 73.00, 79.27, 85.53,
    ];

    for i in 0..10 {
        if !checks[i] {
            continue;
        }
        commands
            .spawn(Node {
                width: Val::Percent(5.),
                height: Val::Percent(5.),
                position_type: PositionType::Absolute,
                top: Val::Percent(Y_PERCENTAGES[i]),
                left: Val::Percent(X_PERCENTAGE),
                ..default()
            })
            .insert(ChildOf(*task_node))
            .insert(UiTransform::from_translation(Val2::new(
                Val::Percent(-50.),
                Val::Percent(-50.),
            )))
            .insert(ImageNode::from(wreckcon_assets.complete_icon.clone()));
    }
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

pub fn despawn_leaderboard_ui(
    loading_ui: Single<Entity, With<LeaderboardUi>>,
    mut commands: Commands,
) {
    info!("Despawning Leaderboard UI!");

    commands.entity(*loading_ui).despawn();
}
