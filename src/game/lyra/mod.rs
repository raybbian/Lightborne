use avian2d::prelude::*;
use bevy::{
    camera::RenderTarget, core_pipeline::tonemapping::Tonemapping, prelude::*, render::view::Hdr,
};

use crate::{
    camera::{build_render_target, HIGHRES_LAYER, LYRA_LAYER},
    game::{
        animation::AnimationConfig,
        camera_op::SnapToLyra,
        defs::{
            one_way_platform::PassThroughOneWayPlatform,
            shard::on_player_intersect_shard,
            tooltip_sign::{display_tooltip_signs, hide_tooltip_signs},
        },
        level_completion::handle_start_end_markers,
        lighting::LineLight2d,
        lyra::{
            animation::{LyraAnimationPlugin, PlayerAnimationType, ANIMATION_FRAMES},
            beam::{
                on_collide_beam_source, on_leave_beam_source, BeamControllerPlugin,
                PlayerLightInventory, PlayerLightSaveData,
            },
            controller::{
                CachedLinearVelocity, CharacterController, CharacterControllerPlugin, MovementInfo,
            },
            indicator::LightIndicatorPlugin,
            kill::{kill_player_on_danger, LyraKillPlugin},
            restart_hint::HintRestartPlugin,
            strand::LyraStrandPlugin,
        },
        Layers,
    },
    ldtk::{LdtkLevelParam, LevelExt},
    shared::GameState,
};

mod animation;
pub mod beam;
pub mod controller;
mod indicator;
mod kill;
mod restart_hint;
mod strand;

pub const LYRA_RESPAWN_EPSILON: f32 = 3.0;

pub struct LyraPlugin;

impl Plugin for LyraPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: do not let ldtk spawn lyra because that would require level select to select a
        // level adjacent to lyra's level in the ldtk file.
        app.add_plugins(CharacterControllerPlugin);
        app.add_plugins(LyraStrandPlugin);
        app.add_plugins(LyraAnimationPlugin);
        app.add_plugins(LyraKillPlugin);
        app.add_plugins(BeamControllerPlugin);
        app.add_plugins(HintRestartPlugin);
        app.add_plugins(LightIndicatorPlugin);
        app.add_systems(OnEnter(GameState::InGame), spawn_lyra);
        app.add_systems(OnEnter(GameState::InGame), spawn_lyra_cam.after(spawn_lyra));
        app.add_systems(OnExit(GameState::InGame), despawn_lyra);
    }
}

#[derive(Component)]
pub struct Lyra;

#[derive(Component)]
pub struct LyraHurtBox;

#[derive(Component)]
pub enum LyraWallCaster {
    Left,
    Right,
}

pub fn lyra_spawn_transform(ldtk_level_param: &LdtkLevelParam) -> Vec3 {
    let Some(lyra_transform) = ldtk_level_param.cur_level().and_then(|level| {
        level
            .raw()
            .start_flag_pos()
            .map(|pos| Vec3::new(pos.x, pos.y + LYRA_RESPAWN_EPSILON, 100.))
    }) else {
        panic!("Current level must exist and it must have a start flag");
    };
    lyra_transform
}

pub fn spawn_lyra(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    ldtk_level_param: LdtkLevelParam,
    player_light_save_data: Res<PlayerLightSaveData>,
) {
    info!("Spawning Lyra!");

    let lyra_transform = lyra_spawn_transform(&ldtk_level_param);
    // NOTE: actual z value doesn't matter because lyra is rendered on a separate layer
    let player = commands
        .spawn(Lyra)
        .insert(Transform::from_translation(lyra_transform))
        .id();

    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(15, 20),
        ANIMATION_FRAMES as u32,
        1,
        None,
        None,
    ));

    // insert sprite here because it depends on texture atlas which needs a resource
    commands
        .entity(player)
        .insert(Sprite {
            image: asset_server.load("lyra_sheet.png"),
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout,
                index: 0,
            }),
            ..default()
        })
        .insert(LYRA_LAYER);

    commands
        .entity(player)
        .insert(CollisionLayers::new(
            Layers::PlayerCollider,
            [Layers::Terrain, Layers::BlueCrystal, Layers::Platform],
        ))
        .insert(CharacterController)
        .insert(RigidBody::Dynamic)
        .insert(TransformInterpolation)
        .insert(Collider::compound(vec![(
            Vec2::new(0.0, -2.0),
            Rotation::default(),
            Collider::rectangle(12.0, 16.0), // 12 x 16
        )]))
        .insert(MovementInfo::default())
        .insert(
            ShapeCaster::new(
                Collider::rectangle(11.8, 0.5),
                Vec2::new(0., -9.75),
                0.0,
                Dir2::NEG_Y,
            )
            .with_max_distance(0.5)
            .with_max_hits(10)
            .with_query_filter(SpatialQueryFilter::default().with_mask([
                Layers::Terrain,
                Layers::BlueCrystal,
                Layers::Platform,
            ])),
        )
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(Friction {
            dynamic_coefficient: 0.0,
            static_coefficient: 0.0,
            combine_rule: CoefficientCombine::Min,
        })
        .insert(Restitution {
            coefficient: 0.0,
            combine_rule: CoefficientCombine::Min,
        })
        .insert(CachedLinearVelocity(Vec2::ZERO))
        .insert(PlayerLightInventory::from(
            player_light_save_data.into_inner(),
        ))
        .insert(PlayerAnimationType::Idle)
        .insert(PassThroughOneWayPlatform::ByNormal)
        .insert(AnimationConfig::from(PlayerAnimationType::Idle));

    commands
        .spawn(Collider::compound(vec![(
            Vec2::new(0.0, -2.0),
            Rotation::default(),
            Collider::rectangle(6.0, 8.0),
        )]))
        .insert(CollisionEventsEnabled)
        .insert(Sensor)
        .insert(ChildOf(player))
        .insert(GravityScale(0.0))
        .insert(LyraHurtBox)
        .insert(Transform::default())
        .insert(CollisionLayers::new(
            Layers::PlayerHurtbox,
            [Layers::DangerBox, Layers::SensorBox],
        ))
        .insert(LineLight2d::point(
            Vec4::new(1.0, 1.0, 1.0, 1.0),
            40.0,
            0.01,
        ))
        .observe(on_collide_beam_source)
        .observe(on_leave_beam_source)
        .observe(hide_tooltip_signs)
        .observe(display_tooltip_signs)
        .observe(handle_start_end_markers)
        .observe(on_player_intersect_shard)
        .observe(kill_player_on_danger);

    commands
        .spawn(
            ShapeCaster::new(
                Collider::rectangle(0.5, 15.8),
                Vec2::new(5.75, -2.0),
                0.0,
                Dir2::X,
            )
            .with_max_distance(0.5)
            .with_max_hits(10)
            .with_query_filter(
                SpatialQueryFilter::default().with_mask([Layers::Terrain, Layers::BlueCrystal]),
            ),
        )
        .insert(LyraWallCaster::Right)
        .insert(ChildOf(player));

    commands
        .spawn(
            ShapeCaster::new(
                Collider::rectangle(0.5, 15.8),
                Vec2::new(-5.75, -2.0),
                0.0,
                Dir2::NEG_X,
            )
            .with_max_distance(0.5)
            .with_max_hits(10)
            .with_query_filter(
                SpatialQueryFilter::default().with_mask([Layers::Terrain, Layers::BlueCrystal]),
            ),
        )
        .insert(LyraWallCaster::Left)
        .insert(ChildOf(player));

    commands.trigger(SnapToLyra);
}

#[derive(Component)]
pub struct PlayerCamera;

pub fn spawn_lyra_cam(
    lyra: Single<Entity, With<Lyra>>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let (lyra_image, lyra_projection) = build_render_target(50, 50);
    let lyra_handle = images.add(lyra_image);

    // NOTE: lyra cam doesn't have pixelperfectcam because childing it to lyra makes it snap
    // automatically, and the canvas as a child means it also snaps automatically
    commands
        .spawn(Camera2d)
        .insert(PlayerCamera)
        .insert(Camera {
            order: 0,
            target: RenderTarget::Image(lyra_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        })
        .insert(Hdr)
        .insert(Tonemapping::TonyMcMapface)
        .insert(lyra_projection)
        .insert(Transform::from_xyz(-0.1, -0.1, 0.0))
        .insert(LYRA_LAYER)
        .insert(ChildOf(*lyra))
        .with_child((
            Sprite::from_image(lyra_handle.clone()),
            HIGHRES_LAYER,
            Transform::default(),
        ));
}

pub fn despawn_lyra(mut commands: Commands, player: Single<Entity, With<Lyra>>) {
    info!("Despawning Lyra!");

    commands.entity(*player).despawn();
}
