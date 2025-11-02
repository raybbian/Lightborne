use std::time::Duration;

use bevy::prelude::*;

use crate::{
    game::{
        defs::shard::CrystalShardMods,
        lyra::{beam::PlayerLightInventory, Lyra},
        LevelSystems,
    },
    ldtk::{LdtkLevelParam, LevelExt},
    shared::GameState,
    ui::tooltip::TooltipSpawner,
};

const PLAYER_STUCK_TOOLTIP_DELAY_SECS: u64 = 8;

pub struct HintRestartPlugin;

impl Plugin for HintRestartPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HintRestartTimer>();
        app.add_systems(OnEnter(GameState::InGame), reset_restart_hint_timer);
        app.add_systems(Update, hint_restart_button.in_set(LevelSystems::Simulation));
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct HintRestartTimer(Timer);

impl Default for HintRestartTimer {
    fn default() -> Self {
        Self(Timer::new(
            Duration::from_secs(PLAYER_STUCK_TOOLTIP_DELAY_SECS),
            TimerMode::Once,
        ))
    }
}

pub fn reset_restart_hint_timer(mut hint_reset_timer: ResMut<HintRestartTimer>) {
    hint_reset_timer.reset();
    hint_reset_timer.unpause()
}

pub fn hint_restart_button(
    mut tooltip_spawner: TooltipSpawner,
    mut triggered: ResMut<HintRestartTimer>,
    lyra: Single<(Entity, &PlayerLightInventory), With<Lyra>>,
    time: Res<Time>,
    shard_mods: Res<CrystalShardMods>,
    ldtk_level_param: LdtkLevelParam,
) {
    let (lyra, inventory) = lyra.into_inner();

    let allowed_colors = ldtk_level_param
        .cur_level()
        .expect("Cur level must exist")
        .raw()
        .allowed_colors();

    let has_color = allowed_colors.iter().any(|(_, allowed)| *allowed)
        || shard_mods.0.iter().any(|(_, allowed)| *allowed);
    let can_shoot = inventory
        .sources
        .iter()
        .any(|(color, has_shot)| (allowed_colors[color] || shard_mods.0[color]) && *has_shot);

    if !can_shoot && has_color {
        triggered.tick(time.delta());
        if triggered.just_finished() {
            // pause timer so it doesn't continue even after reset
            triggered.pause();
            tooltip_spawner.spawn_tooltip(
                "Stuck? Press R to restart",
                lyra,
                Vec3::new(0., 20., 0.),
            );
        }
    } else {
        triggered.reset();
    }
}
