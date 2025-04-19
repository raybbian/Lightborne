use std::time::Duration;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    ecs::system::SystemId,
    prelude::*,
    render::{
        camera::{RenderTarget, ScalingMode},
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
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
            .add_event::<CameraZoomEvent>()
            .add_event::<CameraTransitionEvent>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                FixedUpdate,
                move_camera
                    .after(PhysicsSet::Writeback)
                    .after(switch_level)
                    .in_set(LevelSystems::Simulation),
            )
            .add_systems(
                Update,
                (
                    (
                        (handle_zoom_camera, handle_move_camera),
                        apply_camera_snapping,
                    )
                        .chain()
                        .after(move_camera)
                        .after(switch_level),
                    handle_transition_camera,
                ),
            );
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

pub const CAMERA_WIDTH: u32 = 320;
pub const CAMERA_HEIGHT: u32 = 180;
pub const CAMERA_ANIMATION_SECS: f32 = 0.4;

pub const TERRAIN_LAYER: RenderLayers = RenderLayers::layer(0);
pub const HIGHRES_LAYER: RenderLayers = RenderLayers::layer(2);
pub const TRANSITION_LAYER: RenderLayers = RenderLayers::layer(5);

#[derive(Component)]
pub struct TransitionCamera;

#[derive(Component)]
pub struct TransitionMeshMarker;

#[derive(Component)]
#[require(Transform)]
pub struct CameraPixelOffset(Vec2);

pub fn apply_camera_snapping(
    q_camera: Query<&Transform, With<MainCamera>>,
    mut q_match_camera: Query<(&mut Transform, &mut CameraPixelOffset), Without<MainCamera>>,
) {
    let Ok(main_camera_transform) = q_camera.get_single() else {
        return;
    };
    for (mut transform, mut pixel_offset) in q_match_camera.iter_mut() {
        pixel_offset.0 =
            (main_camera_transform.translation.round() - main_camera_transform.translation).xy();
        transform.translation = pixel_offset.0.extend(transform.translation.z);
    }
}

/// [`Startup`] [`System`] that spawns the [`Camera2d`] in the world.
///
/// Notes:
/// - Spawns the camera with [`OrthographicProjection`] with fixed scaling at 320x180
pub fn setup_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let projection = OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: CAMERA_WIDTH as f32,
            height: CAMERA_HEIGHT as f32,
        },
        ..OrthographicProjection::default_2d()
    };

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
        TRANSITION_LAYER,
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(CAMERA_WIDTH as f32, CAMERA_HEIGHT as f32))),
        MeshMaterial2d(materials.add(Color::BLACK)),
        // send to narnia, should be moved by any animations using it
        Transform::from_xyz(10000.0, 10000.0, 0.0),
        TransitionMeshMarker,
        TRANSITION_LAYER,
    ));

    let main_camera = commands
        .spawn((
            Camera2d,
            MainCamera,
            Camera {
                hdr: true,
                order: 1, // must be after the lowres layers
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Tonemapping::TonyMcMapface,
            projection.clone(),
            Transform::default(),
            HIGHRES_LAYER,
        ))
        .id();

    let canvas_size = Extent3d {
        width: CAMERA_WIDTH as u32 + 2,
        height: CAMERA_HEIGHT as u32 + 2,
        ..default()
    };

    let mut terrain = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    terrain.resize(canvas_size);
    let terrain_handle = images.add(terrain);

    let pixel_projection = OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: (CAMERA_WIDTH + 2) as f32,
            height: (CAMERA_HEIGHT + 2) as f32,
        },
        ..OrthographicProjection::default_2d()
    };

    commands.entity(main_camera).with_children(|child| {
        child
            .spawn((
                Camera2d,
                // MatchMainCameraTransform::Nearest,
                AmbientLight2d {
                    color: Vec4::new(1.0, 1.0, 1.0, 0.4),
                },
                Camera {
                    hdr: true,
                    order: 0,
                    target: RenderTarget::Image(terrain_handle.clone()),
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    ..default()
                },
                Tonemapping::TonyMcMapface,
                CameraPixelOffset(Vec2::ZERO),
                pixel_projection.clone(),
                Transform::default(),
                TERRAIN_LAYER,
            ))
            .with_child((Sprite::from_image(terrain_handle.clone()), HIGHRES_LAYER));

        child.spawn((
            Sprite::from_image(asset_server.load("levels/background.png")),
            HIGHRES_LAYER,
            Transform::from_xyz(0., 0., -5.),
        ));
    });
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

#[derive(Debug)]
pub enum CameraControlType {
    Animated {
        duration: Duration,
        ease_fn: EaseFunction,
        callback: Option<SystemId>,
    },
    Instant,
}

#[derive(Event, Debug)]
pub struct CameraMoveEvent {
    pub to: Vec2,
    pub variant: CameraControlType,
}

#[derive(Debug)]
pub struct CameraAnimationInfo<LERP: Sized> {
    progress: Timer,
    start: LERP,
    end: LERP,
    curve: EasingCurve<f32>,
    callback: Option<SystemId>,
}

#[derive(Event, Debug)]
pub struct CameraZoomEvent {
    pub scale: f32,
    pub variant: CameraControlType,
}

pub fn handle_transition_camera(
    mut commands: Commands,
    mut q_transition_mesh: Query<&mut Transform, With<TransitionMeshMarker>>,
    mut ev_transition_camera: EventReader<CameraTransitionEvent>,
    mut animation: Local<Option<CameraAnimationInfo<Vec3>>>,
    time: Res<Time>,
) {
    let Ok(mut mesh_transform) = q_transition_mesh.get_single_mut() else {
        return;
    };

    for event in ev_transition_camera.read() {
        let anim = match event.effect {
            CameraTransition::SlideFromBlack => CameraAnimationInfo {
                progress: Timer::new(event.duration, TimerMode::Once),
                start: Vec3::new(0.0, 0.0, 0.0),
                end: Vec3::new(0.0, -(CAMERA_HEIGHT as f32), 0.0),
                curve: EasingCurve::new(0.0, 1.0, event.ease_fn),
                callback: event.callback,
            },
            CameraTransition::SlideToBlack => CameraAnimationInfo {
                progress: Timer::new(event.duration, TimerMode::Once),
                start: Vec3::new(0.0, CAMERA_HEIGHT as f32, 0.0),
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

pub fn handle_zoom_camera(
    mut commands: Commands,
    mut q_camera: Query<&mut OrthographicProjection, With<MainCamera>>,
    mut ev_move_camera: EventReader<CameraZoomEvent>,
    mut animation: Local<Option<CameraAnimationInfo<f32>>>,
    time: Res<Time>,
) {
    let Ok(mut camera_projection) = q_camera.get_single_mut() else {
        return;
    };

    for event in ev_move_camera.read() {
        match event.variant {
            CameraControlType::Animated {
                duration,
                ease_fn,
                callback,
            } => {
                let anim = CameraAnimationInfo {
                    progress: Timer::new(duration, TimerMode::Once),
                    start: camera_projection.scale,
                    end: event.scale,
                    curve: EasingCurve::new(0.0, 1.0, ease_fn),
                    callback,
                };
                *animation = Some(anim);
            }
            CameraControlType::Instant => {
                camera_projection.scale = event.scale;
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
    camera_projection.scale = anim
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
    mut animation: Local<Option<CameraAnimationInfo<Vec3>>>,
    time: Res<Time>,
) {
    let Ok(mut camera_transform) = q_camera.get_single_mut() else {
        return;
    };

    for event in ev_move_camera.read() {
        match event.variant {
            CameraControlType::Animated {
                duration,
                ease_fn,
                callback,
            } => {
                let anim = CameraAnimationInfo {
                    progress: Timer::new(duration, TimerMode::Once),
                    start: camera_transform.translation,
                    end: event.to.extend(camera_transform.translation.z),
                    curve: EasingCurve::new(0.0, 1.0, ease_fn),
                    callback,
                };
                *animation = Some(anim);
            }
            CameraControlType::Instant => {
                camera_transform.translation = event.to.extend(camera_transform.translation.z);
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

pub fn camera_position_from_level_with_scale(
    level_box: Rect,
    player_pos: Vec2,
    camera_scale: f32,
) -> Vec2 {
    let (x_min, x_max) = (
        level_box.min.x + CAMERA_WIDTH as f32 * 0.5 * camera_scale,
        level_box.max.x - CAMERA_WIDTH as f32 * 0.5 * camera_scale,
    );
    let (y_min, y_max) = (
        level_box.min.y + CAMERA_HEIGHT as f32 * 0.5 * camera_scale,
        level_box.max.y - CAMERA_HEIGHT as f32 * 0.5 * camera_scale,
    );

    Vec2::new(
        player_pos.x.max(x_min).min(x_max),
        player_pos.y.max(y_min).min(y_max),
    )
}

pub fn camera_position_from_level(level_box: Rect, player_pos: Vec2) -> Vec2 {
    camera_position_from_level_with_scale(level_box, player_pos, 1.)
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
    ev_move_camera.send(CameraMoveEvent {
        to: camera_transform.translation.xy().lerp(camera_pos, 0.2),
        variant: CameraControlType::Instant,
    });
}
