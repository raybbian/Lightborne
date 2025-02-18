use bevy::{prelude::*, render::view::RenderLayers};
use bevy_ecs_ldtk::{prelude::*, systems::process_ldtk_levels};

use crate::{
    player::{LdtkPlayerBundle, PlayerMarker},
    shared::{GameState, ResetLevel},
};
use crystal::CrystalPlugin;
use platform::{MovingPlatformBundle, PlatformPlugin};
use entity::SpikeBundle;
use misc::{init_start_marker, ButtonBundle, StartFlagBundle};
use setup::LevelSetupPlugin;
use walls::{spawn_wall_collision, WallBundle};

pub mod crystal;
pub mod entity;
pub mod misc;
pub mod platform;
mod setup;
mod walls;

/// [`Plugin`] that handles everything related to the level.
pub struct LevelManagementPlugin;

impl Plugin for LevelManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            .add_plugins(LevelSetupPlugin)
            .add_plugins(CrystalPlugin)
            .add_plugins(PlatformPlugin)
            .init_resource::<CurrentLevel>()
            .register_ldtk_entity::<LdtkPlayerBundle>("Lyra")
            .register_ldtk_entity::<ButtonBundle>("Button")
            .register_ldtk_entity::<StartFlagBundle>("Start")
            .register_ldtk_entity::<MovingPlatformBundle>("MovingPlatform")
            .register_ldtk_int_cell_for_layer::<WallBundle>("Terrain", 1)
            .register_ldtk_int_cell_for_layer::<SpikeBundle>("Terrain", 2)
            .add_systems(
                PreUpdate,
                (spawn_wall_collision, init_start_marker).in_set(LevelSystems::Processing),
            )
            .add_systems(Startup, spawn_background)
            .add_systems(Update, switch_level)
            .configure_sets(
                PreUpdate,
                LevelSystems::Processing.after(process_ldtk_levels),
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
#[derive(Default, Resource)]
pub struct CurrentLevel {
    pub level_iid: LevelIid,
    pub level_entity: Option<Entity>,
    pub world_box: Rect,
}

/// [`SystemSet`] used to distinguish different types of systems
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LevelSystems {
    /// Systems used to simulate game logic in [`Update`]
    Simulation,
    /// Systems used to process Ldtk Entities after they spawn in [`PreUpdate`]
    Processing,
}

#[derive(Component)]
pub struct BackgroundMarker;

fn spawn_background(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Sprite {
            image: asset_server.load("levels/background.png"),
            color: Color::srgb(0.3, 0.3, 0.3),
            ..default()
        },
        Transform::default(),
        Visibility::Visible,
        BackgroundMarker,
        RenderLayers::layer(1),
    ));
}

/// [`System`] that will run on [`Update`] to check if the Player has moved to another level. If
/// the player has, then a [`LevelSwitchEvent`] will be sent out to notify other systems.
fn switch_level(
    q_player: Query<&Transform, With<PlayerMarker>>,
    q_level: Query<(Entity, &LevelIid)>,
    mut level_selection: ResMut<LevelSelection>,
    ldtk_projects: Query<&LdtkProjectHandle>,
    ldtk_project_assets: Res<Assets<LdtkProject>>,
    mut ev_reset_level: EventWriter<ResetLevel>,
    mut current_level: ResMut<CurrentLevel>,
) {
    let Ok(transform) = q_player.get_single() else {
        return;
    };
    for (entity, level_iid) in q_level.iter() {
        let ldtk_project = ldtk_project_assets
            .get(ldtk_projects.single())
            .expect("Project should be loaded if level has spawned");

        let level = ldtk_project
            .get_raw_level_by_iid(&level_iid.to_string())
            .expect("Spawned level should exist in Ldtk project");

        let world_box = Rect::new(
            level.world_x as f32,
            -level.world_y as f32,
            (level.world_x + level.px_wid) as f32,
            (-level.world_y - level.px_hei) as f32,
        );

        if world_box.contains(transform.translation.xy()) {
            if current_level.level_iid != *level_iid {
                if !current_level.level_iid.get().is_empty() {
                    ev_reset_level.send(ResetLevel::Switching);
                }

                *current_level = CurrentLevel {
                    level_iid: level_iid.clone(),
                    level_entity: Some(entity),
                    world_box,
                };
                *level_selection = LevelSelection::iid(level_iid.to_string());
            }
            break;
        }
    }
}
