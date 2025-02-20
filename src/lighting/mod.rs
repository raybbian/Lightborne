use core::f32;

use bevy::{prelude::*, render::view::RenderLayers, sprite::Material2dPlugin};
use light::{draw_lights, LineLighting, PointLighting};
use material::{BlurMaterial, CombineFramesMaterial, FrameMaskMaterial, GradientLightMaterial};

use occluder::OccluderPlugin;
use render::LightingRenderData;

use crate::{
    camera::{move_camera, SyncWithMainCamera},
    player::match_player::MatchPlayerZ,
};

const SHOW_DEBUG_FRAMES_SPRITE: bool = true;

pub mod light;
mod material;
pub mod occluder;
mod render;

pub struct LightingPlugin;
impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GradientLightMaterial>::default())
            .add_plugins(Material2dPlugin::<CombineFramesMaterial>::default())
            .add_plugins(Material2dPlugin::<FrameMaskMaterial>::default())
            .add_plugins(Material2dPlugin::<BlurMaterial>::default())
            .add_plugins(OccluderPlugin)
            .init_resource::<LightingRenderData>()
            .add_systems(Startup, setup)
            .add_systems(Update, debug_controls)
            .add_systems(PostUpdate, draw_lights.after(move_camera));
    }
}

#[derive(Component)]
pub struct DebugFramesSprite;

const FRAMES_LAYER: RenderLayers = RenderLayers::layer(2);
const COMBINED_FRAMES_LAYER: RenderLayers = RenderLayers::layer(3);
const BLURRED_LAYER: RenderLayers = RenderLayers::layer(4);
const OCCLUDER_LAYER: RenderLayers = RenderLayers::layer(6);

fn debug_controls(
    mut q_debug: Query<&mut Sprite, With<DebugFramesSprite>>,
    keys: Res<ButtonInput<KeyCode>>,
    render_data: Res<LightingRenderData>,
) {
    let Ok(mut debug) = q_debug.get_single_mut() else {
        return;
    };

    if keys.just_pressed(KeyCode::Digit1) {
        debug.image = render_data.foreground_mask.clone();
    } else if keys.just_pressed(KeyCode::Digit2) {
        debug.image = render_data.background_mask.clone();
    } else if keys.just_pressed(KeyCode::Digit3) {
        debug.image = render_data.intensity_mask.clone();
    } else if keys.just_pressed(KeyCode::Digit4) {
        debug.image = render_data.combined_frames_image.clone();
    } else if keys.just_pressed(KeyCode::Digit5) {
        debug.image = render_data.blurred_image.clone();
    } else if keys.just_pressed(KeyCode::Digit6) {
        debug.image = render_data.occluder_mask.clone();
    }
}

fn setup(mut commands: Commands, lighting_render_data: Res<LightingRenderData>) {
    commands.spawn((
        Camera2d::default(),
        Camera {
            target: lighting_render_data.blurred_image.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        Transform::default(),
        BLURRED_LAYER,
    ));

    commands.spawn((
        Camera2d,
        SyncWithMainCamera,
        Camera {
            target: lighting_render_data.foreground_mask.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        RenderLayers::layer(5),
        Transform::default(),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            target: lighting_render_data.occluder_mask.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        OCCLUDER_LAYER,
        Transform::default(),
    ));

    commands.spawn((
        Camera2d::default(),
        Camera {
            target: lighting_render_data.combined_frames_image.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        Transform::default(),
        COMBINED_FRAMES_LAYER,
    ));

    commands.spawn((
        Camera2d::default(),
        Camera {
            target: lighting_render_data.intensity_mask.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        Transform::default(),
        FRAMES_LAYER,
    ));

    if SHOW_DEBUG_FRAMES_SPRITE {
        commands.spawn((
            Sprite {
                image: lighting_render_data.intensity_mask.clone(),
                custom_size: Some(Vec2::new(320.0 / 4.0, 180.0 / 4.0)),
                ..default()
            },
            Transform::default(),
            SyncWithMainCamera,
            DebugFramesSprite,
            MatchPlayerZ { offset: 2.0 },
        ));
    }

    commands.spawn((
        Mesh2d(lighting_render_data.gradient_mesh.clone()),
        MeshMaterial2d(lighting_render_data.gradient_material.clone()),
        Transform::default(),
        FRAMES_LAYER.clone(),
    ));

    commands.spawn((
        Mesh2d(lighting_render_data.combine_frames_mesh.clone()),
        MeshMaterial2d(lighting_render_data.combine_frames_material.clone()),
        Transform::default(),
        COMBINED_FRAMES_LAYER.clone(),
    ));

    commands.spawn((
        Mesh2d(lighting_render_data.blur_mesh.clone()),
        MeshMaterial2d(lighting_render_data.blur_material.clone()),
        Transform::default(),
        BLURRED_LAYER.clone(),
    ));

    commands.spawn((
        Sprite::from_image(lighting_render_data.combined_frames_image.clone()),
        RenderLayers::layer(7),
        Visibility::Visible,
        Transform::default(),
    ));
}

/// Struct used to represent both LineLighting and PointLighting in a unified way when drawing lights and shadows.
struct CombinedLighting {
    pub pos_1: Vec2,
    pub pos_2: Vec2,
    pub radius: f32,
    pub color: Vec3,
}

fn combine_lights(
    q_line_lights: Query<(&GlobalTransform, &Visibility, &LineLighting)>,
    q_point_lights: Query<(&GlobalTransform, &Visibility, &PointLighting)>,
    amount: usize,
) -> Vec<CombinedLighting> {
    q_line_lights
        .iter()
        .map(|(transform, visibility, line_light)| {
            let unit_vec = transform
                .rotation()
                .mul_vec3(Vec3::new(1.0, 0.0, 0.0))
                .truncate();
            let pos_1 = transform.translation().truncate() + unit_vec * transform.scale().x / 2.;
            let pos_2 = transform.translation().truncate() - unit_vec * transform.scale().x / 2.;
            (
                CombinedLighting {
                    pos_1,
                    pos_2,
                    radius: line_light.radius,
                    color: line_light.color,
                },
                visibility,
            )
        })
        .chain(
            q_point_lights
                .iter()
                .map(|(transform, visibility, point_light)| {
                    let pos = transform.translation().truncate();
                    (
                        CombinedLighting {
                            pos_1: pos,
                            pos_2: pos,
                            radius: point_light.radius,
                            color: point_light.color,
                        },
                        visibility,
                    )
                }),
        )
        .filter(|(_, &visibility)| visibility != Visibility::Hidden)
        .map(|(x, _)| x)
        .take(amount)
        .collect::<Vec<_>>()
}
