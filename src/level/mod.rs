use std::time::Duration;

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_ecs_ldtk::{ldtk::Level, prelude::*, systems::process_ldtk_levels, LevelIid};
use decoration::DecorationPlugin;
use egg::EggPlugin;
use enum_map::{enum_map, EnumMap};
use level_completion::LevelCompletionPlugin;
use merge_tile::spawn_merged_tiles;
use mirror::MirrorPlugin;
use semisolid::SemiSolidPlugin;
use sensor::LightSensorPlugin;
use shard::CrystalShardPlugin;

use crate::{
    camera::{
        camera_position_from_level, CameraControlType, CameraMoveEvent, CAMERA_ANIMATION_SECS,
    },
    light::LightColor,
    player::{LdtkPlayerBundle, PlayerMarker},
    shared::{AnimationState, GameState, ResetLevel},
    sound::{BgmTrack, ChangeBgmEvent},
    ui::level_select::handle_level_selection,
};
use crystal::CrystalPlugin;
use entity::SpikeBundle;
use platform::PlatformPlugin;
use setup::LevelSetupPlugin;
use start_flag::{init_start_marker, StartFlagBundle};
use walls::{Wall, WallBundle};

pub mod crystal;
mod decoration;
mod egg;
pub mod entity;
mod level_completion;
mod merge_tile;
pub mod mirror;
pub mod platform;
mod semisolid;
pub mod sensor;
mod setup;
pub mod shard;
pub mod start_flag;
mod walls;

/// [`Plugin`] that handles everything related to the level.
pub struct LevelManagementPlugin;

impl Plugin for LevelManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            .add_plugins(LevelSetupPlugin)
            .add_plugins(CrystalPlugin)
            .add_plugins(PlatformPlugin)
            .add_plugins(CrystalShardPlugin)
            .add_plugins(LightSensorPlugin)
            .add_plugins(SemiSolidPlugin)
            .add_plugins(MirrorPlugin)
            .add_plugins(EggPlugin)
            .add_plugins(LevelCompletionPlugin)
            .add_plugins(DecorationPlugin)
            .init_resource::<CurrentLevel>()
            .register_ldtk_entity::<LdtkPlayerBundle>("Lyra")
            .register_ldtk_entity::<StartFlagBundle>("Start")
            .register_ldtk_int_cell_for_layer::<WallBundle>("Terrain", 1)
            .register_ldtk_int_cell_for_layer::<SpikeBundle>("Terrain", 2)
            .add_systems(
                PreUpdate,
                (spawn_merged_tiles::<Wall>, init_start_marker).in_set(LevelSystems::Processing),
            )
            .add_systems(
                FixedUpdate,
                (
                    switch_level,
                    set_bgm_from_current_level.in_set(LevelSystems::Simulation),
                )
                    .chain()
                    .after(handle_level_selection),
            )
            .configure_sets(
                PreUpdate,
                LevelSystems::Processing.after(process_ldtk_levels),
            )
            .configure_sets(
                Update,
                LevelSystems::Reset
                    .run_if(on_event::<ResetLevel>)
                    .before(LevelSystems::Simulation),
            )
            .configure_sets(
                FixedUpdate,
                LevelSystems::Reset
                    .run_if(on_event::<ResetLevel>)
                    .before(LevelSystems::Simulation),
            )
            .configure_sets(
                Update,
                LevelSystems::Simulation
                    .run_if(in_state(GameState::Playing).or(in_state(AnimationState::Shard))),
            )
            .configure_sets(
                FixedUpdate,
                LevelSystems::Simulation
                    .run_if(in_state(GameState::Playing).or(in_state(AnimationState::Shard))),
            );
    }
}

/// [`Resource`] that holds the `level_iid` of the current level.
#[derive(Default, Debug, Resource)]
pub struct CurrentLevel {
    pub level_iid: LevelIid,
    pub level_box: Rect,
    pub allowed_colors: EnumMap<LightColor, bool>,
}

/// [`SystemSet`] used to distinguish different types of systems
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LevelSystems {
    /// Systems used to simulate game logic
    Simulation,
    /// Systems used to process Ldtk Entities after they spawn in [`PreUpdate`]
    Processing,
    /// Systems used to clean up the level when the room switches or the player respawns
    Reset,
}

pub fn get_ldtk_level_data<'ldtk>(
    ldtk_assets: &'ldtk Assets<LdtkProject>,
    ldtk_handle: &LdtkProjectHandle,
) -> Result<&'ldtk Vec<Level>, String> {
    let Some(ldtk_project) = ldtk_assets.get(ldtk_handle) else {
        return Err("Failed to get LdtkProject asset!".into());
    };
    Ok(&ldtk_project.json_data().levels)
}

pub fn level_box_from_level(level: &Level) -> Rect {
    Rect::new(
        level.world_x as f32,
        -level.world_y as f32,
        (level.world_x + level.px_wid) as f32,
        (-level.world_y - level.px_hei) as f32,
    )
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
    mut next_anim_state: ResMut<NextState<AnimationState>>,
    mut current_level: ResMut<CurrentLevel>,
    on_level_switch_finish_cb: Local<OnFinishLevelSwitchCallback>,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
    mut ev_level_switch: EventWriter<ResetLevel>,
) {
    let Ok(player_transform) = q_player.get_single() else {
        return;
    };
    let Ok(ldtk_handle) = ldtk_projects.get_single() else {
        return;
    };
    let Ok(ldtk_levels) = get_ldtk_level_data(ldtk_project_assets.into_inner(), ldtk_handle) else {
        return;
    };
    for level in ldtk_levels {
        let level_box = level_box_from_level(level);

        if level_box.contains(player_transform.translation.xy()) {
            if current_level.level_iid.as_str() != level.iid {
                // relies on camera to reset the state back to switching??
                if !current_level.level_iid.to_string().is_empty() {
                    next_game_state.set(GameState::Animating);
                    next_anim_state.set(AnimationState::Switch);

                    ev_move_camera.send(CameraMoveEvent {
                        to: camera_position_from_level(
                            level_box,
                            player_transform.translation.xy(),
                        ),
                        variant: CameraControlType::Animated {
                            duration: Duration::from_secs_f32(CAMERA_ANIMATION_SECS),
                            callback: Some(on_level_switch_finish_cb.0),
                            ease_fn: EaseFunction::SineInOut,
                        },
                    });
                } else {
                    ev_level_switch.send(ResetLevel::Switching);
                }

                let allowed_colors = level
                    .iter_enums_field("AllowedColors")
                    .expect("AllowedColors should be enum array level field.")
                    .map(|color_str| color_str.into())
                    .collect::<Vec<LightColor>>();

                let allowed_colors_map = enum_map! {
                    val => allowed_colors.contains(&val),
                };

                *current_level = CurrentLevel {
                    level_iid: LevelIid::new(level.iid.clone()),
                    level_box,
                    allowed_colors: allowed_colors_map,
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

// FIXME: temp code with lots of copied stuff to impl audio changing
pub fn set_bgm_from_current_level(
    current_level: Res<CurrentLevel>,
    mut ev_change_bgm: EventWriter<ChangeBgmEvent>,
    ldtk_projects: Query<&LdtkProjectHandle>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
) {
    let Ok(ldtk_handle) = ldtk_projects.get_single() else {
        return;
    };
    let Ok(ldtk_levels) = get_ldtk_level_data(ldtk_project_assets.into_inner(), ldtk_handle) else {
        return;
    };
    let cur_id = ldtk_levels.iter().find_map(|level| {
        let level_id = level
            .get_string_field("LevelId")
            .expect("Levels should always have a level id!");
        if level_id.is_empty() {
            panic!("Level id for a level should not be empty!");
        }
        if level.iid == current_level.level_iid.as_str() {
            return Some(level_id);
        }
        None
    });

    let new_bgm = match cur_id {
        Some(val) if &val[0..1] == "2" || &val[0..1] == "1" => BgmTrack::MustntStop,
        Some(val) if &val[0..1] == "3" => BgmTrack::Cutscene1Draft,
        Some(val) if &val[0..1] == "4" => BgmTrack::LightInTheDark,
        _ => BgmTrack::None,
    };

    ev_change_bgm.send(ChangeBgmEvent(new_bgm));
}
