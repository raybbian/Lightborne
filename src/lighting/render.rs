use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

use super::material::{
    BackgroundMaterial, BlurMaterial, CombineFramesMaterial, FrameMaskMaterial, FrameMaskUniforms,
    GradientLightMaterial,
};

#[derive(Resource)]
pub struct LightingRenderData {
    pub gradient_mesh: Handle<Mesh>,
    pub gradient_material: Handle<GradientLightMaterial>,

    pub combine_frames_mesh: Handle<Mesh>,
    pub combine_frames_material: Handle<CombineFramesMaterial>,

    pub blur_mesh: Handle<Mesh>,
    pub blur_material: Handle<BlurMaterial>,

    pub background_mesh: Handle<Mesh>,
    pub background_material: Handle<BackgroundMaterial>,

    pub combined_frames_image: Handle<Image>,
    pub frames_image: Handle<Image>,
    pub blurred_image: Handle<Image>,

    pub default_occluder_mesh: Handle<Mesh>,

    pub frame_mask_materials: [Handle<FrameMaskMaterial>; 16],
}

fn new_render_image(width: u32, height: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            ..default()
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::nearest();
    image
}

impl FromWorld for LightingRenderData {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();

        let gradient_mesh = meshes.add(Mesh::from(Rectangle::new(320. * 4., 180. * 4.)));
        let combine_frames_mesh = meshes.add(Mesh::from(Rectangle::new(320., 180.)));
        let blur_mesh = meshes.add(Mesh::from(Rectangle::new(320., 180.)));
        let background_mesh = meshes.add(Mesh::from(Rectangle::new(320., 180.)));
        let default_occluder_mesh = meshes.add(Mesh::from(Rectangle::new(1., 1.)));

        let mut images = world.resource_mut::<Assets<Image>>();

        let combined_frames_image = images.add(new_render_image(320, 180));
        let blurred_image = images.add(new_render_image(320, 180));
        let frames_image = images.add(new_render_image(320 * 4, 180 * 4));

        let mut materials = world.resource_mut::<Assets<GradientLightMaterial>>();

        let gradient_material = materials.add(GradientLightMaterial {
            light_points: [Vec4::splat(10000000.0); 16], // Position (normalized to screen space)
            light_radiuses: [Vec4::splat(0.0); 16],      // Light radius in pixels
            mesh_transform: Vec4::new(320., 180., 0., 0.),
        });

        let mut materials = world.resource_mut::<Assets<CombineFramesMaterial>>();

        let combine_frames_material = materials.add(CombineFramesMaterial {
            image: frames_image.clone(),
            light_colors: [Vec4::new(0., 1.0, 0., 1.0); 16],
        });

        let mut materials = world.resource_mut::<Assets<FrameMaskMaterial>>();
        let frame_mask_materials = (0..16)
            .map(|i| {
                materials.add(FrameMaskMaterial {
                    frame_info: FrameMaskUniforms {
                        frame_count_x: 4,
                        frame_count_y: 4,
                        frame_index: i,
                        _wasm_padding: 0.0,
                    },
                    color: Vec4::new(1. - (i as f32 / 16.0), 0.0, i as f32 / 16.0, 1.0),
                })
            })
            .collect::<Vec<_>>();

        let mut materials = world.resource_mut::<Assets<BlurMaterial>>();
        let blur_material = materials.add(BlurMaterial {
            image: combined_frames_image.clone(),
            _wasm_padding: Vec2::ZERO,
        });

        let background_image: Handle<Image> = world
            .resource::<AssetServer>()
            .load("levels/background.png");

        let mut materials = world.resource_mut::<Assets<BackgroundMaterial>>();
        let background_material = materials.add(BackgroundMaterial {
            light_image: blurred_image.clone(),
            background_image,
        });

        LightingRenderData {
            gradient_material,
            combine_frames_material,
            gradient_mesh,
            combine_frames_mesh,
            combined_frames_image,
            frames_image,
            frame_mask_materials: frame_mask_materials.try_into().unwrap(),
            blur_mesh,
            blur_material,
            blurred_image,
            background_material,
            background_mesh,
            default_occluder_mesh,
        }
    }
}
