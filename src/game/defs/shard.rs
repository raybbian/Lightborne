use std::time::Duration;

use avian2d::prelude::*;
use bevy::{audio::Volume, prelude::*};
use bevy_ecs_ldtk::prelude::*;

use crate::{
    asset::LoadResource,
    callback::Callback,
    camera::{CameraControlType, CameraMoveEvent, CameraZoomEvent},
    game::{
        animation::AnimationConfig,
        camera_op::{camera_position_from_level, camera_position_from_level_with_scale},
        dialogue::{Dialogue, DialogueAssets, DialogueEntry},
        light::LightColor,
        lighting::LineLight2d,
        lyra::{
            beam::{BeamAction, PlayerLightInventory, PlayerLightSaveData},
            Lyra,
        },
        Layers,
    },
    ldtk::{LdtkLevelParam, LevelExt},
    shared::{AnimationState, PlayState},
    sound::{BgmMarker, Fade, FadeSettings, BGM_VOLUME},
};

pub struct CrystalShardPlugin;

impl Plugin for CrystalShardPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ShardAssets>();
        app.load_resource::<ShardAssets>();
        app.init_resource::<ShardAnimationRes>();
        app.register_ldtk_entity::<CrystalShardBundle>("CrystalShard");
        app.add_observer(on_add_crystal_shard);
        app.add_observer(start_shard_animation);
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
}

#[derive(Resource, Reflect, Asset, Clone)]
#[reflect(Resource)]
pub struct ShardAssets {
    #[dependency]
    shards: Handle<Image>,
    #[dependency]
    sfx: Handle<AudioSource>,
}

impl FromWorld for ShardAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            shards: asset_server.load("crystal_shard_sheet.png"),
            sfx: asset_server.load("sfx/shard_acquire.wav"),
        }
    }
}

pub fn on_add_crystal_shard(
    event: On<Add, CrystalShard>,
    mut commands: Commands,
    q_crystal_shard: Query<&CrystalShard>,
    shard_assets: Res<ShardAssets>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    inventory: Single<&PlayerLightInventory, With<Lyra>>,
) {
    let Ok(shard) = q_crystal_shard.get(event.entity) else {
        return;
    };

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
        // LightColor::Black => 4,
    };

    let visibility = if inventory.allowed[shard.light_color] {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };

    let start_index = shard_row(shard) * CRYSTAL_SHARD_FRAMES;
    commands
        .entity(event.entity)
        .insert(LineLight2d::point(
            shard.light_color.lighting_color().extend(1.0),
            40.0,
            0.025,
        ))
        .insert(Collider::rectangle(12., 12.))
        .insert(CollisionLayers::new(
            Layers::SensorBox,
            Layers::PlayerHurtbox,
        ))
        .insert(Sensor)
        .insert(Sprite {
            image: shard_assets.shards.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout.clone(),
                index: start_index,
            }),
            ..default()
        })
        .insert(visibility)
        .insert(AnimationConfig::new(
            start_index,
            start_index + CRYSTAL_SHARD_FRAMES - 1,
            6,
            true,
        ));
}

pub fn on_player_intersect_shard(
    event: On<CollisionStart>,
    mut commands: Commands,
    q_shards: Query<(Entity, &CrystalShard, &Visibility)>,
) {
    let Ok((shard_entity, shard, shard_visibility)) = q_shards.get(event.collider2) else {
        return;
    };
    if shard_visibility == Visibility::Hidden {
        return;
    }
    commands.trigger(ShardAnimationEvent((shard_entity, shard.light_color)));
}

#[derive(Event)]
pub struct ShardAnimationEvent((Entity, LightColor));

#[derive(Resource, Default)]
pub struct ShardAnimationRes {
    shard: Option<Entity>,
    color: Option<LightColor>,
}

const SHARD_FADE_DURATION: Duration = Duration::from_millis(500);
const SHARD_FADE_VOLUME: f32 = 0.1;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn start_shard_animation(
    shard_info: On<ShardAnimationEvent>,
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<PlayState>>,
    mut next_anim_state: ResMut<NextState<AnimationState>>,
    lyra: Single<(Entity, &Transform), With<Lyra>>,
    ldtk_level_param: LdtkLevelParam,
    shard_assets: Res<ShardAssets>,
    q_bgm: Query<(&AudioSink, Entity, Option<&FadeSettings>), (With<BgmMarker>, Without<Lyra>)>,
    mut animation_res: ResMut<ShardAnimationRes>,
) {
    if animation_res.shard.is_some() {
        warn!("Duplicate shard animation");
        return;
    }
    animation_res.shard = Some(shard_info.0 .0);
    animation_res.color = Some(shard_info.0 .1);

    let (player_entity, player_transform) = lyra.into_inner();

    commands.entity(player_entity).with_child((
        AudioPlayer::new(shard_assets.sfx.clone()),
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
            Volume::Linear(SHARD_FADE_VOLUME),
        ));
    }

    const SHARD_ANIMATION_CAMERA_SCALE: f32 = 0.75;

    commands.trigger(CameraZoomEvent {
        scale: SHARD_ANIMATION_CAMERA_SCALE,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback_entity: None,
        },
    });

    let camera_pos = camera_position_from_level_with_scale(
        ldtk_level_param
            .cur_level()
            .expect("Cur level should exist")
            .raw()
            .level_box(),
        player_transform.translation.xy(),
        SHARD_ANIMATION_CAMERA_SCALE,
    );

    let after_zoom_in = commands.spawn(()).observe(on_shard_zoom_in_finished).id();

    commands.trigger(CameraMoveEvent {
        to: camera_pos,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback_entity: Some(after_zoom_in),
        },
    });
    next_game_state.set(PlayState::Animating);
    next_anim_state.set(AnimationState::InputLocked);
}

pub fn on_shard_zoom_in_finished(
    _: On<Callback>,
    mut commands: Commands,
    dialogue_assets: Res<DialogueAssets>,
    animation_res: Res<ShardAnimationRes>,
) {
    let on_dialogue_finish = commands.spawn(()).observe(on_shard_text_read_finish).id();

    let text = match animation_res.color.unwrap() {
        LightColor::Green => "Oh good, the first piece of the Divine Prism. This should let me shoot a bouncing light beam.",
        LightColor::Blue => "Blue light, formerly known as the light of harmony. Could this one shoot through the active blue crystals above me?",
        LightColor::White => "A different feeling than before... could this color have a special reflective properties?",
        LightColor::Purple => "This one's even more powerful... the purple light beam should bounce twice instead of once.",
    };

    commands.trigger(Dialogue {
        entries: vec![DialogueEntry {
            text: text.to_string(),
            image: dialogue_assets.lyra_happy.clone(),
        }],
        duration: Duration::from_millis(20),
        callback_entity: Some(on_dialogue_finish),
    });
}

#[allow(clippy::too_many_arguments)]
pub fn on_shard_text_read_finish(
    _: On<Callback>,
    mut commands: Commands,
    ldtk_level_param: LdtkLevelParam,
    mut light_save_data: ResMut<PlayerLightSaveData>,
    lyra: Single<(&Transform, &mut PlayerLightInventory), With<Lyra>>,
    q_bgm: Query<Entity, With<BgmMarker>>,
    mut ev_beam_action: MessageWriter<BeamAction>,
    animation_res: Res<ShardAnimationRes>,
) {
    let (lyra, mut inventory) = lyra.into_inner();
    ev_beam_action.write(BeamAction::SwitchColor(animation_res.color));

    commands
        .entity(animation_res.shard.unwrap())
        .insert(Visibility::Hidden);

    inventory.allowed[animation_res.color.unwrap()] = true;
    light_save_data.unlocked[animation_res.color.unwrap()] = true;

    let after_zoom_out = commands.spawn(()).observe(on_shard_zoom_back_finish).id();

    let camera_pos = camera_position_from_level(
        ldtk_level_param
            .cur_level()
            .expect("Cur level should exist")
            .raw()
            .level_box(),
        lyra.translation.xy(),
    );

    commands.trigger(CameraMoveEvent {
        to: camera_pos,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback_entity: Some(after_zoom_out),
        },
    });
    commands.trigger(CameraZoomEvent {
        scale: 1.,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback_entity: None,
        },
    });

    for bgm in q_bgm.iter() {
        commands.entity(bgm).insert(Fade::new(
            SHARD_FADE_DURATION,
            Volume::Linear(SHARD_FADE_VOLUME),
            BGM_VOLUME,
        ));
    }
}

pub fn on_shard_zoom_back_finish(
    _: On<Callback>,
    mut next_game_state: ResMut<NextState<PlayState>>,
    mut animation_res: ResMut<ShardAnimationRes>,
) {
    animation_res.color = None;
    animation_res.shard = None;
    next_game_state.set(PlayState::Playing);
}
