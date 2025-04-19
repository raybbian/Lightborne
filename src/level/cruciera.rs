use std::{cmp::Ordering, time::Duration};

use bevy::{ecs::system::SystemId, prelude::*};
use bevy_ecs_ldtk::prelude::*;

use crate::{
    animation::AnimationConfig,
    camera::{
        camera_position_from_level, camera_position_from_level_with_scale, CameraControlType,
        CameraMoveEvent, CameraZoomEvent, HIGHRES_LAYER, TERRAIN_LAYER,
    },
    lighting::LineLight2d,
    player::{InputLocked, PlayerMarker},
    shared::{AnimationState, GameState},
};

use super::{CurrentLevel, LevelSystems};

pub struct CrucieraPlugin;

impl Plugin for CrucieraPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<LdtkCrucieraBundle>("Gala")
            .init_resource::<CrucieraCallbacks>()
            .add_systems(PreUpdate, setup_cruciera.in_set(LevelSystems::Processing))
            .add_systems(
                Update,
                lyra_cruciera_dialogue.run_if(in_state(AnimationState::CrucieraDialogue)),
            )
            .add_systems(
                Update,
                reset_cruciera_on_level_switch.in_set(LevelSystems::Reset),
            )
            .add_systems(
                FixedUpdate,
                check_start_cutscene.in_set(LevelSystems::Simulation),
            );
    }
}

#[derive(Component)]
pub struct Cruciera {
    played_cutscene: bool,
}

#[derive(Bundle, LdtkEntity)]
pub struct LdtkCrucieraBundle {
    animation_config: AnimationConfig,
    cruciera: Cruciera,
}

impl Default for LdtkCrucieraBundle {
    fn default() -> Self {
        Self {
            animation_config: AnimationConfig::new(0, 2, 5, true),
            cruciera: Cruciera {
                played_cutscene: false,
            },
        }
    }
}

pub fn setup_cruciera(
    mut commands: Commands,
    q_added_cruciera: Query<Entity, Added<Cruciera>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Ok(cruciera) = q_added_cruciera.get_single() else {
        return;
    };

    let texture_atlas_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(15, 23),
        3,
        1,
        None,
        None,
    ));

    // insert sprite here because it depends on texture atlas which needs a resource
    commands
        .entity(cruciera)
        .insert((
            Sprite {
                image: asset_server.load("gala_sheet.png"),
                texture_atlas: Some(TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                }),
                ..default()
            },
            HIGHRES_LAYER,
        ))
        .with_child((
            LineLight2d::point(Vec4::new(1.0, 0.2, 0.2, 0.8), 40., 0.008),
            TERRAIN_LAYER,
        ));
}

#[derive(Resource)]
pub struct CrucieraCallbacks {
    start_dialogue: SystemId,
    cur_dialogue: usize,
    end_dialogue: SystemId,
    reset_state: SystemId,
}

impl FromWorld for CrucieraCallbacks {
    fn from_world(world: &mut World) -> Self {
        Self {
            start_dialogue: world.register_system(setup_dialogue_box),
            cur_dialogue: 0,
            end_dialogue: world.register_system(end_dialogue),
            reset_state: world.register_system(reset_state),
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn check_start_cutscene(
    mut commands: Commands,
    mut q_cruciera: Query<(&GlobalTransform, &mut Cruciera)>,
    q_lyra: Query<(Entity, &GlobalTransform), (With<PlayerMarker>, Without<Cruciera>)>,
    current_level: Res<CurrentLevel>,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
    mut ev_zoom_camera: EventWriter<CameraZoomEvent>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_anim_state: ResMut<NextState<AnimationState>>,
    callbacks: Res<CrucieraCallbacks>,
    cur_game_state: Res<State<GameState>>,
) {
    if *cur_game_state.get() == GameState::Animating {
        return;
    }
    let Ok((cruciera_transform, mut cruciera)) = q_cruciera.get_single_mut() else {
        return;
    };
    let Ok((lyra_entity, lyra_transform)) = q_lyra.get_single() else {
        return;
    };

    if cruciera_transform
        .translation()
        .distance(lyra_transform.translation())
        < 40.
        && !cruciera.played_cutscene
    {
        cruciera.played_cutscene = true;
        commands.entity(lyra_entity).insert(InputLocked);
        const CUTSCENE_CAMERA_SCALE: f32 = 0.75;

        ev_zoom_camera.send(CameraZoomEvent {
            scale: CUTSCENE_CAMERA_SCALE,
            variant: CameraControlType::Animated {
                duration: Duration::from_millis(500),
                ease_fn: EaseFunction::SineInOut,
                callback: Some(callbacks.start_dialogue),
            },
        });

        let camera_pos = camera_position_from_level_with_scale(
            current_level.level_box,
            lyra_transform.translation().xy(),
            CUTSCENE_CAMERA_SCALE,
        );

        ev_move_camera.send(CameraMoveEvent {
            to: camera_pos,
            variant: CameraControlType::Animated {
                duration: Duration::from_millis(500),
                ease_fn: EaseFunction::SineInOut,
                callback: None,
            },
        });

        next_game_state.set(GameState::Animating);
        next_anim_state.set(AnimationState::Cruciera);
    }
}

#[derive(Component)]
pub struct DialogueBoxMarker;

#[derive(Component)]
pub struct DialogueTextMarker;

#[derive(Component)]
pub struct DialogueImageMarker;

pub fn setup_dialogue_box(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_anim_state: ResMut<NextState<AnimationState>>,
) {
    let font = TextFont {
        font: asset_server.load("fonts/Outfit-Medium.ttf"),
        ..default()
    };

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                padding: UiRect::all(Val::Px(32.)),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            DialogueBoxMarker,
        ))
        .with_children(|container| {
            container
                .spawn((
                    Node {
                        width: Val::Percent(100.),
                        max_width: Val::Px(1280.),
                        height: Val::Auto,
                        aspect_ratio: Some(2775. / 630.), // FIXME: magic values!
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::new(
                            Val::Px(200.),
                            Val::Px(200.),
                            Val::Px(32.),
                            Val::Px(32.),
                        ),
                        ..default()
                    },
                    DialogueImageMarker,
                    ImageNode::new(asset_server.load("dialogue-box-cruciera.png")),
                ))
                .with_children(|text_box| {
                    text_box.spawn((
                        Node::default(),
                        font.clone().with_font_size(24.),
                        TextLayout::new_with_justify(JustifyText::Center),
                        Text::new(""),
                        DialogueTextMarker,
                    ));
                });
        });
    next_anim_state.set(AnimationState::CrucieraDialogue);
}

#[derive(Clone, Copy)]
pub enum DialoguePortrait {
    LyraNeutral,
    LyraHappy,
    LyraSad,
    Cruciera,
}

pub static LYRA_CRUCIERA_DIALOGUE: [(DialoguePortrait, &str); 11] = [
    (DialoguePortrait::LyraNeutral, "Did you call for me, Lady Cruciera?"),
    (DialoguePortrait::Cruciera, "Indeed I have, young Lyra. I have one final job for you."),
    (DialoguePortrait::LyraHappy, "A job? What would you like me to do, Lady Cruciera?"),
    (DialoguePortrait::Cruciera, "I ask that you bring back the Divine Prism we have granted those... foolish humans down below."),
    (DialoguePortrait::LyraSad, "You wish for me to retrieve the Divine Prism? But I thought that it was a gift to the humans? Without it, they'll be misguided..."),
    (DialoguePortrait::Cruciera, "It was a gift, but they were misguided even with it in their possession."),
    (DialoguePortrait::Cruciera, "Their greed has split the Prism into pieces, and such pieces have been scattered across their realm."),
    (DialoguePortrait::Cruciera, "The task falls upon you, young Lyra. Those who corrupted such a relic don't deserve to keep it."),
    (DialoguePortrait::LyraNeutral, "Very well, Lady Cruciera. I'll try my best to retrieve the pieces."),
    (DialoguePortrait::Cruciera, "It won't be as easy as you think, but it will be a good chance for you to experience the full power of a goddess."),
    (DialoguePortrait::Cruciera, "The prism is strong, yet volatile. Harness its powers well and pass its trials - that is the only way you will understand the true meaning of responsibility."),
];

impl From<DialoguePortrait> for String {
    fn from(value: DialoguePortrait) -> Self {
        match value {
            DialoguePortrait::LyraSad => "dialogue-box-lyra-sad.png".to_string(),
            DialoguePortrait::LyraHappy => "dialogue-box-lyra-happy.png".to_string(),
            DialoguePortrait::LyraNeutral => "dialogue-box-lyra-neutral.png".to_string(),
            DialoguePortrait::Cruciera => "dialogue-box-cruciera.png".to_string(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn lyra_cruciera_dialogue(
    mut commands: Commands,
    mut q_dialogue_text: Query<&mut Text, With<DialogueTextMarker>>,
    mut q_dialogue_image: Query<&mut ImageNode, With<DialogueImageMarker>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut callbacks: ResMut<CrucieraCallbacks>,
    mut timer: Local<Option<Timer>>,
    time: Res<Time>,
    mut next_anim_state: ResMut<NextState<AnimationState>>,
    asset_server: Res<AssetServer>,
) {
    let mut text = q_dialogue_text
        .get_single_mut()
        .expect("Dialogue text should exist!");
    let mut image = q_dialogue_image
        .get_single_mut()
        .expect("Dialogue image should exist!");

    if (*timer).is_none() {
        *timer = Some(Timer::new(Duration::from_millis(20), TimerMode::Repeating));

        // FIXME: copied reinit
        image.image = asset_server.load(String::from(
            LYRA_CRUCIERA_DIALOGUE[callbacks.cur_dialogue].0,
        ))
    }

    if keys.any_just_pressed([KeyCode::Space, KeyCode::Enter])
        || mouse.just_pressed(MouseButton::Left)
    {
        match text
            .len()
            .cmp(&LYRA_CRUCIERA_DIALOGUE[callbacks.cur_dialogue].1.len())
        {
            Ordering::Less => {
                //if animating the text rn, display it fully
                *text = LYRA_CRUCIERA_DIALOGUE[callbacks.cur_dialogue].1.into();
            }
            Ordering::Equal => {
                // if done animating, move on to next
                if callbacks.cur_dialogue + 1 >= LYRA_CRUCIERA_DIALOGUE.len() {
                    // if done with all text, end the dialogue
                    next_anim_state.set(AnimationState::Cruciera);
                    commands.run_system(callbacks.end_dialogue);
                } else {
                    // otherwise, keep running the dialogue runner
                    callbacks.cur_dialogue += 1;
                    *text = "".into();
                    image.image = asset_server.load(String::from(
                        LYRA_CRUCIERA_DIALOGUE[callbacks.cur_dialogue].0,
                    ));
                }
            }
            _ => {}
        }
        return;
    }

    timer.as_mut().unwrap().tick(time.delta());
    // normal function call, animate text and then  update it
    if text.len() < LYRA_CRUCIERA_DIALOGUE[callbacks.cur_dialogue].1.len()
        && timer.as_ref().unwrap().just_finished()
    {
        *text = LYRA_CRUCIERA_DIALOGUE[callbacks.cur_dialogue].1[0..text.len() + 1].into();
    }
}

pub fn reset_cruciera_on_level_switch(mut q_cruciera: Query<&mut Cruciera>) {
    let Ok(mut cruciera) = q_cruciera.get_single_mut() else {
        return;
    };
    cruciera.played_cutscene = false;
}

pub fn end_dialogue(
    mut commands: Commands,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
    mut ev_zoom_camera: EventWriter<CameraZoomEvent>,
    current_level: ResMut<CurrentLevel>,
    q_player: Query<&GlobalTransform, With<PlayerMarker>>,
    q_dialogue_box: Query<Entity, With<DialogueBoxMarker>>,
    mut callbacks: ResMut<CrucieraCallbacks>,
) {
    let dialogue_box = q_dialogue_box
        .get_single()
        .expect("Dialogue box should not die during cutscene");
    let player_transform = q_player
        .get_single()
        .expect("Player should not die during cutscene");

    commands.entity(dialogue_box).despawn_recursive();

    let camera_pos =
        camera_position_from_level(current_level.level_box, player_transform.translation().xy());

    ev_move_camera.send(CameraMoveEvent {
        to: camera_pos,
        variant: CameraControlType::Animated {
            duration: Duration::from_millis(500),
            ease_fn: EaseFunction::SineInOut,
            callback: Some(callbacks.reset_state),
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
    callbacks.cur_dialogue = 0;
}

pub fn reset_state(
    mut commands: Commands,
    mut next_game_state: ResMut<NextState<GameState>>,
    q_player: Query<Entity, With<PlayerMarker>>,
) {
    let player_entity = q_player
        .get_single()
        .expect("Player should not die during cutscene");
    next_game_state.set(GameState::Playing);
    commands.entity(player_entity).remove::<InputLocked>();
}
