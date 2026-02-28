use std::time::Duration;

use avian2d::prelude::*;
use avian2d::{math::PI, physics_transform::PhysicsTransformSystems};
use bevy::prelude::*;
use enum_map::EnumMap;

use crate::asset::LoadResource;
use crate::camera::TERRAIN_LAYER;
use crate::game::light::LightBeamSource;
use crate::game::lighting::LineLight2d;
use crate::game::lyra::spawn_lyra;
use crate::ldtk::LdtkParam;
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
pub const LYRA_SHARD_HOLD_X: f32 = 4.;
pub const LYRA_SHARD_HOLD_Y: f32 = -2.;

pub struct LightIndicatorPlugin;

impl Plugin for LightIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<IndicatorAssets>();
        app.load_resource::<IndicatorAssets>();
        app.init_resource::<LightIndicators>();
        app.add_systems(
            FixedPostUpdate,
            (update_light_indicator, lerp_translations)
                .chain()
                .in_set(LevelSystems::Simulation)
                .after(PhysicsTransformSystems::PositionToTransform),
        );
        app.add_systems(
            OnEnter(GameState::InGame),
            init_light_indicators.after(spawn_lyra),
        );
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

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct IndicatorAssets {
    #[dependency]
    icons: [Handle<Image>; 4],
}

impl FromWorld for IndicatorAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            icons: [
                asset_server.load("indicator-shard/green-shard.png"),
                asset_server.load("indicator-shard/purple-shard.png"),
                asset_server.load("indicator-shard/white-shard.png"),
                asset_server.load("indicator-shard/blue-shard.png"),
            ],
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
    assets: Res<IndicatorAssets>,
    mut indicators: ResMut<LightIndicators>,
    ldtk_param: LdtkParam,
    lyra: Single<(&Transform, &PlayerLightInventory), With<Lyra>>,
) {
    let (transform, inventory) = lyra.into_inner();
    let color_ind = |color: LightColor| match color {
        LightColor::Green => 0,
        LightColor::Purple => 1,
        LightColor::White => 2,
        LightColor::Blue => 3,
    };
    for (color, entity) in indicators.indicators.iter_mut() {
        let id = commands
            .spawn(Sprite::from_image(assets.icons[color_ind(color)].clone()))
            .insert(LightShardIndicator)
            .insert(HIGHRES_LAYER)
            .with_child((
                LineLight2d::point(color.lighting_color().extend(1.0), 20.0, 0.04),
                TERRAIN_LAYER,
            ))
            .id();

        if !inventory.allowed[color] {
            commands
                .entity(id)
                .insert(Visibility::Hidden)
                .insert(Transform::from_translation(
                    ldtk_param
                        .crystal_shard_pos(color)
                        .expect("Crystal shard must be in level")
                        .extend(0.),
                ));
        } else {
            commands.entity(id).insert(Transform::from_translation(
                transform.translation.with_z(transform.translation.z - 0.1),
            ));
        }

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

#[derive(Component)]
pub struct WasChilded;

pub fn update_light_indicator(
    lyra: Single<
        (
            Entity,
            &PlayerLightInventory,
            &Transform,
            &LinearVelocity,
            &Sprite,
        ),
        With<Lyra>,
    >,
    q_transforms: Query<(&Transform, Has<WasChilded>), (Without<Lyra>, Without<LightBeamSource>)>,
    q_beam_sources: Query<(&Transform, &LightBeamSource)>,
    mut commands: Commands,
    mut light_indicators: ResMut<LightIndicators>,
    time: Res<Time>,
    mut shard_moving_state: Local<ShardMovingState>,
) {
    let (lyra_entity, inventory, base_transform, linear_velocity, lyra_sprite) = lyra.into_inner();
    light_indicators.angle_offset += time.delta_secs();

    for (color, entity) in light_indicators.indicators {
        let Some(entity) = entity else {
            continue;
        };
        if inventory.allowed[color] {
            commands.entity(entity).insert(Visibility::Visible);
        }
    }

    let indicators: Vec<_> = light_indicators
        .indicators
        .iter()
        .filter_map(|(c, e)| e.and_then(|e| Some((c, e))))
        .collect();

    let handle_shooting = |commands: &mut Commands, c: LightColor, entity: Entity| -> bool {
        let is_selected = inventory.current_color.is_some_and(|color| color == c);
        let is_shooting = is_selected && inventory.can_shoot();
        let Ok((indicator_transform, was_childed)) = q_transforms.get(entity) else {
            return false;
        };
        // TODO: fixme use inverse transforms instead
        if is_shooting {
            if !was_childed {
                commands
                    .entity(entity)
                    .insert(Transform::from_translation(
                        (indicator_transform.translation - base_transform.translation).with_z(1.),
                    ))
                    .insert(WasChilded)
                    .insert(ChildOf(lyra_entity));
            }
            let mult = if lyra_sprite.flip_x { -1. } else { 1. };
            commands.entity(entity).insert(LerpTranslation {
                to: LerpTranslationTarget::Position(Vec3::new(
                    mult * LYRA_SHARD_HOLD_X,
                    LYRA_SHARD_HOLD_Y,
                    0.,
                )),
                lerp: 0.2,
            });
            return true;
        } else if !inventory.can_shoot_color(c) {
            if was_childed {
                commands
                    .entity(entity)
                    .remove::<ChildOf>()
                    .remove::<WasChilded>()
                    .insert(Transform::from_translation(
                        base_transform.translation + indicator_transform.translation,
                    ));
            }
            let Some((source_transform, _)) =
                q_beam_sources.iter().find(|(_, source)| source.color == c)
            else {
                return true;
            };
            commands.entity(entity).insert(LerpTranslation {
                to: LerpTranslationTarget::Position(
                    source_transform
                        .translation
                        .with_z(source_transform.translation.z + 1.),
                ),
                lerp: 0.2,
            });
            return true;
        } else if is_selected {
            if !was_childed {
                commands
                    .entity(entity)
                    .insert(Transform::from_translation(
                        (indicator_transform.translation - base_transform.translation).with_z(1.),
                    ))
                    .insert(WasChilded)
                    .insert(ChildOf(lyra_entity));
            }
            commands.entity(entity).insert(LerpTranslation {
                to: LerpTranslationTarget::Position(Vec3::new(0., 13., 0.)),
                lerp: 0.2,
            });
            return true;
        }
        if was_childed {
            commands
                .entity(entity)
                .remove::<ChildOf>()
                .remove::<WasChilded>()
                .insert(Transform::from_translation(
                    base_transform.translation + indicator_transform.translation,
                ));
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
                            to: LerpTranslationTarget::Position(
                                base_transform
                                    .translation
                                    .with_z(base_transform.translation.z - 0.1),
                            ),
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
