use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{AlphaMode2d, Material2d},
};
use enum_map::{enum_map, EnumMap};

use super::{LightColor, LIGHT_SEGMENT_THICKNESS};

/// The path to the shader used by the [`LightMaterial`]
const LIGHT_SHADER_PATH: &str = "shaders/light.wgsl";

/// A resource that stored handles to the [`Mesh2d`] and [`MeshMaterial2d`] used in the rendering
/// of [`LightSegment`](super::segments::LightSegmentBundle)s.
#[derive(Resource)]
pub struct LightRenderData {
    pub mesh: Mesh2d,
    pub material_map: EnumMap<LightColor, MeshMaterial2d<LightMaterial>>,
}

impl FromWorld for LightRenderData {
    /// Initializes the [`LightRenderData`] [`Resource`] from the [`World`], before the app runs.
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes
            .add(Rectangle::new(1.0, LIGHT_SEGMENT_THICKNESS))
            .into();

        let mut materials = world.resource_mut::<Assets<LightMaterial>>();

        LightRenderData {
            mesh: mesh_handle,
            material_map: enum_map! {
                LightColor::Green => materials.add(LightMaterial::from(LightColor::Green)).into(),
                LightColor::Purple => materials.add(LightMaterial::from(LightColor::Purple)).into(),
                LightColor::White => materials.add(LightMaterial::from(LightColor::White)).into(),
                LightColor::Blue => materials.add(LightMaterial::from(LightColor::Blue)).into(),
                LightColor::Black => materials.add(LightMaterial::from(LightColor::Black)).into(),
            },
        }
    }
}

/// Custom [`Material2d`] that will use our custom WGSL shader to draw
/// [`LightSegment`](super::segments::LightSegmentBundle)s.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LightMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    pub alpha_mode: AlphaMode2d,
    // WebGL2 requires this struct be 16-byte aligned
    pub _wasm_padding: Vec2,
}

impl Material2d for LightMaterial {
    fn fragment_shader() -> ShaderRef {
        LIGHT_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }
}

// WebGL2 requires these structs be 16-byte aligned
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn light_material_alignment() {
        assert_eq!(mem::size_of::<LightMaterial>() % 16, 0);
    }
}
