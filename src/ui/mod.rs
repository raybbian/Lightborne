use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    audio::Volume,
    prelude::*,
    ui::ui_focus_system,
};

use follow::TargetFollowingPlugin;
// use level_select::LevelSelectPlugin;
use pause::PausePlugin;
use tooltip::TooltipPlugin;

use crate::{
    asset::LoadResource,
    shared::{GameState, PlayState},
    ui::{
        leaderboard::LeaderboardUiPlugin, level_select::LevelSelectPlugin, light::LightUiPlugin,
        loading::LoadingUiPlugin, scroll::ScrollPlugin, settings::SettingsPlugin,
        speedrun::SpeedrunTimerPlugin, start_menu::StartMenuPlugin,
    },
};

pub mod follow;
mod leaderboard;
pub mod level_select;
mod light;
mod loading;
mod pause;
mod scroll;
pub mod settings;
pub mod speedrun;
mod start_menu;
pub mod tooltip;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiSfx>();
        app.load_resource::<UiSfx>();
        app.add_plugins(TargetFollowingPlugin);
        app.add_plugins(TooltipPlugin);
        app.add_plugins(ScrollPlugin);
        app.add_plugins(LeaderboardUiPlugin);
        app.add_plugins(PausePlugin);
        app.add_plugins(LoadingUiPlugin);
        app.add_plugins(SpeedrunTimerPlugin);
        app.add_plugins(StartMenuPlugin);
        app.add_plugins(SettingsPlugin);
        app.add_plugins(LevelSelectPlugin);
        app.add_plugins(LightUiPlugin);
        app.add_systems(Update, change_scaling);
        app.add_systems(
            PreUpdate,
            button_sfx
                .after(ui_focus_system)
                .run_if(in_state(GameState::Ui).or(in_state(PlayState::Paused))),
        );
        app.add_systems(PreUpdate, trigger_interaction_events.after(button_sfx));

        embedded_asset!(app, "Outfit-Medium.ttf");
        app.insert_resource(UiFont {
            font: load_embedded_asset!(app, "Outfit-Medium.ttf"),
        });
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct UiSfx {
    #[dependency]
    on_click: Handle<AudioSource>,
    #[dependency]
    on_hover: Handle<AudioSource>,
}

impl FromWorld for UiSfx {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            on_click: asset_server.load("sfx/click.wav"),
            on_hover: asset_server.load("sfx/hover.wav"),
        }
    }
}

#[derive(Resource)]
pub struct UiFont {
    font: Handle<Font>,
}

impl UiFont {
    pub fn text_font(&self) -> TextFont {
        TextFont {
            font: self.font.clone(),
            ..default()
        }
    }
}

#[derive(Component)]
pub struct UiButton;

pub fn button_sfx(
    mut commands: Commands,
    q_button: Query<&Interaction, (With<UiButton>, Changed<Interaction>)>,
    ui_sfx: Res<UiSfx>,
) {
    for interaction in q_button.iter() {
        match *interaction {
            Interaction::Pressed => {
                commands.spawn((
                    AudioPlayer::new(ui_sfx.on_click.clone()),
                    PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5)),
                ));
            }
            Interaction::Hovered => {
                commands.spawn((
                    AudioPlayer::new(ui_sfx.on_hover.clone()),
                    PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.5)),
                ));
            }
            _ => (),
        }
    }
}

// TODO: switch to On<Pointer<Press>>
#[derive(EntityEvent)]
pub struct UiClick {
    entity: Entity,
}

pub fn trigger_interaction_events(
    mut commands: Commands,
    q_interactions: Query<(Entity, &Interaction), Changed<Interaction>>,
) {
    for (entity, interaction) in q_interactions.iter() {
        if *interaction == Interaction::Pressed {
            commands.trigger(UiClick { entity });
        }
    }
}

pub struct UiFontSize;

impl UiFontSize {
    pub const HEADER: f32 = 64.;
    pub const BUTTON: f32 = 48.;
    pub const TEXT: f32 = 32.;
}

fn change_scaling(input: Res<ButtonInput<KeyCode>>, mut ui_scale: ResMut<UiScale>) {
    if input.just_pressed(KeyCode::Equal) {
        let scale = (ui_scale.0 + 0.25).min(8.);
        ui_scale.0 = scale;
    }
    if input.just_pressed(KeyCode::Minus) {
        let scale = (ui_scale.0 - 0.25).max(1. / 4.);
        ui_scale.0 = scale;
    }
}
