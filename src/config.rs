use bevy::prelude::KeyCode;
use bevy::prelude::*;
use serde::Deserialize;

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let config: Config = match std::fs::read_to_string("Lightborne.toml") {
            Ok(contents) => toml::from_str(&contents).expect("Failed to parse Lightborne.toml"),
            Err(_) => Config::default(),
        };
        app.insert_resource(config);
    }
}

#[derive(Deserialize, Resource)]
pub struct Config {
    pub level_config: LevelConfig,
    pub debug_config: DebugConfig,
    pub controls_config: ControlsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            level_config: LevelConfig {
                level_path: "levels/lightborne.ldtk".into(),
            },
            debug_config: DebugConfig::default(),
            controls_config: ControlsConfig {
                // Movement
                key_up: KeyCode::KeyW,
                key_down: KeyCode::KeyS,
                key_left: KeyCode::KeyA,
                key_right: KeyCode::KeyD,
                key_jump: KeyCode::Space,
            },
        }
    }
}

#[derive(Deserialize, Default)]
pub struct DebugConfig {
    pub ui: bool,
    pub unlock_levels: bool,
}

#[derive(Deserialize)]
pub struct LevelConfig {
    pub level_path: String,
}

#[derive(Deserialize)]
pub struct ControlsConfig {
    // Movement
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_right: KeyCode,
    pub key_left: KeyCode,
    pub key_jump: KeyCode,
}
