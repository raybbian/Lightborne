use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{animation::AnimationConfig, lighting::light::PointLighting, shared::GroupLabel};

use super::{
    animation::PlayerAnimationType, light::PlayerLightInventory, movement::PlayerMovement,
    PlayerBundle, PlayerMarker,
};

/// Attached to player hitbox
#[derive(Default, Component)]
pub struct PlayerHurtMarker;

/// Used by Ldtk to spawn the player correctly with all of the correct [`Component`]s.
pub fn init_player_bundle(_: &EntityInstance) -> PlayerBundle {
    PlayerBundle {
        body: RigidBody::KinematicPositionBased,
        controller: KinematicCharacterController {
            filter_groups: Some(CollisionGroups::new(
                GroupLabel::PLAYER_COLLIDER,
                GroupLabel::TERRAIN,
            )),
            offset: CharacterLength::Absolute(1.0),
            ..default()
        },
        controller_output: KinematicCharacterControllerOutput::default(),
        collider: Collider::compound(vec![(
            Vect::new(0.0, -2.0),
            Rot::default(),
            Collider::cuboid(6.0, 7.0),
        )]),
        collision_groups: CollisionGroups::new(GroupLabel::PLAYER_COLLIDER, GroupLabel::TERRAIN),
        player_movement: PlayerMovement::default(),
        friction: Friction {
            coefficient: 0.,
            combine_rule: CoefficientCombineRule::Min,
        },
        restitution: Restitution {
            coefficient: 0.,
            combine_rule: CoefficientCombineRule::Min,
        },
        light_inventory: PlayerLightInventory::default(),
        point_lighting: PointLighting {
            color: Vec3::new(0.8, 0.8, 0.8),
            radius: 40.0,
        },
        animation_type: PlayerAnimationType::Idle,
        animation_config: AnimationConfig::from(PlayerAnimationType::Idle),
    }
}

/// [`System`] that spawns the player's hurtbox [`Collider`] as a child entity.
pub fn add_player_sensors(
    mut commands: Commands,
    q_player: Query<Entity, Added<PlayerMarker>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Ok(player) = q_player.get_single() else {
        return;
    };

    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(15, 20),
        21,
        1,
        None,
        None,
    ));

    // insert sprite here because it depends on texture atlas which needs a resource
    commands.entity(player).insert(Sprite {
        image: asset_server.load("lyra_sheet.png"),
        texture_atlas: Some(TextureAtlas {
            layout: texture_atlas_layout,
            index: 0,
        }),
        ..default()
    });

    commands.entity(player).with_children(|parent| {
        parent
            .spawn(Collider::compound(vec![(
                Vect::new(0.0, -2.0),
                Rot::default(),
                Collider::cuboid(4.0, 5.0),
            )]))
            .insert(Sensor)
            .insert(RigidBody::Dynamic)
            .insert(GravityScale(0.0))
            .insert(PlayerHurtMarker)
            .insert(CollisionGroups::new(
                GroupLabel::PLAYER_SENSOR,
                GroupLabel::HURT_BOX | GroupLabel::TERRAIN,
            ))
            .insert(PointLight {
                intensity: 100_000.0,
                ..default()
            });
    });
}
