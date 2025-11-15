use std::time::Duration;

use avian2d::prelude::{CollisionStart, Position};
use bevy::{input::common_conditions::input_just_pressed, prelude::*};

use crate::{
    asset::LoadResource,
    callback::Callback,
    camera::{CameraTransition, CameraTransitionEvent},
    game::{
        camera_op::SnapToLyra,
        defs::DangerBox,
        lyra::{lyra_spawn_transform, Lyra},
    },
    ldtk::LdtkLevelParam,
    shared::{AnimationState, PlayState, ResetLevels},
};

pub struct LyraKillPlugin;

impl Plugin for LyraKillPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<KillSfx>();
        app.load_resource::<KillSfx>();
        app.add_observer(start_kill_animation);
        app.add_observer(play_death_sound_on_kill);
        app.add_systems(
            Update,
            quick_reset
                .run_if(input_just_pressed(KeyCode::KeyR))
                .run_if(in_state(PlayState::Playing)),
        );
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct KillSfx {
    sfx: Handle<AudioSource>,
}

impl FromWorld for KillSfx {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            sfx: asset_server.load("sfx/death.wav"),
        }
    }
}

pub fn quick_reset(mut commands: Commands) {
    commands.trigger(KillPlayer);
}

pub fn play_death_sound_on_kill(
    _: On<KillPlayer>,
    mut commands: Commands,
    lyra: Single<Entity, With<Lyra>>,
    kill_sfx: Res<KillSfx>,
) {
    commands.entity(*lyra).with_child((
        AudioPlayer::new(kill_sfx.sfx.clone()),
        PlaybackSettings::DESPAWN,
    ));
}

#[derive(Event)]
pub struct KillPlayer;

pub fn kill_player_on_danger(
    event: On<CollisionStart>,
    mut commands: Commands,
    q_danger_box: Query<&DangerBox>,
) {
    if q_danger_box.get(event.collider2).is_err() {
        return;
    }
    commands.trigger(KillPlayer);
}

pub fn start_kill_animation(
    _: On<KillPlayer>,
    mut commands: Commands,
    play_state: ResMut<State<PlayState>>,
    mut next_play_state: ResMut<NextState<PlayState>>,
    mut next_anim_state: ResMut<NextState<AnimationState>>,
) {
    if *play_state != PlayState::Playing {
        return;
    }
    info!("Killing player!");
    let cb1 = commands
        .spawn(())
        .observe(
            |_: On<Callback>,
             mut commands: Commands,
             lyra: Single<(&mut Transform, &mut Position), With<Lyra>>,
             ldtk_level_param: LdtkLevelParam| {
                let (mut transform, mut position) = lyra.into_inner();
                let cb2 = commands
                    .spawn(())
                    .observe(
                        |_: On<Callback>, mut next_play_state: ResMut<NextState<PlayState>>| {
                            next_play_state.set(PlayState::Playing);
                        },
                    )
                    .id();

                commands.trigger(CameraTransitionEvent {
                    duration: Duration::from_millis(400),
                    ease_fn: EaseFunction::SineInOut,
                    callback_entity: Some(cb2),
                    effect: CameraTransition::SlideFromBlack,
                });
                commands.trigger(ResetLevels);

                let lyra_transform = lyra_spawn_transform(&ldtk_level_param);
                *transform = Transform::from_translation(lyra_transform);
                *position = Position(lyra_transform.truncate());
                info!("Moving lyra to {}", lyra_transform);

                commands.trigger(SnapToLyra);
            },
        )
        .id();

    commands.trigger(CameraTransitionEvent {
        duration: Duration::from_millis(400),
        ease_fn: EaseFunction::SineInOut,
        callback_entity: Some(cb1),
        effect: CameraTransition::SlideToBlack,
    });

    next_play_state.set(PlayState::Animating);
    next_anim_state.set(AnimationState::Frozen);
}
