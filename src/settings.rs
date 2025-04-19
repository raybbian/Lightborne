use std::ops::RangeInclusive;

use bevy::audio::Volume;
use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use enum_map::{enum_map, Enum, EnumMap};

use crate::camera::handle_move_camera;
use crate::shared::{GameState, UiState};
use crate::sound::{BgmTrack, ChangeBgmEvent};

pub struct SettingsPlugin;

#[derive(Component)]
struct SettingsUiMarker;

#[derive(Debug, Clone)]
pub struct Setting {
    name: String,
    variant: SettingVariant,
}

impl<T: Clone> SettingValue<T> {
    fn from_default(default: T) -> Self {
        Self {
            value: default.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingValue<T> {
    value: T,
}
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SettingVariant {
    Slider {
        value: SettingValue<f32>,
        range: RangeInclusive<f32>,
        unit: String,
    },
}

impl Setting {
    fn new_slider(name: String, value: f32, range: RangeInclusive<f32>, unit: String) -> Self {
        Self {
            name,
            variant: SettingVariant::Slider {
                value: SettingValue::from_default(value),
                range,
                unit,
            },
        }
    }
}

#[derive(Resource)]
pub struct Settings(EnumMap<SettingName, Setting>);

#[derive(Component, Debug, Clone, PartialEq, Eq, Copy)]
pub struct SettingsIndex(usize);

#[derive(Component, Debug, Clone)]
pub struct SliderButton(f32);

#[derive(Component)]
pub struct SettingParentMarker(SettingName);

#[derive(Event)]
pub struct RedrawSetting(SettingName);

#[derive(Event)]
pub struct UpdateSetting(SettingName);

#[derive(Component, Debug, Clone, PartialEq, Eq, Copy, Enum)]
pub enum SettingName {
    Volume,
}

fn init_settings() -> Settings {
    // Settings(vec![Setting::new_slider(
    //     "Volume".to_owned(),
    //     100.0,
    //     0.0..=100.0,
    //     "%".to_owned(),
    // )])
    Settings(enum_map! {
        SettingName::Volume => Setting::new_slider(
            "Volume".to_owned(),
            100.0,
            0.0..=100.0,
            "%".to_owned(),
        )
    })
}

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            switch_to_settings.run_if(input_just_pressed(KeyCode::Comma)),
        )
        .insert_resource(init_settings())
        .add_event::<RedrawSetting>()
        .add_event::<UpdateSetting>()
        .add_systems(
            FixedUpdate,
            (
                spawn_settings.run_if(in_state(UiState::Settings)),
                handle_slider_buttons.run_if(in_state(UiState::Settings)),
                despawn_settings
                    .after(handle_move_camera)
                    .run_if(not(in_state(UiState::Settings))),
                (redraw_setting, update_setting)
                    .after(handle_slider_buttons)
                    .run_if(in_state(UiState::Settings)),
            ),
        );
    }
}

fn switch_to_settings(
    mut next_ui_state: ResMut<NextState<UiState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    next_game_state.set(GameState::Ui);
    next_ui_state.set(UiState::Settings);
}

fn spawn_settings(
    mut commands: Commands,
    level_select_ui_query: Query<Entity, With<SettingsUiMarker>>,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
    mut ev_change_bgm: EventWriter<ChangeBgmEvent>,
) {
    if level_select_ui_query.get_single().is_ok() {
        return;
    }
    let font = TextFont {
        font: asset_server.load("fonts/Munro.ttf"),
        ..default()
    };

    ev_change_bgm.send(ChangeBgmEvent(BgmTrack::None));

    let setting_nodes = (0..settings.0.len())
        .map(|i| {
            commands
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Auto,
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    SettingParentMarker(SettingName::from_usize(i)),
                ))
                .with_children(|parent| {
                    spawn_setting_children(parent, SettingName::from_usize(i), &settings, &font);
                })
                .id()
        })
        .collect::<Vec<_>>();
    commands
        .spawn((
            SettingsUiMarker,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Interaction::None,
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("Settings"), font.clone().with_font_size(36.)));
            parent
                .spawn(Node {
                    width: Val::Percent(50.),
                    padding: UiRect::all(Val::Px(16.0)),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,

                    ..default()
                })
                .add_children(&setting_nodes);
        });
}

fn spawn_setting_children(
    parent: &mut ChildBuilder,
    settings_index: SettingName,
    settings: &Settings,
    font: &TextFont,
) {
    let setting = &settings.0[settings_index];
    parent.spawn((Text::new(&setting.name), font.clone().with_font_size(24.0)));
    parent
        .spawn(Node {
            width: Val::Auto,
            height: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|parent| match &setting.variant {
            SettingVariant::Slider { value, unit, .. } => {
                let slider_button_bundle = (
                    Node {
                        align_content: AlignContent::Center,
                        padding: UiRect {
                            left: Val::Px(4.0),
                            right: Val::Px(4.0),
                            top: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                        },
                        ..default()
                    },
                    Button,
                    font.clone().with_font_size(24.0),
                    settings_index,
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                );
                parent.spawn((
                    slider_button_bundle.clone(),
                    Text::new("-10"),
                    SliderButton(-10.0),
                ));
                parent.spawn((
                    slider_button_bundle.clone(),
                    Text::new("-1"),
                    SliderButton(-1.0),
                ));

                parent
                    .spawn((Node {
                        width: Val::Px(60.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },))
                    .with_child((
                        Text::new(format!("{}{}", value.value, unit)),
                        font.clone().with_font_size(24.0),
                    ));

                parent.spawn((
                    slider_button_bundle.clone(),
                    Text::new("+1"),
                    SliderButton(1.0),
                ));
                parent.spawn((
                    slider_button_bundle.clone(),
                    Text::new("+10"),
                    SliderButton(10.0),
                ));
            }
        });
}

fn despawn_settings(
    mut commands: Commands,
    mut level_select_ui_query: Query<Entity, With<SettingsUiMarker>>,
) {
    let Ok(entity) = level_select_ui_query.get_single_mut() else {
        return;
    };

    commands.entity(entity).despawn_recursive();
}

#[allow(clippy::type_complexity)]
fn handle_slider_buttons(
    interaction_query: Query<
        (&Interaction, &SliderButton, &SettingName),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: ResMut<Settings>,
    mut redraw_ev: EventWriter<RedrawSetting>,
    mut update_ev: EventWriter<UpdateSetting>,
) {
    for (interaction, slider_button, setting_name) in interaction_query.iter() {
        if interaction == &Interaction::Pressed {
            let setting = &mut settings.0[*setting_name];
            let SettingVariant::Slider {
                ref mut value,
                ref range,
                ..
            } = setting.variant;

            value.value += slider_button.0;
            value.value = value.value.clamp(*range.start(), *range.end());

            redraw_ev.send(RedrawSetting(*setting_name));
            update_ev.send(UpdateSetting(*setting_name));
        }
    }
}

fn redraw_setting(
    mut commands: Commands,
    mut ev: EventReader<RedrawSetting>,
    setting_parents: Query<(Entity, &SettingParentMarker)>,
    settings: Res<Settings>,
    asset_server: Res<AssetServer>,
) {
    let font = TextFont {
        font: asset_server.load("fonts/Munro.ttf"),
        ..default()
    };
    for RedrawSetting(settings_index) in ev.read() {
        let Some(setting_parent_id) = setting_parents
            .iter()
            .find(|(_, setting_parent)| setting_parent.0 == *settings_index)
            .map(|(entity, _)| entity)
        else {
            continue;
        };
        commands
            .entity(setting_parent_id)
            .despawn_descendants()
            .with_children(|parent| {
                spawn_setting_children(parent, *settings_index, &settings, &font);
            });
    }
}

fn update_setting(
    mut ev: EventReader<UpdateSetting>,
    settings: Res<Settings>,
    mut global_volume: ResMut<GlobalVolume>,
) {
    for UpdateSetting(setting_name) in ev.read() {
        let setting = &settings.0[SettingName::Volume];
        match setting_name {
            SettingName::Volume => {
                let SettingVariant::Slider { ref value, .. } = setting.variant;
                global_volume.volume = Volume::new(value.value / 100.0);
            }
        }
    }
}
