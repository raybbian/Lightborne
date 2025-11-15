use bevy::{prelude::*, ui::widget::NodeImageMode};

use crate::{
    asset::LoadResource,
    shared::UiState,
    sound::{BgmTrack, ChangeBgmEvent},
    ui::{UiButton, UiClick, UiFont, UiFontSize},
};

pub struct StartMenuPlugin;

impl Plugin for StartMenuPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<StartMenuAssets>();
        app.load_resource::<StartMenuAssets>();
        app.add_systems(OnEnter(UiState::StartMenu), spawn_start_menu);
        app.add_systems(OnExit(UiState::StartMenu), despawn_start_menu);
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct StartMenuAssets {
    #[dependency]
    background: Handle<Image>,
}

impl FromWorld for StartMenuAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            background: asset_server.load("ui/start.png"),
        }
    }
}

#[derive(Component)]
pub struct StartMenuMarker;

fn spawn_start_menu(mut commands: Commands, ui_font: Res<UiFont>, assets: Res<StartMenuAssets>) {
    info!("Spawning Start Menu!");

    commands.trigger(ChangeBgmEvent(BgmTrack::LevelSelect));

    let container = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(32.0),
            row_gap: Val::Px(32.0),
            padding: UiRect::all(Val::Px(96.)).with_top(Val::Vh(50.)),
            ..default()
        })
        .insert(ImageNode::from(assets.background.clone()).with_mode(NodeImageMode::Stretch))
        .insert(BackgroundColor(Color::BLACK))
        .insert(StartMenuMarker)
        .id();

    commands
        .spawn(Node {
            width: Val::Auto,
            height: Val::Auto,
            ..default()
        })
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .insert(Text::new("Play"))
        .insert(Button)
        .insert(UiButton)
        .insert(ChildOf(container))
        .observe(
            |_: On<UiClick>, mut next_ui_state: ResMut<NextState<UiState>>| {
                next_ui_state.set(UiState::Leaderboard);
            },
        );

    commands
        .spawn(Node {
            width: Val::Auto,
            height: Val::Auto,
            ..default()
        })
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .insert(Text::new("Settings"))
        .insert(Button)
        .insert(UiButton)
        .insert(ChildOf(container))
        .observe(
            |_: On<UiClick>, mut next_ui_state: ResMut<NextState<UiState>>| {
                next_ui_state.set(UiState::Settings);
            },
        );

    commands
        .spawn(Node {
            width: Val::Auto,
            height: Val::Auto,
            ..default()
        })
        .insert(ui_font.text_font().with_font_size(UiFontSize::BUTTON))
        .insert(Text::new("Quit"))
        .insert(Button)
        .insert(UiButton)
        .insert(ChildOf(container))
        .observe(|_: On<UiClick>, mut ev_app_exit: MessageWriter<AppExit>| {
            ev_app_exit.write(AppExit::Success);
        });
}

fn despawn_start_menu(mut commands: Commands, start_menu: Single<Entity, With<StartMenuMarker>>) {
    info!("Despawning Start Menu!");

    commands.entity(*start_menu).despawn();
}
