use std::time::Duration;

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use enum_map::EnumMap;

use crate::{
    animation::AnimationConfig,
    camera::{
        camera_position_from_level, camera_position_from_level_with_scale, CameraControlType,
        CameraMoveEvent, CameraZoomEvent, MainCamera,
    },
    light::LightColor,
    lighting::LineLight2d,
    player::{
        light::{
            despawn_angle_increments_indicators, despawn_angle_indicator, should_shoot_light,
            PlayerLightInventory,
        },
        InputLocked, PlayerHurtMarker, PlayerMarker,
    },
    shared::{AnimationState, GameState, ResetLevel},
    sound::{BgmMarker, Fade, FadeSettings, BGM_VOLUME},
};

use super::{entity::FixedEntityBundle, CurrentLevel, LevelSystems};

pub struct CrystalShardPlugin;

impl Plugin for CrystalShardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ShardAnimationEvent>()
            .init_resource::<CrystalShardMods>()
            .init_resource::<ShardAnimationCallbacks>()
            .register_ldtk_entity::<CrystalShardBundle>("CrystalShard")
            .add_systems(
                PreUpdate,
                add_crystal_shard_sprites.in_set(LevelSystems::Processing),
            )
            // FIXME: if the player is holding down the mouse button while collecting a shard,
            // the preview and angle indicator will stay. need to potentially consider a input
            // manager resource
            .add_systems(
                Update,
                (
                    despawn_angle_indicator,
                    despawn_angle_increments_indicators,
                    should_shoot_light::<false>,
                )
                    .run_if(on_event::<ShardAnimationEvent>),
            )
            .add_systems(
                Update,
                (
                    reset_shard_visibility,
                    (reset_shard_effects_on_kill, reset_shard_effects_cache).chain(),
                )
                    .in_set(LevelSystems::Reset),
            )
            .add_systems(
                FixedUpdate,
                (on_player_intersect_shard, start_shard_animation)
                    .chain()
                    .in_set(LevelSystems::Simulation),
            );
    }
}

#[derive(Component, Debug)]
pub struct CrystalShard {
    light_color: LightColor,
}

impl From<&EntityInstance> for CrystalShard {
    fn from(value: &EntityInstance) -> Self {
        let light_color = value
            .get_enum_field("light_color")
            .expect("All crystal shards should have a light_color enum field")
            .into();

        Self { light_color }
    }
}

#[derive(Bundle, LdtkEntity)]
pub struct CrystalShardBundle {
    #[from_entity_instance]
    shard: CrystalShard,
    #[from_entity_instance]
    physics: FixedEntityBundle,
    #[with(crystal_shard_light)]
    light: LineLight2d,
    #[default]
    sensor: Sensor,
}

pub fn crystal_shard_light(entity_instance: &EntityInstance) -> LineLight2d {
    let light_color: LightColor = entity_instance
        .get_enum_field("light_color")
        .expect("All crystal shards should have a light_color enum field")
        .into();

    LineLight2d::point(light_color.lighting_color().extend(1.0), 60.0, 0.015)
}

#[derive(Resource, Default)]
/// Sets a value to true if the light color was obtained from a crystal in the current level
pub struct CrystalShardMods(EnumMap<LightColor, bool>);

pub fn add_crystal_shard_sprites(
    mut commands: Commands,
    q_shards: Query<(Entity, &CrystalShard), Added<CrystalShard>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
) {
    const CRYSTAL_SHARD_FRAMES: usize = 7;
    const CRYSTAL_SHARD_ROWS: usize = 4;

    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(12, 16),
        CRYSTAL_SHARD_FRAMES as u32,
        CRYSTAL_SHARD_ROWS as u32,
        None,
        None,
    ));

    let shard_row = |shard: &CrystalShard| match shard.light_color {
        LightColor::Blue => 0,
        LightColor::Green => 1,
        LightColor::Purple => 2,
        LightColor::White => 3,
        LightColor::Black => 4,
    };

    for (shard_entity, shard) in q_shards.iter() {
        let start_index = shard_row(shard) * CRYSTAL_SHARD_FRAMES;
        commands.entity(shard_entity).insert((
            Sprite {
                image: asset_server.load("crystal_shard_sheet.png"),
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout.clone(),
                    index: start_index,
                }),
                ..default()
            },
            AnimationConfig::new(start_index, start_index + CRYSTAL_SHARD_FRAMES - 1, 6, true),
        ));
    }
}

pub fn reset_shard_visibility(mut q_shards: Query<&mut Visibility, With<CrystalShard>>) {
    for mut visibility in q_shards.iter_mut() {
        *visibility = Visibility::Visible;
    }
}
pub fn reset_shard_effects_cache(mut shard_mods: ResMut<CrystalShardMods>) {
    for (_, is_temporary) in shard_mods.0.iter_mut() {
        *is_temporary = false;
    }
}

pub fn reset_shard_effects_on_kill(
    mut current_level: ResMut<CurrentLevel>,
    mut shard_mods: ResMut<CrystalShardMods>,
    mut ev_reset_level: EventReader<ResetLevel>,
) {
    if !ev_reset_level.read().any(|ev| *ev == ResetLevel::Respawn) {
        return;
    }

    for (color, is_temporary) in shard_mods.0.iter_mut() {
        if *is_temporary {
            current_level.allowed_colors[color] = false;
        }
    }
}

pub fn on_player_intersect_shard(
    q_shards: Query<(Entity, &CrystalShard, &Visibility)>,
    mut q_player: Query<Entity, With<PlayerHurtMarker>>,
    rapier_context: Query<&RapierContext>,
    current_level: Res<CurrentLevel>,
    mut shard_mods: ResMut<CrystalShardMods>,
    mut ev_shard_animation: EventWriter<ShardAnimationEvent>,
) {
    let Ok(rapier_context) = rapier_context.get_single() else {
        return;
    };
    let Ok(player_entity) = q_player.get_single_mut() else {
        return;
    };
    for (shard_entity, shard, shard_visibility) in q_shards.iter() {
        // FIXME: we hide shards once we collect them, so we don't try to collect them again if
        // they are hidden
        if shard_visibility == Visibility::Hidden {
            continue;
        }
        if let Some(true) = rapier_context.intersection_pair(player_entity, shard_entity) {
            ev_shard_animation.send(ShardAnimationEvent((shard_entity, shard.light_color)));
            if !current_level.allowed_colors[shard.light_color] {
                // only mark as temporary modification if not actually allowed
                shard_mods.0[shard.light_color] = true;
            }
        }
    }
}

#[derive(Event)]
pub struct ShardAnimationEvent((Entity, LightColor));

#[derive(Resource)]
pub struct ShardAnimationCallbacks {
    for_shard: Option<(Entity, LightColor)>,
    cb: [SystemId; 3],
}

impl FromWorld for ShardAnimationCallbacks {
    fn from_world(world: &mut World) -> Self {
        Self {
            for_shard: None,
            cb: [
                world.register_system(on_shard_zoom_in_finished),
                world.register_system(on_shard_text_read_finish),
                world.register_system(on_shard_zoom_back_finish),
            ],
        }
    }
}

const SHARD_FADE_DURATION: Duration = Duration::from_millis(500);
const SHARD_FADE_VOLUME: f32 = 0.1;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn start_shard_animation(
    mut commands: Commands,
    cur_game_state: Res<State<GameState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_anim_state: ResMut<NextState<AnimationState>>,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
    mut ev_zoom_camera: EventWriter<CameraZoomEvent>,
    mut ev_shard_animation: EventReader<ShardAnimationEvent>,
    q_player: Query<(Entity, &GlobalTransform), With<PlayerMarker>>,
    mut shard_anim_cbs: ResMut<ShardAnimationCallbacks>,
    current_level: Res<CurrentLevel>,
    asset_server: Res<AssetServer>,
    q_bgm: Query<
        (&AudioSink, Entity, Option<&FadeSettings>),
        (With<BgmMarker>, Without<PlayerMarker>),
    >,
) {
    if ev_shard_animation.is_empty() {
        return;
    }
    let shard_info = ev_shard_animation.read().next().unwrap().0;

    shard_anim_cbs.for_shard = Some(shard_info);
    if *cur_game_state.get() == GameState::Animating {
        return;
    }
    let Ok((player_entity, player_transform)) = q_player.get_single() else {
        return;
    };
    commands.entity(player_entity).insert(InputLocked);

    commands.entity(player_entity).with_child((
        AudioPlayer::new(asset_server.load("sfx/shard_acquire.wav")),
        PlaybackSettings::DESPAWN,
    ));

    for (sink, bgm, fade_settings) in q_bgm.iter() {
        // FIXME: If the entity has FadeSettings::Despawn fade just let it despawn
        if fade_settings.is_some_and(|settings| *settings == FadeSettings::Despawn) {
            continue;
        }
        commands.entity(bgm).insert(Fade::new(
            SHARD_FADE_DURATION,
            sink.volume(),
            SHARD_FADE_VOLUME,
        ));
    }

    const SHARD_ANIMATION_CAMERA_SCALE: f32 = 0.75;

    ev_zoom_camera.send(CameraZoomEvent {
        scale: SHARD_ANIMATION_CAMERA_SCALE,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback: None,
        },
    });

    let camera_pos = camera_position_from_level_with_scale(
        current_level.level_box,
        player_transform.translation().xy(),
        SHARD_ANIMATION_CAMERA_SCALE,
    );

    ev_move_camera.send(CameraMoveEvent {
        to: camera_pos,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback: Some(shard_anim_cbs.cb[0]),
        },
    });
    next_game_state.set(GameState::Animating);
    next_anim_state.set(AnimationState::Shard);
}

#[derive(Component)]
pub struct ShardUiMarker;

pub fn on_shard_zoom_in_finished(
    mut commands: Commands,
    mut ev_zoom_camera: EventWriter<CameraMoveEvent>,
    q_camera: Query<(Entity, &GlobalTransform), With<MainCamera>>,
    shard_anim_cbs: Res<ShardAnimationCallbacks>,
    asset_server: Res<AssetServer>,
) {
    let (main_camera, camera_transform) = q_camera
        .get_single()
        .expect("Main camera should not die during shard transition");

    let (_, shard_color) = shard_anim_cbs
        .for_shard
        .expect("Shard animation should be for a shard");

    // zoom the camera position to the same spot for 5 seconds, showing text and waiting for the
    // sfx to finish
    ev_zoom_camera.send(CameraMoveEvent {
        to: camera_transform.translation().xy(),
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(5000),
            ease_fn: EaseFunction::SineInOut,
            callback: Some(shard_anim_cbs.cb[1]),
        },
    });

    let font = TextFont {
        font: asset_server.load("fonts/Munro.ttf"),
        ..default()
    };

    let shard_text = match shard_color {
        LightColor::Green => "Oh good, the first piece of the Divine Prism. This should let me shoot a bouncing light beam.",
        LightColor::Blue => "Blue light, formerly known as the light of harmony. I wonder what it'll do if I shoot it directly at the sensor above me?",
        LightColor::White => "A different feeling than before... could this color have a special property?",
        LightColor::Purple => "This one's even more powerful... I should be able to bounce this one more than once.",
        LightColor::Black => "Devs Only!!!",
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::End,
                ..default()
            },
            Visibility::Visible,
            ShardUiMarker,
            // spawn underneath the level select UI
            GlobalZIndex(-1),
            // show underneath screen transitions
            TargetCamera(main_camera),
        ))
        .with_child(Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_child((
            Node {
                width: Val::Auto,
                height: Val::Auto,
                margin: UiRect::all(Val::Px(24.)),
                ..default()
            },
            Text::new(shard_text),
            TextLayout::new_with_justify(JustifyText::Center),
            font.clone().with_font_size(36.),
        ));
}

#[allow(clippy::too_many_arguments)]
pub fn on_shard_text_read_finish(
    mut commands: Commands,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
    mut ev_zoom_camera: EventWriter<CameraZoomEvent>,
    mut current_level: ResMut<CurrentLevel>,
    mut q_player: Query<(&GlobalTransform, &mut PlayerLightInventory), With<PlayerMarker>>,
    q_shard_text: Query<Entity, With<ShardUiMarker>>,
    shard_anim_cbs: Res<ShardAnimationCallbacks>,
    q_bgm: Query<Entity, (With<BgmMarker>, Without<ShardUiMarker>)>,
) {
    let (player_transform, mut player_light_inventory) = q_player
        .get_single_mut()
        .expect("Player should not die during shard transition");
    let shard_text = q_shard_text
        .get_single()
        .expect("Shard text should not die during shard transition");
    let (shard_entity, shard_color) = shard_anim_cbs
        .for_shard
        .expect("Shard animation should be for a shard");

    commands.entity(shard_text).despawn_recursive();
    commands.entity(shard_entity).insert(Visibility::Hidden);
    player_light_inventory.current_color = Some(shard_color);
    current_level.allowed_colors[shard_color] = true;

    let camera_pos =
        camera_position_from_level(current_level.level_box, player_transform.translation().xy());

    ev_move_camera.send(CameraMoveEvent {
        to: camera_pos,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback: Some(shard_anim_cbs.cb[2]),
        },
    });
    ev_zoom_camera.send(CameraZoomEvent {
        scale: 1.,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback: None,
        },
    });

    for bgm in q_bgm.iter() {
        commands.entity(bgm).insert(Fade::new(
            SHARD_FADE_DURATION,
            SHARD_FADE_VOLUME,
            BGM_VOLUME,
        ));
    }
}

pub fn on_shard_zoom_back_finish(
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameState>>,
    q_player: Query<Entity, With<PlayerMarker>>,
) {
    let player_entity = q_player
        .get_single()
        .expect("Player should not die during shard transition");
    next_game_state.set(GameState::Playing);
    commands.entity(player_entity).remove::<InputLocked>();
}
