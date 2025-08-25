use bevy::prelude::*;

use super::PlayerMarker;

/// [`Component`] that sets Entity's `z` to the player's `z` plus `offset`
#[derive(Component)]
pub struct MatchPlayerZ {
    pub offset: f32,
}

pub fn update_match_player_z(
    mut query: Query<(&mut Transform, &MatchPlayerZ)>,
    player_transform: Query<&Transform, (With<PlayerMarker>, Without<MatchPlayerZ>)>,
) {
    let Ok(player_transform) = player_transform.get_single() else {
        return;
    };
    for (mut transform, MatchPlayerZ { offset }) in query.iter_mut() {
        transform.translation.z = player_transform.translation.z + offset;
    }
}
