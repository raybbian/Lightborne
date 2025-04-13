use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    particle::dust::DustSurface,
    player::{movement::PlayerMovement, PlayerMarker},
    shared::GroupLabel,
};

use super::{entity::FixedEntityBundle, LevelSystems};

pub struct SemiSolidPlugin;
impl Plugin for SemiSolidPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_int_cell_for_layer::<SemiSolidPlatformBundle>("Terrain", 15)
            .add_systems(
                PreUpdate,
                adjust_semisolid_colliders.in_set(LevelSystems::Processing),
            )
            .add_systems(
                FixedUpdate,
                update_semisolid_colliders.in_set(LevelSystems::Simulation),
            );
    }
}

#[derive(Default, Component)]
pub struct SemiSolid;

/// Bundle for Semi-Solid Platforms
#[derive(Bundle, LdtkIntCell)]
pub struct SemiSolidPlatformBundle {
    #[from_int_grid_cell]
    fixed_entity_bundle: FixedEntityBundle,
    semi_solid: SemiSolid,
    dust_surface: DustSurface,
}

impl Default for SemiSolidPlatformBundle {
    fn default() -> Self {
        Self {
            fixed_entity_bundle: FixedEntityBundle::default(),
            semi_solid: SemiSolid,
            dust_surface: DustSurface::Wood,
        }
    }
}
pub fn adjust_semisolid_colliders(mut q_semisolid: Query<&mut Transform, Added<SemiSolid>>) {
    for mut transform in q_semisolid.iter_mut() {
        transform.translation.y += 3.;
    }
}

/// Sets the state of SemiSolids based on Player's y coord
pub fn update_semisolid_colliders(
    q_player: Query<(&PlayerMovement, &GlobalTransform), With<PlayerMarker>>,
    mut q_semisolid: Query<(&GlobalTransform, &mut CollisionGroups), With<SemiSolid>>,
) {
    let Ok((movement, player)) = q_player.get_single() else {
        return;
    };
    const PLAYER_HALF_HEIGHT: f32 = 9.0;
    let cutoff_height = if movement.crouching {
        PLAYER_HALF_HEIGHT / 2.0
    } else {
        PLAYER_HALF_HEIGHT
    };

    for (transform, mut collisions) in q_semisolid.iter_mut() {
        if player.compute_transform().translation.y - transform.compute_transform().translation.y
            > cutoff_height
        {
            *collisions = CollisionGroups::new(GroupLabel::TERRAIN, GroupLabel::ALL);
        } else {
            *collisions = CollisionGroups::new(
                GroupLabel::TERRAIN,
                GroupLabel::ALL & !GroupLabel::PLAYER_COLLIDER,
            );
        }
    }
}
