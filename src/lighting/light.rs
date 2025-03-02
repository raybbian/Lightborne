use bevy::prelude::*;

use crate::camera::MainCamera;

use super::{
    combine_lights,
    material::{CombineFramesMaterial, GradientLightMaterial},
    CombinedLighting, LightingRenderData,
};

/// [`Component`] that represents a light spread evenly around a line segment, used primarily for the laser lights.
/// The two points of the line segment are calculated based on the entity's [`Transform`] component, using the rotation and x scale:
/// ```no_run
/// let unit_vec = transform
///     .rotation
///     .mul_vec3(Vec3::new(1.0, 0.0, 0.0))
///     .truncate();
/// let point_1 = transform.translation.truncate() + unit_vec * transform.scale.x / 2.;
/// let point_2 = transform.translation.truncate() - unit_vec * transform.scale.x / 2.;
/// ```
/// Note that there currently can only be a total of 16 entities with [`LineLighting`] or [`PointLighting`].
/// Any lights above that limit won't be rendered.
#[derive(Component, Debug, Default, Clone)]
#[require(Transform, Visibility)]
pub struct LineLighting {
    pub radius: f32,
    /// RGB representation of the color, the higher each number the brighter the light
    pub color: Vec3,
}

/// [`Component`] that represents a light spread evenly around a point. Named "PointLighting" to disambiguate from Bevy's [`PointLight`].
/// Note that there currently can only be a total of 16 entities with [`LineLighting`] or [`PointLighting`].
/// Any lights above that limit won't be rendered.
#[derive(Component, Debug, Default, Clone)]
#[require(Transform, Visibility)]
pub struct PointLighting {
    pub radius: f32,
    /// RGB representation of the color, the higher each number the brighter the light
    pub color: Vec3,
}

pub fn draw_lights(
    q_line_lights: Query<(&GlobalTransform, &Visibility, &LineLighting)>,
    q_point_lights: Query<(&GlobalTransform, &Visibility, &PointLighting)>,

    render: Res<LightingRenderData>,

    mut res_gradient_material: ResMut<Assets<GradientLightMaterial>>,
    mut res_combine_frames_material: ResMut<Assets<CombineFramesMaterial>>,

    q_camera: Query<&Transform, With<MainCamera>>,
) {
    let Ok(camera_t) = q_camera.get_single() else {
        return;
    };
    let camera_translation = camera_t.translation.truncate();

    let Some(gradient_material) = res_gradient_material.get_mut(&render.gradient_material) else {
        return;
    };
    let Some(combine_frames_material) =
        res_combine_frames_material.get_mut(&render.combine_frames_material)
    else {
        return;
    };
    let lights = combine_lights(q_line_lights, q_point_lights, 16);

    let mut light_points = [Vec4::splat(99999999999.); 16];
    let mut light_radiuses = [Vec4::splat(0.0); 16];
    let mut light_colors = [Vec4::splat(0.0); 16];
    for (
        i,
        CombinedLighting {
            pos_1,
            pos_2,
            radius,
            color,
            ..
        },
    ) in lights.iter().enumerate()
    {
        light_points[i] = Vec4::new(pos_1.x, pos_1.y, pos_2.x, pos_2.y);
        light_radiuses[i].x = *radius;
        light_colors[i] = color.extend(1.0);
    }
    gradient_material.light_points = light_points;
    gradient_material.light_radiuses = light_radiuses;
    gradient_material.mesh_transform.z = camera_translation.x;
    gradient_material.mesh_transform.w = camera_translation.y;
    combine_frames_material.light_colors = light_colors;
}
