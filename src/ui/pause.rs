use bevy::{input::common_conditions::input_just_pressed, prelude::*};

use crate::{
    asset::LoadResource,
    game::light::LightColor,
    save::{Save, SaveData, SaveParam},
    shared::{GameState, PlayState, UiState},
    ui::{UiButton, UiClick, UiFont, UiFontSize},
};

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WreckconAssets>();
        app.load_resource::<WreckconAssets>();
        app.register_type::<PauseAssets>();
        app.load_resource::<PauseAssets>();
        app.add_systems(OnEnter(PlayState::Paused), spawn_pause);
        app.add_systems(OnExit(PlayState::Paused), despawn_pause);
        app.add_systems(
            Update,
            toggle_pause
                .run_if(in_state(GameState::InGame))
                .run_if(input_just_pressed(KeyCode::Escape)),
        );
    }
}
#[derive(Resource, Asset, Reflect, Clone)]
#[reflect(Resource)]
pub struct WreckconAssets {
    #[dependency]
    pub bg: Handle<Image>,
    #[dependency]
    pub complete_icon: Handle<Image>,
}

impl FromWorld for WreckconAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            bg: asset_server.load("lightborne_task_card_wreckcon.png"),
            complete_icon: asset_server.load("wreckcon_complete_star.png"),
        }
    }
}

#[derive(Resource, Asset, Reflect, Clone)]
#[reflect(Resource)]
pub struct PauseAssets {
    #[dependency]
    bg: Handle<Image>,
}

impl FromWorld for PauseAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            bg: asset_server.load("ui/pause_menu.png"),
        }
    }
}

#[derive(Component)]
pub struct PauseMarker;

fn spawn_pause(
    mut commands: Commands,
    ui_font: Res<UiFont>,
    wreckcon_assets: Res<WreckconAssets>,
    save: SaveParam,
) {
    let container = commands
        .spawn(PauseMarker)
        .insert(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            display: Display::Flex,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(32.),
            ..default()
        })
        .observe(
            |_: On<UiClick>, mut next_play_state: ResMut<NextState<PlayState>>| {
                next_play_state.set(PlayState::Playing);
            },
        )
        .id();

    let pause_container = commands
        .spawn(())
        .insert(Node {
            width: Val::Auto,
            height: Val::Auto,
            // margin: UiRect::new(Val::Vw(20.), Val::Vw(20.), Val::Vh(20.), Val::Vh(20.)),
            justify_content: JustifyContent::SpaceBetween,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::new(Val::Px(120.), Val::Px(120.), Val::Px(96.), Val::Px(96.)),
            column_gap: Val::Px(32.),
            row_gap: Val::Px(96.),
            border: UiRect::all(Val::Px(2.)),
            ..default()
        })
        .insert(BorderColor::all(Color::WHITE))
        .insert(ChildOf(container))
        .insert(BackgroundColor(Color::BLACK.with_alpha(0.5)))
        .id();

    let border = commands
        .spawn(Node {
            width: Val::Auto,
            height: Val::Percent(75.),
            aspect_ratio: Some(1600. / 2022.),
            padding: UiRect::all(Val::Px(4.)),
            border: UiRect::all(Val::Px(2.)),
            ..default()
        })
        .insert(ChildOf(container))
        .insert(BorderColor::all(Color::WHITE))
        .id();

    let Some(save_data) = save.get_save_data() else {
        return;
    };

    let checks = check_save_state(save_data);
    const X_PERCENTAGE: f32 = 87.5;
    const Y_PERCENTAGES: &[f32] = &[
        29.12, 35.39, 41.65, 47.92, 54.19, 60.46, 66.73, 73.00, 79.27, 85.53,
    ];

    let bg = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            position_type: PositionType::Relative,
            ..default()
        })
        .insert(ImageNode::from(wreckcon_assets.bg.clone()))
        .insert(ChildOf(border))
        .id();

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
            .insert(ChildOf(bg))
            .insert(UiTransform::from_translation(Val2::new(
                Val::Percent(-50.),
                Val::Percent(-50.),
            )))
            .insert(ImageNode::from(wreckcon_assets.complete_icon.clone()));
    }

    commands
        .spawn(Text::new("Paused"))
        .insert(ui_font.text_font().with_font_size(UiFontSize::HEADER))
        .insert(ChildOf(pause_container));

    let center_container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(32.),
            row_gap: Val::Px(32.),
            ..default()
        })
        .insert(ChildOf(pause_container))
        .id();

    commands
        .spawn(Text::new("Main Menu"))
        .insert(Button)
        .insert(UiButton)
        .insert(ChildOf(center_container))
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .observe(
            |_: On<UiClick>,
             mut commands: Commands,
             mut next_game_state: ResMut<NextState<GameState>>,
             mut next_ui_state: ResMut<NextState<UiState>>| {
                commands.trigger(Save);
                next_game_state.set(GameState::Ui);
                next_ui_state.set(UiState::StartMenu);
            },
        );

    commands
        .spawn(Text::new("Resume"))
        .insert(Button)
        .insert(UiButton)
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .insert(ChildOf(center_container))
        .observe(
            |_: On<UiClick>, mut next_play_state: ResMut<NextState<PlayState>>| {
                next_play_state.set(PlayState::Playing);
            },
        );
}

fn despawn_pause(mut commands: Commands, pause: Single<Entity, With<PauseMarker>>) {
    commands.entity(*pause).despawn();
}

fn toggle_pause(
    mut commands: Commands,
    state: Res<State<PlayState>>,
    mut next_state: ResMut<NextState<PlayState>>,
) {
    match state.get() {
        PlayState::Paused => next_state.set(PlayState::Playing),
        PlayState::Playing => {
            commands.trigger(Save);
            next_state.set(PlayState::Paused);
        }
        _ => {}
    }
}
pub fn check_save_state(save: &SaveData) -> [bool; 10] {
    let mut out: [bool; 10] = [false; 10];

    out[0] = save.light.unlocked.contains(&LightColor::Green);
    out[1] = save.light.unlocked.contains(&LightColor::Purple);
    out[2] = save.light.unlocked.contains(&LightColor::White);
    out[3] = save.light.unlocked.contains(&LightColor::Blue);
    out[4] = save.level.0[0].complete;
    out[5] = save.level.0[5].complete;
    out[6] = save
        .level
        .0
        .iter()
        .map(|lvl| lvl.complete as i32)
        .sum::<i32>()
        >= 5;
    out[7] = save.level.0.iter().any(|lvl| lvl.solutions_used.len() >= 2);

    static DESIRED_COUNTS: [usize; 15] = [1, 1, 0, 2, 2, 2, 2, 1, 2, 2, 0, 2, 3, 1, 4];
    out[8] = save.level.0.iter().any(|lvl| {
        lvl.solutions_used
            .iter()
            .any(|sol| sol.light_order.len() < DESIRED_COUNTS[lvl.sorted_level_index])
    });

    let official_sols: [Vec<Vec<LightColor>>; 15] = [
        vec![vec![LightColor::Green]],
        vec![vec![LightColor::Green]],
        vec![vec![]],
        vec![
            vec![LightColor::Green, LightColor::Purple],
            vec![LightColor::Purple, LightColor::Green],
        ],
        vec![
            vec![LightColor::Green, LightColor::Purple],
            vec![LightColor::Purple, LightColor::Green],
        ],
        vec![
            vec![LightColor::Green, LightColor::Purple],
            vec![LightColor::Purple, LightColor::Green],
        ],
        vec![vec![LightColor::Green, LightColor::Purple]],
        vec![vec![LightColor::Green], vec![LightColor::Purple]],
        vec![vec![LightColor::Purple, LightColor::Green]],
        vec![vec![LightColor::Green, LightColor::Purple]],
        vec![vec![]],
        vec![vec![LightColor::White, LightColor::Purple]],
        vec![vec![
            LightColor::White,
            LightColor::Purple,
            LightColor::Green,
        ]],
        vec![vec![LightColor::Blue]],
        vec![
            vec![
                LightColor::Blue,
                LightColor::White,
                LightColor::Purple,
                LightColor::Green,
            ],
            vec![LightColor::White, LightColor::Purple, LightColor::Green],
            vec![LightColor::White, LightColor::Green, LightColor::Purple],
        ],
    ];
    out[9] = save.level.0.iter().any(|lvl| {
        lvl.solutions_used.iter().any(|sol| {
            if sol.light_order.len() > DESIRED_COUNTS[lvl.sorted_level_index] {
                return false;
            }
            official_sols[lvl.sorted_level_index]
                .iter()
                .all(|official| sol.light_order != *official)
        })
    });

    out
}
