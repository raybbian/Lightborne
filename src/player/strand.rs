use std::ops::Range;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{player::match_player::MatchPlayerPixel, shared::GroupLabel};

use super::{match_player::MatchPlayerZ, PlayerMarker};

#[derive(Component)]
/// [`Component`] representing one node in a chain of strands, used to simulate hair and clothes.
pub struct Strand {
    /// [`Entity`] the strand is connected to, that entity should have a [`Transform`] component
    pub connect: Entity,
    /// Offsets the point the strand connects to
    pub offset: Vec2,
    /// Maximum distance between this strand and `connect`
    pub dist: f32,

    /// Acceleration due to gravity, applied every [`FixedUpdate`]
    pub gravity: f32,
    /// The strand's velocity is multiplied by `friction` before being added to the [`Transform`] every [`FixedUpdate`]
    pub friction: f32,
    /// Specifies update order, with lower numbers updated first. Usually, strands nearer to the source (e.g. the player)
    /// should have a lower `priority` value.
    pub priority: u32,

    /// Specifies whether the strand collides with the ground
    pub physics: bool,

    last_pos: Vec2,
}

impl Strand {
    fn new(
        connect: Entity,
        offset: Vec2,
        dist: f32,
        gravity: f32,
        friction: f32,
        priority: u32,
        physics: bool,
    ) -> Self {
        Self {
            connect,
            offset,
            dist,
            gravity,
            friction,
            priority,
            last_pos: Vec2::new(0.0, 0.0),
            physics,
        }
    }
}

pub fn update_strand(
    mut q_strand: Query<(Entity, &mut Strand)>,
    mut q_transforms: Query<&mut Transform>,
    rapier_context: ReadDefaultRapierContext,
) {
    let mut strands = q_strand.iter_mut().collect::<Vec<_>>();
    strands.sort_by(|(_, a), (_, b)| a.priority.cmp(&b.priority));
    for (entity, strand) in strands.iter_mut() {
        let Ok([mut transform, connect_transform]) =
            q_transforms.get_many_mut([*entity, strand.connect])
        else {
            continue;
        };
        let connect_pos = connect_transform.translation.truncate() + strand.offset;
        let mut pos = transform.translation.truncate();

        let velocity = (pos - strand.last_pos) * strand.friction;

        strand.last_pos = pos;

        let acceleration = Vec2::new(0.0, -strand.gravity);
        pos += velocity + acceleration;

        if strand.physics {
            let filter = QueryFilter::new().groups(CollisionGroups::new(
                GroupLabel::STRAND,
                GroupLabel::TERRAIN,
            ));
            if let Some((_, toi)) = rapier_context.cast_ray(
                transform.translation.truncate(),
                Vec2::new(0.0, -1.0),
                2.0,
                true,
                filter,
            ) {
                let ground_level = transform.translation.y - toi;
                if pos.y < ground_level {
                    pos.y = ground_level;
                }
            }
        }

        let diff = connect_pos - pos;
        if diff.length() != strand.dist {
            let dist_to_move = diff.length() - strand.dist;
            pos += diff.normalize_or_zero() * dist_to_move;
        }

        transform.translation = pos.extend(transform.translation.z);
    }
}

pub struct StrandLayerGroup<'a> {
    assets: &'a [&'a str],
}

impl<'a> StrandLayerGroup<'a> {
    fn new(assets: &'a [&str]) -> Self {
        StrandLayerGroup { assets }
    }
}

pub fn add_player_hair_and_cloth(
    mut commands: Commands,
    q_player: Query<Entity, Added<PlayerMarker>>,
    asset_server: Res<AssetServer>,
) {
    let Ok(entity) = q_player.get_single() else {
        return;
    };
    add_player_strand(
        2.0,
        0.2..0.15,
        0.8,
        &[
            StrandLayerGroup::new(&["hair/clump_tiny_outline.png", "hair/clump_tiny.png"]),
            StrandLayerGroup::new(&["hair/clump_small_outline.png", "hair/clump_small.png"]),
            StrandLayerGroup::new(&["hair/clump_outline.png", "hair/clump.png"]),
        ],
        &[(2, false), (1, false), (1, false), (0, false)],
        Vec3::new(-2.0, 3.0, -0.3),
        PlayerRootStrandType::Hair,
        &mut commands,
        entity,
        &asset_server,
        Vec2::ZERO,
    );
    for i in 0..=1 {
        add_player_strand(
            1.0,
            0.12..0.03,
            0.6,
            &[
                StrandLayerGroup::new(&["cloth/clump_tiny_outline.png", "cloth/clump_tiny.png"]),
                StrandLayerGroup::new(&["cloth/clump_small_outline.png", "cloth/clump_small.png"]),
                StrandLayerGroup::new(&["cloth/clump_outline.png", "cloth/clump.png"]),
            ],
            &[
                (1, false),
                (1, false),
                (0, false),
                (0, false),
                (0, false),
                (0, true),
                (0, true),
                (0, true),
                (0, true),
                (0, true),
                (0, true),
            ],
            // &[
            //     1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            // ],
            Vec3::new(if i == 0 { -3.0 } else { 5.0 }, -5.0, -0.2),
            if i == 0 {
                PlayerRootStrandType::LeftCloth
            } else {
                PlayerRootStrandType::RightCloth
            },
            &mut commands,
            entity,
            &asset_server,
            Vec2::new(0.0, 1.0),
        );
    }
}

/// Creates a chain of strands to the player.
///
/// Each created [`Strand`] component has a `dist` of `strand_dist`, and a `gravity` of at `strand_gravity.start` near the player that slowly turns into
/// `strand_gravity.end`. The function layers the sprites in each list of `layer_groups`
/// based on their order, and creates an entity for each index in `layer_group_order`.
#[allow(clippy::too_many_arguments)]
pub fn add_player_strand(
    strand_dist: f32,
    strand_gravity: Range<f32>,
    strand_friction: f32,

    layer_groups: &[StrandLayerGroup],
    layer_group_order: &[(usize, bool)],
    player_offset: Vec3,
    player_root_strand_type: PlayerRootStrandType,

    commands: &mut Commands,
    player_entity: Entity,
    asset_server: &Res<AssetServer>,
    sprite_translate: Vec2,
) {
    let mut connect = player_entity;
    for (i, &(layer_index, physics)) in layer_group_order.iter().enumerate() {
        let first = i == 0;
        let strand_layer_group = &layer_groups[layer_index];
        let new_id = commands
            .spawn((
                Strand::new(
                    connect,
                    if first {
                        player_offset.truncate()
                    } else {
                        Vec2::ZERO
                    },
                    if first { 0.0 } else { strand_dist },
                    strand_gravity.start
                        + (i as f32 / layer_group_order.len() as f32)
                            * (strand_gravity.end - strand_gravity.start),
                    strand_friction,
                    i as u32,
                    physics,
                ),
                MatchPlayerPixel(default()),
                Transform::default(),
                InheritedVisibility::default(),
                MatchPlayerZ {
                    offset: player_offset.z,
                },
            ))
            .with_children(|parent| {
                for (layer_i, &layer) in strand_layer_group.assets.iter().enumerate() {
                    let layer_transform = Transform::from_translation(
                        Vec3::new(0., 0., (layer_i as f32) * 0.01) + sprite_translate.extend(0.0),
                    );

                    parent.spawn((
                        Sprite::from_image(asset_server.load(layer)),
                        layer_transform,
                    ));
                }
            })
            .id();
        if first {
            commands
                .entity(new_id)
                .insert(player_root_strand_type.clone());
        }
        connect = new_id;
    }
}

/// [`Component`] attached to the "root" strand (the strand closest to the player, the strand with `connect` equal to player)
/// for all strand chains attached to player. Used to query [`Strand`] components to update`offset` in response to player model changing,
/// e.g. lowering hair strand when player crouches.
#[derive(Component, Debug, Clone)]
pub enum PlayerRootStrandType {
    Hair,
    LeftCloth,
    RightCloth,
}

/// [`System`] that updates [`Strand`] offsets based on [`PlayerRootStrandType`] and player state. Currently doesn't do anything,
/// but should be used to make [`Strand`] offsets correct when player changes direction, crouches, etc.
pub fn update_player_strand_offsets(
    mut strands: Query<(&mut Strand, &PlayerRootStrandType)>,
    // currently queries for nothing, could be changed to query for a e.g. a Direction component.
    player: Query<(), With<PlayerMarker>>,
) {
    let Ok(()) = player.get_single() else { return }; // update this to read player state, e.g. player direction.
    for (mut strand, ty) in strands.iter_mut() {
        strand.offset = match ty {
            // update these to dynamically reflect player state, e.g. setting the Hair strand's offset to (2.0, 3.0) when facing left.
            PlayerRootStrandType::Hair => Vec2::new(-2.0, 3.0),
            PlayerRootStrandType::LeftCloth => Vec2::new(-3.0, -5.0),
            PlayerRootStrandType::RightCloth => Vec2::new(5.0, -5.0),
        };
    }
}
