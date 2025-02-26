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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            level_config: LevelConfig {
                level_index: 9,
                level_path: "levels/lightborne.ldtk".into(),
            },
            debug_config: DebugConfig::default(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct DebugConfig {
    pub ui: bool,
}

#[derive(Deserialize)]
pub struct LevelConfig {
    pub level_index: usize,
    pub level_path: String,
}
