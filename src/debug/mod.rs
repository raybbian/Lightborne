use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::schedule::{LogLevel, ScheduleBuildSettings},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_ecs_ldtk::LevelIid;
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiPlugin},
    egui,
};
use bevy_rapier2d::render::RapierDebugRenderPlugin;

use crate::config::Config;

pub struct DebugPlugin {
    pub physics: bool,
    pub frame_time: bool,
    pub ui: bool,
    pub ambiguity: bool,
}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if self.ui {
            app.add_plugins(EguiPlugin)
                .add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
                .add_systems(Last, debug_ui);
        }

        if self.physics {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }
        if self.frame_time {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }
        if self.ambiguity {
            app.edit_schedule(Update, |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..default()
                });
            });
            app.edit_schedule(PreUpdate, |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..default()
                });
            });
            app.edit_schedule(PostUpdate, |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..default()
                });
            });
            app.edit_schedule(FixedUpdate, |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..default()
                });
            });
        }
    }
}

impl Default for DebugPlugin {
    fn default() -> Self {
        DebugPlugin {
            ambiguity: false,
            physics: false,
            frame_time: true,
            ui: true,
        }
    }
}

pub fn debug_ui(world: &mut World) {
    let config = world.resource::<Config>();
    if !config.debug_config.ui {
        return;
    }

    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();
    // TODO: put this back in?
    // let Some(level_entity) = world.resource::<CurrentLevel>().level_entity else {
    //     return;
    // };

    egui::Window::new("UI").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Current Level");
            // TODO: put this back in?
            // ui_for_entity_with_children(world, level_entity, ui);

            ui.heading("Loaded Levels");
            let mut query = world.query::<&LevelIid>();
            let levels: Vec<&LevelIid> = query.iter(world).collect();
            for level in levels {
                ui.label(level.get());
            }
        });
    });
}
