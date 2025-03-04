use std::time::Duration;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    ecs::system::SystemId,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
};
use bevy_rapier2d::plugin::PhysicsSet;

use crate::{
    level::{switch_level, CurrentLevel, LevelSystems},
    lighting::AmbientLight2d,
    player::PlayerMarker,
};

/// The [`Plugin`] responsible for handling anything Camera related.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CameraMoveEvent>()
            .add_event::<CameraTransitionEvent>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                FixedUpdate,
                move_camera
                    .after(PhysicsSet::Writeback)
                    .after(switch_level)
                    .in_set(LevelSystems::Simulation),
            )
            // Has event reader, so place in update
            .add_systems(
                Update,
                handle_move_camera.after(move_camera).after(switch_level),
            )
            .add_systems(Update, handle_transition_camera);
    }
}

/// Marker [`Component`] used to query for the main camera in the world.
///
/// Your query might look like this:
/// ```rust
/// Query<&Transform, With<MainCamera>>
/// ```
#[derive(Component, Default)]
pub struct MainCamera;

/// Marker [`Component`] used to query for the background camera. Note that for an entity to be
/// rendered on this Camera, it must be given the `RenderLayers::layer(1)` component.
#[derive(Component, Default)]
pub struct BackgroundCamera;

pub const CAMERA_WIDTH: f32 = 320.;
pub const CAMERA_HEIGHT: f32 = 180.;
pub const CAMERA_ANIMATION_SECS: f32 = 0.4;

#[derive(Component)]
pub struct TransitionCamera;

#[derive(Component)]
pub struct TransitionMeshMarker;

pub const TRANSITION_CAMERA_LAYER: RenderLayers = RenderLayers::layer(5);

/// [`Startup`] [`System`] that spawns the [`Camera2d`] in the world.
///
/// Notes:
/// - Spawns the camera with [`OrthographicProjection`] with fixed scaling at 320x180
fn setup_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let projection = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: CAMERA_WIDTH,
            height: CAMERA_HEIGHT,
        },
        ..OrthographicProjection::default_2d()
    });

    commands.spawn((
        Camera2d,
        TransitionCamera,
        Camera {
            hdr: true,
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        projection.clone(),
        Transform::default(),
        TRANSITION_CAMERA_LAYER,
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(320.0, 180.0))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        // send to narnia, should be moved by any animations using it
        Transform::from_xyz(10000.0, 10000.0, 0.0),
        TransitionMeshMarker,
        TRANSITION_CAMERA_LAYER,
    ));

    commands.spawn((
        Camera2d,
        MainCamera,
        AmbientLight2d {
            color: Vec4::new(1.0, 1.0, 1.0, 0.4),
        },
        Camera {
            hdr: true,
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        // Bloom::default(),
        projection.clone(),
        Transform::default(),
    ));

    commands.spawn((
        Camera2d,
        BackgroundCamera,
        Camera {
            hdr: true, // If Cameras mix HDR and non-HDR, then weird ass stuff happens. Seems like
            // https://github.com/bevyengine/bevy/pull/13419 was only a partial fix
            ..default()
        },
        projection,
        RenderLayers::layer(1),
        Transform::default(),
    ));
}

#[derive(Debug)]
pub enum CameraTransition {
    SlideToBlack,
    SlideFromBlack,
}

#[derive(Event, Debug)]
pub struct CameraTransitionEvent {
    pub duration: Duration,
    pub ease_fn: EaseFunction,
    pub callback: Option<SystemId>,
    pub effect: CameraTransition,
}

#[derive(Event, Debug)]
pub enum CameraMoveEvent {
    Animated {
        to: Vec2,
        duration: Duration,
        // start and end use seconds
        ease_fn: EaseFunction,
        callback: Option<SystemId>,
    },
    Instant {
        to: Vec2,
    },
}

#[derive(Debug)]
pub struct Animation {
    progress: Timer,
    start: Vec3,
    end: Vec3,
    // start and end use seconds
    curve: EasingCurve<f32>,
    callback: Option<SystemId>,
}

pub fn handle_transition_camera(
    mut commands: Commands,
    mut q_transition_mesh: Query<&mut Transform, With<TransitionMeshMarker>>,
    mut ev_transition_camera: EventReader<CameraTransitionEvent>,
    mut animation: Local<Option<Animation>>,
    time: Res<Time>,
) {
    let Ok(mut mesh_transform) = q_transition_mesh.get_single_mut() else {
        return;
    };

    for event in ev_transition_camera.read() {
        let anim = match event.effect {
            CameraTransition::SlideFromBlack => Animation {
                progress: Timer::new(event.duration, TimerMode::Once),
                start: Vec3::new(0.0, 0.0, 0.0),
                end: Vec3::new(0.0, -CAMERA_HEIGHT, 0.0),
                curve: EasingCurve::new(0.0, 1.0, event.ease_fn),
                callback: event.callback,
            },
            CameraTransition::SlideToBlack => Animation {
                progress: Timer::new(event.duration, TimerMode::Once),
                start: Vec3::new(0.0, CAMERA_HEIGHT, 0.0),
                end: Vec3::new(0.0, 0.0, 0.0),
                curve: EasingCurve::new(0.0, 1.0, event.ease_fn),
                callback: event.callback,
            },
        };
        *animation = Some(anim);
    }

    let Some(anim) = &mut *animation else {
        return;
    };

    anim.progress.tick(time.delta());

    let percent = anim.progress.elapsed_secs() / anim.progress.duration().as_secs_f32();

    mesh_transform.translation = anim
        .start
        .lerp(anim.end, anim.curve.sample_clamped(percent));

    if anim.progress.just_finished() {
        if anim.callback.is_some() {
            commands.run_system(anim.callback.unwrap());
        }
        *animation = None;
    }
}

pub fn handle_move_camera(
    mut commands: Commands,
    mut q_camera: Query<&mut Transform, With<MainCamera>>,
    mut ev_move_camera: EventReader<CameraMoveEvent>,
    mut animation: Local<Option<Animation>>,
    time: Res<Time>,
) {
    let Ok(mut camera_transform) = q_camera.get_single_mut() else {
        return;
    };

    for event in ev_move_camera.read() {
        match event {
            CameraMoveEvent::Animated {
                to,
                duration,
                ease_fn,
                callback,
            } => {
                let anim = Animation {
                    progress: Timer::new(*duration, TimerMode::Once),
                    start: camera_transform.translation,
                    end: to.extend(camera_transform.translation.z),
                    curve: EasingCurve::new(0.0, 1.0, *ease_fn),
                    callback: *callback,
                };
                *animation = Some(anim);
            }
            CameraMoveEvent::Instant { to } => {
                camera_transform.translation = to.extend(camera_transform.translation.z);
            }
        }
    }

    // This is a reborrow, something that treats Bevy's "smart pointers" as actual Rust references,
    // which allows you to do the things you are supposed to (like pattern match on them).
    let Some(anim) = &mut *animation else {
        return;
    };

    anim.progress.tick(time.delta());

    let percent = anim.progress.elapsed_secs() / anim.progress.duration().as_secs_f32();
    camera_transform.translation = anim
        .start
        .lerp(anim.end, anim.curve.sample_clamped(percent));

    if anim.progress.just_finished() {
        if anim.callback.is_some() {
            commands.run_system(anim.callback.unwrap());
        }
        *animation = None;
    }
}

pub fn camera_position_from_level(level_box: Rect, player_pos: Vec2) -> Vec2 {
    let (x_min, x_max) = (
        level_box.min.x + CAMERA_WIDTH * 0.5,
        level_box.max.x - CAMERA_WIDTH * 0.5,
    );
    let (y_min, y_max) = (
        level_box.min.y + CAMERA_HEIGHT * 0.5,
        level_box.max.y - CAMERA_HEIGHT * 0.5,
    );

    let new_pos = Vec2::new(
        player_pos.x.max(x_min).min(x_max),
        player_pos.y.max(y_min).min(y_max),
    );
    new_pos
}

/// [`System`] that moves camera to player's position and constrains it to the [`CurrentLevel`]'s `world_box`.
pub fn move_camera(
    current_level: Res<CurrentLevel>,
    q_player: Query<&Transform, With<PlayerMarker>>,
    q_camera: Query<&Transform, With<MainCamera>>,
    mut ev_move_camera: EventWriter<CameraMoveEvent>,
) {
    let Ok(player_transform) = q_player.get_single() else {
        return;
    };
    let Ok(camera_transform) = q_camera.get_single() else {
        return;
    };

    let camera_pos =
        camera_position_from_level(current_level.level_box, player_transform.translation.xy());
    ev_move_camera.send(CameraMoveEvent::Instant {
        to: camera_transform.translation.xy().lerp(camera_pos, 0.2),
    });
}
