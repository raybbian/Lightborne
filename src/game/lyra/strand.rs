use std::ops::Range;

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    asset::LoadResource,
    camera::LYRA_LAYER,
    game::{
        animation::AnimationConfig,
        lyra::{
            animation::{flip_player_direction, PlayerAnimationType},
            spawn_lyra, Lyra,
        },
        Layers, LevelSystems,
    },
    shared::GameState,
};

pub struct LyraStrandPlugin;

impl Plugin for LyraStrandPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HairClothAssets>();
        app.load_resource::<HairClothAssets>();
        app.add_systems(
            OnEnter(GameState::InGame),
            add_lyra_hair_cloth.after(spawn_lyra),
        );
        app.add_systems(OnExit(GameState::InGame), despawn_strands);
        app.add_systems(
            FixedUpdate,
            update_strand
                .after(update_player_strand_offsets)
                .in_set(LevelSystems::Simulation),
        );
        app.add_systems(
            FixedUpdate,
            update_player_strand_offsets
                .after(flip_player_direction)
                .in_set(LevelSystems::Simulation),
        );
    }
}

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
    ) -> Self {
        Self {
            connect,
            offset,
            dist,
            gravity,
            friction,
            priority,
            last_pos: Vec2::new(0.0, 0.0),
        }
    }
}

pub fn update_strand(
    mut q_strand: Query<(Entity, &mut Strand)>,
    q_rays: Query<(&RayCaster, &RayHits)>,
    mut q_transforms: Query<&mut Transform>,
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

        if let Ok((ray, hits)) = q_rays.get(*entity) {
            if let Some(hit) = hits.iter().next() {
                let hit = ray.global_origin() + *ray.global_direction() * hit.distance;
                if pos.y < hit.y {
                    pos.y = hit.y;
                }
            };
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
    assets: &'a [Handle<Image>],
}

impl<'a> StrandLayerGroup<'a> {
    fn new(assets: &'a [Handle<Image>]) -> Self {
        StrandLayerGroup { assets }
    }
}

#[derive(Resource, Asset, Reflect, Clone)]
#[reflect(Resource)]
pub struct HairClothAssets {
    hair_tiny: [Handle<Image>; 2],
    hair_small: [Handle<Image>; 2],
    hair: [Handle<Image>; 2],
    cloth_tiny: [Handle<Image>; 2],
    cloth_small: [Handle<Image>; 2],
    cloth: [Handle<Image>; 2],
}

impl FromWorld for HairClothAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            hair_tiny: [
                asset_server.load("hair/clump_tiny_outline.png"),
                asset_server.load("hair/clump_tiny.png"),
            ],
            hair_small: [
                asset_server.load("hair/clump_small_outline.png"),
                asset_server.load("hair/clump_small.png"),
            ],
            hair: [
                asset_server.load("hair/clump_outline.png"),
                asset_server.load("hair/clump.png"),
            ],
            cloth_tiny: [
                asset_server.load("cloth/clump_tiny_outline.png"),
                asset_server.load("cloth/clump_tiny.png"),
            ],
            cloth_small: [
                asset_server.load("cloth/clump_small_outline.png"),
                asset_server.load("cloth/clump_small.png"),
            ],
            cloth: [
                asset_server.load("cloth/clump_outline.png"),
                asset_server.load("cloth/clump.png"),
            ],
        }
    }
}

pub fn add_lyra_hair_cloth(
    lyra: Single<Entity, With<Lyra>>,
    mut commands: Commands,
    hair_cloth_assets: Res<HairClothAssets>,
) {
    add_player_strand(
        2.0,
        0.2..0.15,
        0.8,
        &[
            StrandLayerGroup::new(&hair_cloth_assets.hair_tiny),
            StrandLayerGroup::new(&hair_cloth_assets.hair_small),
            StrandLayerGroup::new(&hair_cloth_assets.hair),
        ],
        &[(2, false), (1, false), (1, false), (0, false)],
        Vec3::new(-2.0, 3.0, -0.3),
        PlayerRootStrandType::Hair,
        &mut commands,
        *lyra,
        Vec2::ZERO,
    );
    for i in 0..=1 {
        add_player_strand(
            1.0,
            0.12..0.03,
            0.6,
            &[
                StrandLayerGroup::new(&hair_cloth_assets.cloth_tiny),
                StrandLayerGroup::new(&hair_cloth_assets.cloth_small),
                StrandLayerGroup::new(&hair_cloth_assets.cloth),
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
            Vec3::new(if i == 0 { -3.0 } else { 5.0 }, -5.0, -0.2),
            if i == 0 {
                PlayerRootStrandType::LeftCloth
            } else {
                PlayerRootStrandType::RightCloth
            },
            &mut commands,
            *lyra,
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
                ),
                InheritedVisibility::default(),
                Transform::from_translation(Vec3::new(0., 0., player_offset.z)),
                LYRA_LAYER,
            ))
            .with_children(|parent| {
                for (layer_i, layer) in strand_layer_group.assets.iter().enumerate() {
                    let layer_transform = Transform::from_translation(
                        Vec3::new(0., 0., (layer_i as f32) * 0.01) + sprite_translate.extend(0.0),
                    );

                    parent.spawn((
                        LYRA_LAYER,
                        Sprite::from_image(layer.clone()),
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

        if physics {
            let query_filter = SpatialQueryFilter::default().with_mask([
                Layers::Terrain,
                Layers::BlueCrystal,
                Layers::Platform,
            ]);
            commands.entity(new_id).insert(
                RayCaster::new(Vec2::ZERO, Dir2::NEG_Y)
                    .with_solidness(true)
                    .with_max_distance(2.0)
                    .with_query_filter(query_filter),
            );
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

pub fn update_player_strand_offsets(
    mut strands: Query<(&mut Strand, &PlayerRootStrandType)>,
    player: Single<(&PlayerAnimationType, &Sprite, &AnimationConfig), With<Lyra>>,
) {
    let (anim_type, sprite, anim_config) = player.into_inner();
    for (mut strand, ty) in strands.iter_mut() {
        strand.offset = match ty {
            PlayerRootStrandType::Hair => anim_type.hair_offset(anim_config.cur_index),
            PlayerRootStrandType::LeftCloth => anim_type.left_cloth_offset(anim_config.cur_index),
            PlayerRootStrandType::RightCloth => anim_type.right_cloth_offset(anim_config.cur_index),
        };
        if sprite.flip_x {
            strand.offset.x *= -1.0;
        }
    }
}

pub fn despawn_strands(mut commands: Commands, q_strands: Query<Entity, With<Strand>>) {
    for strand in q_strands.iter() {
        commands.entity(strand).try_despawn();
    }
}
