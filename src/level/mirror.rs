use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use super::entity::FixedEntityBundle;

pub struct MirrorPlugin;
impl Plugin for MirrorPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_int_cell_for_layer::<MirrorBundle>("Terrain", 16);
        // .add_systems(
        //     PreUpdate,
        //     adjust_semisolid_colliders.in_set(LevelSystems::Processing),
        // )
        // .add_systems(
        //     FixedUpdate,
        //     update_semisolid_colliders.in_set(LevelSystems::Simulation),
        // );
    }
}

#[derive(Default, Component)]
pub struct Mirror;

/// Bundle for Semi-Solid Platforms
#[derive(Bundle, Default, LdtkIntCell)]
pub struct MirrorBundle {
    #[from_int_grid_cell]
    fixed_entity_bundle: FixedEntityBundle,
    mirror: Mirror,
}

// pub fn adjust_semisolid_colliders(mut q_semisolid: Query<&mut Transform, Added<SemiSolid>>) {
//     for mut transform in q_semisolid.iter_mut() {
//         transform.translation.y += 3.;
//     }
// }

// pub fn update_semisolid_colliders(
//     q_player: Query<(&PlayerMovement, &GlobalTransform), With<PlayerMarker>>,
//     mut q_semisolid: Query<(&GlobalTransform, &mut CollisionGroups), With<SemiSolid>>,
// ) {
//     let Ok((movement, player)) = q_player.get_single() else {
//         return;
//     };
//     const PLAYER_HALF_HEIGHT: f32 = 9.0;
//     let cutoff_height = if movement.crouching {
//         PLAYER_HALF_HEIGHT / 2.0
//     } else {
//         PLAYER_HALF_HEIGHT
//     };

//     for (transform, mut collisions) in q_semisolid.iter_mut() {
//         if player.compute_transform().translation.y - transform.compute_transform().translation.y
//             > cutoff_height
//         {
//             *collisions = CollisionGroups::new(GroupLabel::TERRAIN, GroupLabel::ALL);
//         } else {
//             *collisions = CollisionGroups::new(
//                 GroupLabel::TERRAIN,
//                 GroupLabel::ALL & !GroupLabel::PLAYER_COLLIDER,
//             );
//         }
//     }
// }
