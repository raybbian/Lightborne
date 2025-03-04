use std::time::Duration;

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_ecs_ldtk::{ldtk::Level, prelude::*, systems::process_ldtk_levels, LevelIid};
use sensor::{color_sensors, reset_light_sensors, update_light_sensors, LightSensorBundle};

use crate::{
    camera::{CameraMoveEvent, CAMERA_ANIMATION_SECS, CAMERA_HEIGHT, CAMERA_WIDTH},
    level_select::handle_level_selection,
    light::{segments::simulate_light_sources, LightColor},
    player::{LdtkPlayerBundle, PlayerMarker},
    shared::{GameState, ResetLevel},
};
use crystal::CrystalPlugin;
use entity::{SemiSolidPlatformBundle, SpikeBundle};
use setup::LevelSetupPlugin;
use start_flag::{init_start_marker, StartFlagBundle};
use walls::{spawn_wall_collision, WallBundle};

pub mod crystal;
pub mod entity;
pub mod sensor;
mod setup;
pub mod start_flag;
mod walls;

/// [`Plugin`] that handles everything related to the level.
pub struct LevelManagementPlugin;

impl Plugin for LevelManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            .add_plugins(LevelSetupPlugin)
            .add_plugins(CrystalPlugin)
            .init_resource::<CurrentLevel>()
            .register_ldtk_entity::<LdtkPlayerBundle>("Lyra")
            .register_ldtk_entity::<LightSensorBundle>("Sensor")
            .register_ldtk_entity::<StartFlagBundle>("Start")
            .register_ldtk_int_cell_for_layer::<WallBundle>("Terrain", 1)
            .register_ldtk_int_cell_for_layer::<SpikeBundle>("Terrain", 2)
            .register_ldtk_int_cell_for_layer::<SemiSolidPlatformBundle>("Terrain", 15)
            .add_systems(
                PreUpdate,
                (spawn_wall_collision, init_start_marker, color_sensors)
                    .in_set(LevelSystems::Processing),
            )
            .add_systems(Update, reset_light_sensors.in_set(LevelSystems::Reset))
            .add_systems(
                FixedUpdate,
                (
                    switch_level.after(handle_level_selection),
                    update_light_sensors.after(simulate_light_sources),
                ),
            )
            .configure_sets(
                PreUpdate,
                LevelSystems::Processing.after(process_ldtk_levels),
            )
            .configure_sets(Update, LevelSystems::Reset.run_if(on_event::<ResetLevel>))
            .configure_sets(
                FixedUpdate,
                LevelSystems::Reset.run_if(on_event::<ResetLevel>),
            )
            .configure_sets(
                Update,
                LevelSystems::Simulation.run_if(in_state(GameState::Playing)),
            )
            .configure_sets(
                FixedUpdate,
                LevelSystems::Simulation.run_if(in_state(GameState::Playing)),
            );
    }
}

/// [`Resource`] that holds the `level_iid` of the current level.
#[derive(Default, Debug, Resource)]
pub struct CurrentLevel {
    pub level_iid: LevelIid,
    pub world_box: Rect,
    pub allowed_colors: Vec<LightColor>,
}

/// [`SystemSet`] used to distinguish different types of systems
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LevelSystems {
    /// Systems used to simulate game logic in [`Update`]
    Simulation,
    /// Systems used to process Ldtk Entities after they spawn in [`PreUpdate`]
    Processing,
    /// Systems used to clean up the level when the room switches or the player respawns
    Reset,
}

// TODO: do not use levels.clone() here!
pub fn get_ldtk_level_data(
    ldtk_assets: Res<Assets<LdtkProject>>,
    query_ldtk: Query<&LdtkProjectHandle>,
) -> Result<Vec<Level>, String> {
    let Ok(ldtk_handle) = query_ldtk.get_single() else {
        return Err("Could not find LDTK project handle!".into());
    };
    let Some(ldtk_project) = ldtk_assets.get(ldtk_handle) else {
        return Err("Failed to get LdtkProject asset!".into());
    };
    Ok(ldtk_project.json_data().levels.clone())
}

/// [`System`] that will run on [`Update`] to check if the Player has moved to another level. If
/// the player has, then a MoveCameraEvent is sent. After the animation is finished, the Camera
/// handling code will send a LevelSwitch event that will notify other systems to cleanup the
/// levels.
#[allow(clippy::too_many_arguments)]
pub fn switch_level(
    q_player: Query<&Transform, With<PlayerMarker>>,
    mut level_selection: ResMut<LevelSelection>,
    ldtk_projects: Query<&LdtkProjectHandle>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut current_level: ResMut<CurrentLevel>,
    on_level_switch_finish_cb: Local<OnFinishLevelSwitchCallback>,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
    mut ev_level_switch: EventWriter<ResetLevel>,
) {
    let Ok(player_transform) = q_player.get_single() else {
        return;
    };
    let Ok(ldtk_levels) = get_ldtk_level_data(ldtk_project_assets, ldtk_projects) else {
        return;
    };
    for level in ldtk_levels {
        let world_box = Rect::new(
            level.world_x as f32,
            -level.world_y as f32,
            (level.world_x + level.px_wid) as f32,
            (-level.world_y - level.px_hei) as f32,
        );

        if world_box.contains(player_transform.translation.xy()) {
            if current_level.level_iid.as_str() != level.iid {
                // relies on camera to reset the state back to switching??
                if !current_level.level_iid.to_string().is_empty() {
                    next_game_state.set(GameState::SwitchAnimation);

                    let (x_min, x_max) = (
                        world_box.min.x + CAMERA_WIDTH * 0.5,
                        world_box.max.x - CAMERA_WIDTH * 0.5,
                    );
                    let (y_min, y_max) = (
                        world_box.min.y + CAMERA_HEIGHT * 0.5,
                        world_box.max.y - CAMERA_HEIGHT * 0.5,
                    );

                    let new_pos = Vec2::new(
                        player_transform.translation.x.max(x_min).min(x_max),
                        player_transform.translation.y.max(y_min).min(y_max),
                    );

                    ev_move_camera.send(CameraMoveEvent::Animated {
                        to: new_pos,
                        duration: Duration::from_secs_f32(CAMERA_ANIMATION_SECS),
                        callback: Some(on_level_switch_finish_cb.0),
                        ease_fn: EaseFunction::SineInOut,
                    });
                } else {
                    ev_level_switch.send(ResetLevel::Switching);
                }

                let allowed_colors = level
                    .iter_enums_field("AllowedColors")
                    .expect("AllowedColors should be enum array level field.")
                    .map(|color_str| color_str.into())
                    .collect::<Vec<LightColor>>();

                *current_level = CurrentLevel {
                    level_iid: LevelIid::new(level.iid.clone()),
                    world_box,
                    allowed_colors,
                };
                *level_selection = LevelSelection::iid(current_level.level_iid.clone());
            }
            break;
        }
    }
}

pub struct OnFinishLevelSwitchCallback(pub SystemId);

impl FromWorld for OnFinishLevelSwitchCallback {
    fn from_world(world: &mut World) -> Self {
        OnFinishLevelSwitchCallback(world.register_system(on_finish_level_switch))
    }
}

pub fn on_finish_level_switch(
    mut next_game_state: ResMut<NextState<GameState>>,
    mut ev_reset_level: EventWriter<ResetLevel>,
) {
    next_game_state.set(GameState::Playing);
    ev_reset_level.send(ResetLevel::Switching);
}
