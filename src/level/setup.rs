use crate::config::Config;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

pub struct LevelSetupPlugin;

impl Plugin for LevelSetupPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LevelSelection::index(3))
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
                    load_level_neighbors: true,
                },
                level_background: LevelBackground::Nonexistent,
                ..default()
            })
            .add_systems(Startup, setup_level);
    }
}

fn setup_level(mut commands: Commands, asset_server: Res<AssetServer>, config: Res<Config>) {
    commands.insert_resource(LevelSelection::index(config.level_config.level_index));
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load(&config.level_config.level_path).into(),
        ..Default::default()
    });
}
