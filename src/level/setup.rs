use crate::config::Config;
use crate::shared::GameState;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

pub struct LevelSetupPlugin;

impl Plugin for LevelSetupPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LevelSelection::index(8))
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

pub fn setup_level(
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameState>>,
    asset_server: Res<AssetServer>,
    config: Res<Config>,
) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load(&config.level_config.level_path).into(),
        ..Default::default()
    });
    next_game_state.set(GameState::Ui);
}
