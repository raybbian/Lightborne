use core::f32;

use bevy::{prelude::*, render::view::RenderLayers, sprite::Material2dPlugin};
use light::{draw_lights, LineLighting, PointLighting};
use material::{
    BackgroundMaterial, BlurMaterial, CombineFramesMaterial, FrameMaskMaterial,
    GradientLightMaterial,
};

use occluder::OccluderPlugin;
use render::LightingRenderData;

use crate::{
    camera::{move_camera, MainCamera},
    player::match_player::MatchPlayerZ,
};

const SHOW_DEBUG_FRAMES_SPRITE: bool = false;

pub mod light;
mod material;
pub mod occluder;
mod phase;
mod pipeline;
mod render;

pub struct LightingPlugin;
impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GradientLightMaterial>::default())
            .add_plugins(Material2dPlugin::<CombineFramesMaterial>::default())
            .add_plugins(Material2dPlugin::<FrameMaskMaterial>::default())
            .add_plugins(Material2dPlugin::<BlurMaterial>::default())
            .add_plugins(Material2dPlugin::<BackgroundMaterial>::default())
            .add_plugins(OccluderPlugin)
            .init_resource::<LightingRenderData>()
            .add_systems(Startup, setup)
            .add_systems(PostUpdate, update_debug_frames_sprite.after(move_camera))
            .add_systems(PostUpdate, draw_lights.after(move_camera));
    }
}

#[derive(Component)]
pub struct DebugFramesSprite;

#[derive(Component)]
pub struct BackgroundMarker;

const BACKGROUND_LAYER: RenderLayers = RenderLayers::layer(1);
const FRAMES_LAYER: RenderLayers = RenderLayers::layer(2);
const COMBINED_FRAMES_LAYER: RenderLayers = RenderLayers::layer(3);
const BLURRED_LAYER: RenderLayers = RenderLayers::layer(4);
fn setup(mut commands: Commands, lighting_render_data: Res<LightingRenderData>) {
    commands.spawn((
        Camera2d,
        Camera {
            target: lighting_render_data.blurred_image.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        Transform::default(),
        BLURRED_LAYER.clone(),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            target: lighting_render_data.combined_frames_image.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        Transform::default(),
        COMBINED_FRAMES_LAYER.clone(),
    ));

    commands.spawn((
        Camera2d,
        Camera {
            target: lighting_render_data.frames_image.clone().into(),
            clear_color: Color::NONE.into(),
            ..default()
        },
        Transform::default(),
        FRAMES_LAYER.clone(),
    ));

    if SHOW_DEBUG_FRAMES_SPRITE {
        commands.spawn((
            Sprite {
                image: lighting_render_data.frames_image.clone(),
                custom_size: Some(Vec2::new(320.0 / 4.0, 180.0 / 4.0)),
                ..default()
            },
            Transform::default(),
            DebugFramesSprite,
            MatchPlayerZ { offset: 2.0 },
        ));
    }

    commands.spawn((
        Mesh2d(lighting_render_data.background_mesh.clone()),
        MeshMaterial2d(lighting_render_data.background_material.clone()),
        Transform::default(),
        BACKGROUND_LAYER.clone(),
        BackgroundMarker,
    ));

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
}

fn update_debug_frames_sprite(
    q_camera: Query<&Transform, With<MainCamera>>,
    mut q_light_layer: Query<&mut Transform, (With<DebugFramesSprite>, Without<MainCamera>)>,
) {
    let Ok(camera_pos) = q_camera.get_single() else {
        return;
    };
    let Ok(mut light_layer_pos) = q_light_layer.get_single_mut() else {
        return;
    };

    light_layer_pos.translation = camera_pos.translation.with_z(light_layer_pos.translation.z);
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
