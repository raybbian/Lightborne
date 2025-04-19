use std::time::Duration;

use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BgmTracks>()
            .add_event::<ChangeBgmEvent>()
            .add_systems(Update, (handle_change_bgm_event, fade_bgm));
    }
}

#[derive(Component, Default)]
pub struct BgmMarker;

#[derive(Resource)]
pub struct BgmTracks {
    mustnt_stop: Handle<AudioSource>,
    light_in_the_dark: Handle<AudioSource>,
    cutscene_1_draft: Handle<AudioSource>,
    level_select: Handle<AudioSource>,
}

impl FromWorld for BgmTracks {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            mustnt_stop: asset_server.load("music/Mustn't Stop - M2 Version.mp3"),
            light_in_the_dark: asset_server.load("music/A Light in the Dark - Two Loops.mp3"),
            cutscene_1_draft: asset_server.load("music/lightborne cutscene 1 draft 2.mp3"),
            level_select: asset_server.load("music/main_menu.wav"),
        }
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum BgmTrack {
    MustntStop,
    LightInTheDark,
    Cutscene1Draft,
    LevelSelect,
    #[default]
    None,
}

pub const BGM_VOLUME: f32 = 0.8;

/// Fades out all other bgm tracks, and spawns the selected track
#[derive(Event)]
pub struct ChangeBgmEvent(pub BgmTrack);

pub fn handle_change_bgm_event(
    mut commands: Commands,
    mut ev_change_bgm: EventReader<ChangeBgmEvent>,
    q_active_tracks: Query<(Entity, &AudioSink), With<BgmMarker>>,
    tracks: Res<BgmTracks>,
    mut current_bgm: Local<BgmTrack>,
) {
    const BGM_FADE_DURATION: Duration = Duration::from_secs(3);
    if ev_change_bgm.is_empty() {
        return;
    }

    let evs = ev_change_bgm
        .read()
        .filter(|ev| ev.0 != *current_bgm)
        .collect::<Vec<&ChangeBgmEvent>>();

    if evs.is_empty() {
        return;
    }

    for (track, sink) in q_active_tracks.iter() {
        commands.entity(track).insert((
            Fade::new(BGM_FADE_DURATION, sink.volume(), 0.0),
            FadeSettings::Despawn,
        ));
    }

    for ev in evs.iter() {
        let source = match ev.0 {
            BgmTrack::MustntStop => tracks.mustnt_stop.clone(),
            BgmTrack::LightInTheDark => tracks.light_in_the_dark.clone(),
            BgmTrack::Cutscene1Draft => tracks.cutscene_1_draft.clone(),
            BgmTrack::LevelSelect => tracks.level_select.clone(),
            BgmTrack::None => continue,
        };

        commands.spawn((
            AudioPlayer::new(source),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::ZERO,
                ..default()
            },
            Fade::new(BGM_FADE_DURATION, 0.0, BGM_VOLUME),
            BgmMarker,
        ));

        // NOTE: only take first event
        *current_bgm = ev.0;
        break;
    }
}

#[derive(Component, Default, Debug, PartialEq, Eq)]
pub enum FadeSettings {
    Despawn,
    #[default]
    Continue,
}

#[derive(Component)]
#[require(FadeSettings)]
pub struct Fade {
    timer: Timer,
    from: f32,
    to: f32,
}

impl Fade {
    pub fn new(duration: Duration, from: f32, to: f32) -> Self {
        Self {
            timer: Timer::new(duration, TimerMode::Once),
            from,
            to,
        }
    }
}

fn fade_bgm(
    mut commands: Commands,
    mut audio_sink: Query<(&mut AudioSink, Entity, &mut Fade, &FadeSettings)>,
    time: Res<Time>,
    global_volume: Res<GlobalVolume>,
) {
    for (audio, entity, mut fade, fade_settings) in audio_sink.iter_mut() {
        fade.timer.tick(time.delta());
        let progress = fade.timer.elapsed_secs() / fade.timer.duration().as_secs_f32();
        audio.set_volume(fade.from.lerp(fade.to, progress) * global_volume.volume.get());
        if !fade.timer.just_finished() {
            continue;
        }

        // make sure its actually the end vol
        audio.set_volume(fade.to * global_volume.volume.get());

        match fade_settings {
            FadeSettings::Continue => {
                commands.entity(entity).remove::<Fade>();
            }
            FadeSettings::Despawn => {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
