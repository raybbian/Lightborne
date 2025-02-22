use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    level::{
        entity::{HurtMarker, Spike},
        misc::StartFlag,
        CurrentLevel,
    },
    shared::{GameState, ResetLevel},
};

use super::{
    light::PlayerLightInventory,
    movement::{PlayerMovement, PlayerState},
    PlayerHurtMarker, PlayerMarker,
};

/// [`System`] that runs on [`GameState::Respawning`]. Will turn the state back into playing
/// immediately.
pub fn reset_player_position(
    mut q_player: Query<&mut Transform, With<PlayerMarker>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut ev_reset_level: EventReader<ResetLevel>,
    q_start_flag: Query<(&StartFlag, &EntityInstance)>,
    current_level: Res<CurrentLevel>,
) {
    // check that we recieved a ResetLevel event asking us to Respawn
    if !ev_reset_level.read().any(|x| *x == ResetLevel::Respawn) {
        return;
    }
    let Ok(mut transform) = q_player.get_single_mut() else {
        return;
    };

    next_game_state.set(GameState::Playing);

    for (flag, instance) in q_start_flag.iter() {
        if current_level.level_iid == flag.level_iid {
            transform.translation.x =
                instance.world_x.expect("Lightborne uses Free world layout") as f32;
            transform.translation.y =
                -instance.world_y.expect("Lightborne uses Free world layout") as f32 + 3.0;
            // add small height so Lyra is not stuck into the floor
            return;
        }
    }

    panic!("Couldn't find start flag to respawn at");
}

/// Resets the player inventory and movement information on a [`LevelSwitchEvent`]
pub fn reset_player_on_level_switch(
    mut q_player: Query<
        (
            &mut PlayerMovement,
            &mut PlayerLightInventory,
            &mut PlayerState,
            &mut Transform,
        ),
        With<PlayerMarker>,
    >,
) {
    let Ok((mut movement, mut inventory, mut state, mut transform)) = q_player.get_single_mut()
    else {
        return;
    };

    *transform = transform.with_scale(Vec3::ONE);
    *state = PlayerState::Idle;
    *movement = PlayerMovement::default();
    *inventory = PlayerLightInventory::default();
}

/// Kills player upon touching a HURT_BOX
pub fn kill_player_on_spike(
    rapier_context: Query<&RapierContext>,
    q_player: Query<Entity, With<PlayerHurtMarker>>,
    mut q_hurt: Query<(&mut Spike, Entity), With<HurtMarker>>,
    mut ev_reset_level: EventWriter<ResetLevel>,
) {
    let Ok(rapier) = rapier_context.get_single() else {
        return;
    };
    let Ok(player) = q_player.get_single() else {
        return;
    };

    for (mut spike, hurt) in q_hurt.iter_mut() {
        if rapier.intersection_pair(player, hurt) == Some(true) {
            spike.add_death();
            ev_reset_level.send(ResetLevel::Respawn);
            return;
        }
    }
}
