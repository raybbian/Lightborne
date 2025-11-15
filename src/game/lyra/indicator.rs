use std::time::Duration;

use avian2d::prelude::*;
use avian2d::{math::PI, physics_transform::PhysicsTransformSystems};
use bevy::prelude::*;
use enum_map::{enum_map, EnumMap};

use crate::camera::TERRAIN_LAYER;
use crate::game::lighting::LineLight2d;
use crate::{
    camera::HIGHRES_LAYER,
    game::{
        light::LightColor,
        lyra::{beam::PlayerLightInventory, Lyra},
        LevelSystems,
    },
    shared::GameState,
};

pub const LYRA_SHARD_RADIUS: f32 = 12.;
pub const LYRA_SHARD_HEIGHT_DELTA: f32 = 4.;
pub const LYRA_SHARD_Y_OFFSET: f32 = -4.;
pub const LYRA_SHARD_PEAK_SHIFT_RATE: f32 = 1.2;
pub const LYRA_SHARD_MOVING_DELAY_MILLIS: u64 = 250;

pub struct LightIndicatorPlugin;

impl Plugin for LightIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightIndicatorData>();
        app.init_resource::<LightIndicators>();
        app.add_systems(
            FixedPostUpdate,
            (update_light_indicator, lerp_translations)
                .chain()
                .in_set(LevelSystems::Simulation)
                .after(PhysicsTransformSystems::PositionToTransform),
        );
        app.add_systems(OnEnter(GameState::InGame), init_light_indicators);
        app.add_systems(OnExit(GameState::InGame), cleanup_light_indicators);
    }
}

#[derive(Component)]
pub struct LerpTranslation {
    to: LerpTranslationTarget,
    lerp: f32,
}

pub enum LerpTranslationTarget {
    Entity(Entity),
    Position(Vec3),
}

pub fn lerp_translations(
    mut q_transforms: Query<&mut Transform>,
    q_following: Query<(Entity, &LerpTranslation)>,
) {
    for (entity, lerp_translation) in q_following.iter() {
        let pos = match lerp_translation.to {
            LerpTranslationTarget::Entity(e) => {
                let Ok(following) = q_transforms.get(e) else {
                    warn!("Lerp translation target entity had no transform!");
                    continue;
                };
                following.translation
            }
            LerpTranslationTarget::Position(pos) => pos,
        };
        let Ok(mut transform) = q_transforms.get_mut(entity) else {
            warn!("Lerp translation source entity had no transform!");
            continue;
        };
        transform.translation = transform.translation.lerp(pos, lerp_translation.lerp);
    }
}

/// A resource that stored handles to the [`Mesh2d`] and [`MeshMaterial2d`] used in the rendering
/// of [`LightSegment`](super::segments::LightSegmentBundle)s.
#[derive(Resource)]
pub struct LightIndicatorData {
    pub mesh: Mesh2d,
    pub material_map: EnumMap<LightColor, MeshMaterial2d<ColorMaterial>>,
}

impl FromWorld for LightIndicatorData {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes.add(Circle::new(3.0)).into();

        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

        LightIndicatorData {
            mesh: mesh_handle,
            material_map: enum_map! {
                val => materials.add(val.indicator_color()).into(),
            },
        }
    }
}

#[derive(Resource, Default)]
pub struct LightIndicators {
    angle_offset: f32,
    pub indicators: EnumMap<LightColor, Option<Entity>>,
}

#[derive(Component)]
pub struct LightShardIndicator;

pub fn init_light_indicators(
    mut commands: Commands,
    light_data: Res<LightIndicatorData>,
    mut indicators: ResMut<LightIndicators>,
) {
    for (color, entity) in indicators.indicators.iter_mut() {
        let id = commands
            .spawn(light_data.mesh.clone())
            .insert(light_data.material_map[color].clone())
            .insert(LightShardIndicator)
            .insert(Transform::default())
            .insert(HIGHRES_LAYER)
            .with_child((
                LineLight2d::point(color.lighting_color().extend(1.0), 20.0, 0.04),
                TERRAIN_LAYER,
            ))
            .id();

        *entity = Some(id);
    }
}

pub fn cleanup_light_indicators(mut commands: Commands, mut indicators: ResMut<LightIndicators>) {
    for (_, entity) in indicators.indicators.iter_mut() {
        if let Some(e) = entity {
            commands.entity(*e).despawn();
        }
        *entity = None;
    }
}

pub struct ShardMovingState {
    timer: Timer,
    state: ShardMovementState,
}

#[derive(Clone, Copy)]
pub enum ShardMovementState {
    Moving,
    Stationary,
}

impl Default for ShardMovingState {
    fn default() -> Self {
        Self {
            timer: Timer::new(
                Duration::from_millis(LYRA_SHARD_MOVING_DELAY_MILLIS),
                TimerMode::Once,
            ),
            state: ShardMovementState::Stationary,
        }
    }
}

pub fn update_light_indicator(
    lyra: Single<(&PlayerLightInventory, &Transform, &LinearVelocity), With<Lyra>>,
    mut commands: Commands,
    mut light_indicators: ResMut<LightIndicators>,
    time: Res<Time>,
    mut shard_moving_state: Local<ShardMovingState>,
) {
    let (inventory, base_transform, linear_velocity) = lyra.into_inner();
    light_indicators.angle_offset += time.delta_secs();

    let indicators: Vec<_> = light_indicators
        .indicators
        .iter()
        .filter_map(|(c, entity)| {
            if !inventory.can_shoot_color(c) {
                return None;
            }
            entity.map(|e| (c, e))
        })
        .collect();

    let handle_shooting = |commands: &mut Commands, c: LightColor, entity: Entity| -> bool {
        let is_shooting =
            inventory.current_color.is_some_and(|color| color == c) && inventory.can_shoot();

        if is_shooting {
            commands.entity(entity).insert(LerpTranslation {
                to: LerpTranslationTarget::Position(base_transform.translation),
                lerp: 1.0,
            });
            return true;
        }
        false
    };

    let is_moving = linear_velocity.0.length() > 0.5;

    match (is_moving, shard_moving_state.state) {
        (true, ShardMovementState::Stationary) => {
            shard_moving_state.timer.tick(time.delta());
            if shard_moving_state.timer.just_finished() {
                shard_moving_state.state = ShardMovementState::Moving;
                shard_moving_state.timer.reset();
            }
        }
        (false, ShardMovementState::Moving) => {
            shard_moving_state.timer.tick(time.delta());
            if shard_moving_state.timer.just_finished() {
                shard_moving_state.state = ShardMovementState::Stationary;
                shard_moving_state.timer.reset();
            }
        }
        _ => {
            shard_moving_state.timer.reset();
        }
    }

    match shard_moving_state.state {
        ShardMovementState::Moving => {
            let mut prev_e: Option<Entity> = None;
            for (c, entity) in indicators.iter() {
                if handle_shooting(&mut commands, *c, *entity) {
                    continue;
                }
                match prev_e {
                    Some(e) => {
                        commands.entity(*entity).insert(LerpTranslation {
                            to: LerpTranslationTarget::Entity(e),
                            lerp: 0.1,
                        });
                    }
                    None => {
                        commands.entity(*entity).insert(LerpTranslation {
                            to: LerpTranslationTarget::Position(base_transform.translation),
                            lerp: 0.1,
                        });
                    }
                }
                prev_e = Some(*entity);
            }
        }
        ShardMovementState::Stationary => {
            let mut rem_indicators = Vec::new();
            for (c, entity) in indicators.iter() {
                if handle_shooting(&mut commands, *c, *entity) {
                    continue;
                }
                rem_indicators.push((c, entity));
            }

            for (i, (_, entity)) in rem_indicators.iter().enumerate() {
                let ang = i as f32 / rem_indicators.len() as f32 * 2. * PI
                    + light_indicators.angle_offset;
                let pos = Vec3::new(
                    LYRA_SHARD_RADIUS * ang.cos(),
                    LYRA_SHARD_HEIGHT_DELTA / 2. * -(ang * LYRA_SHARD_PEAK_SHIFT_RATE).sin()
                        + LYRA_SHARD_Y_OFFSET,
                    LYRA_SHARD_RADIUS * ang.sin(),
                );
                commands.entity(**entity).insert(LerpTranslation {
                    to: LerpTranslationTarget::Position(pos + base_transform.translation),
                    lerp: 0.05,
                });
            }
        }
    }
}
