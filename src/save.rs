use std::{collections::HashMap, fs, path::Path};

#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::IoTaskPool;
use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    game::lyra::beam::PlayerLightProgress,
    ui::{level_select::LevelProgress, speedrun::SpeedrunTimer},
};

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_save);
        app.insert_resource(CurrentUser("".to_string()));
        app.add_observer(on_save);
    }
}

#[derive(Resource)]
pub struct CurrentUser(pub String);

#[derive(Event)]
pub struct Save;

#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub level: LevelProgress,
    pub light: PlayerLightProgress,
    pub timer: SpeedrunTimer,
}

#[derive(Resource, Serialize, Deserialize, Clone, TypePath, Default)]
pub struct SaveFile {
    pub data: HashMap<String, SaveData>,
}

#[derive(SystemParam)]
pub struct SaveParam<'w> {
    pub save_file: ResMut<'w, SaveFile>,
    pub current_user: Res<'w, CurrentUser>,
}

impl SaveParam<'_> {
    pub fn get_save_data(&self) -> Option<&SaveData> {
        self.save_file.data.get(&self.current_user.0)
    }
}

const SAVE_PATH: &str = "SaveData.toml";

pub fn init_save(mut commands: Commands) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !Path::new(SAVE_PATH).exists() {
            commands.insert_resource(SaveFile::default());
            return;
        }
        let Ok(contents) = fs::read_to_string(SAVE_PATH) else {
            commands.insert_resource(SaveFile::default());
            return;
        };
        let Ok(save_file) = toml::from_str::<SaveFile>(&contents) else {
            commands.insert_resource(SaveFile::default());
            return;
        };

        commands.insert_resource(save_file);
    }
    #[cfg(target_arch = "wasm32")]
    {
        commands.insert_resource(SaveFile::defaut());
    }
}

pub fn on_save(
    _: On<Save>,
    mut save_param: SaveParam,
    level_progress: Res<LevelProgress>,
    light_progress: Res<PlayerLightProgress>,
    speedrun_timer: Res<SpeedrunTimer>,
) {
    let username = save_param.current_user.0.clone();
    let level = level_progress.into_inner().clone();
    let light = light_progress.into_inner().clone();
    let timer = speedrun_timer.into_inner().clone();

    save_param.save_file.data.insert(
        username,
        SaveData {
            level,
            light,
            timer,
        },
    );
    let save_file = save_param.save_file.clone();

    #[cfg(not(target_arch = "wasm32"))]
    {
        IoTaskPool::get()
            .spawn(async move {
                if let Ok(serialized) = toml::to_string_pretty(&save_file) {
                    let _ = fs::write(SAVE_PATH, serialized);
                }
            })
            .detach();
    }
}
