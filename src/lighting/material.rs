use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::sprite::{AlphaMode2d, Material2d};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GradientLightMaterial {
    #[uniform(0)]
    pub light_points: [Vec4; 16],
    #[uniform(1)]
    pub light_radiuses: [Vec4; 16],
    #[uniform(2)]
    pub mesh_transform: Vec4,
}

impl Material2d for GradientLightMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/gradient_circle.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CombineFramesMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub image: Handle<Image>,
    #[uniform(2)]
    pub light_colors: [Vec4; 16],
}

impl Material2d for CombineFramesMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/combine_frames.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FrameMaskMaterial {
    #[uniform(0)]
    pub frame_count_x: i32,
    #[uniform(1)]
    pub frame_count_y: i32,
    #[uniform(2)]
    pub frame_index: i32,
    #[uniform(3)]
    pub color: Vec4,
}

impl Material2d for FrameMaskMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/frame_mask.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BlurMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub image: Handle<Image>,
    // WASM Build requires that this struct be 16-byte aligned
    pub webgl_padding: Vec2,
}

impl Material2d for BlurMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/blur.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BackgroundMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub light_image: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub background_image: Handle<Image>,
}

impl Material2d for BackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/background.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// WASM build requires all of these structs be 16-byte aligned
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn gradient_light_material_alignment() {
        assert_eq!(mem::size_of::<GradientLightMaterial>() % 16, 0);
    }

    #[test]
    fn combine_frames_material_alignment() {
        assert_eq!(mem::size_of::<CombineFramesMaterial>() % 16, 0);
    }

    #[test]
    fn frame_mask_material_alignment() {
        assert_eq!(mem::size_of::<FrameMaskMaterial>() % 16, 0);
    }

    #[test]
    fn blur_material_alignment() {
        assert_eq!(mem::size_of::<BlurMaterial>() % 16, 0);
    }

    #[test]
    fn background_material_alignment() {
        assert_eq!(mem::size_of::<BackgroundMaterial>() % 16, 0);
    }
}
