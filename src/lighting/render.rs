use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

use super::material::{
    BlurMaterial, CombineFramesMaterial, FrameMaskMaterial, GradientLightMaterial,
};

#[derive(Resource)]
pub struct LightingRenderData {
    pub gradient_mesh: Handle<Mesh>,
    pub gradient_material: Handle<GradientLightMaterial>,

    pub combine_frames_mesh: Handle<Mesh>,
    pub combine_frames_material: Handle<CombineFramesMaterial>,

    pub blur_mesh: Handle<Mesh>,
    pub blur_material: Handle<BlurMaterial>,

    pub combined_frames_image: Handle<Image>,
    pub intensity_mask: Handle<Image>,
    pub blurred_image: Handle<Image>,

    pub foreground_mask: Handle<Image>,
    pub background_mask: Handle<Image>,
    pub occluder_mask: Handle<Image>,

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
    return image;
}

impl FromWorld for LightingRenderData {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let frame_count = UVec2::new(4, 4);

        let gradient_mesh = meshes
            .add(Mesh::from(Rectangle::new(
                320. * frame_count.x as f32,
                180. * frame_count.y as f32,
            )))
            .into();
        let combine_frames_mesh = meshes.add(Mesh::from(Rectangle::new(320., 180.))).into();
        let blur_mesh = meshes.add(Mesh::from(Rectangle::new(320., 180.))).into();
        let default_occluder_mesh = meshes.add(Mesh::from(Rectangle::new(1., 1.))).into();

        let mut images = world.resource_mut::<Assets<Image>>();

        let combined_frames_image = images.add(new_render_image(320, 180));
        let blurred_image = images.add(new_render_image(320, 180));
        let foreground_mask = images.add(new_render_image(320, 180));
        let background_mask = images.add(new_render_image(320, 180));
        let occluder_mask = images.add(new_render_image(320 * frame_count.x, 180 * frame_count.y));
        let intensity_mask = images.add(new_render_image(320 * frame_count.x, 180 * frame_count.y));

        let mut materials = world.resource_mut::<Assets<GradientLightMaterial>>();

        let gradient_material = materials.add(GradientLightMaterial {
            light_points: [Vec4::splat(10000000.0); 16], // Position (normalized to screen space)
            light_radiuses: [Vec4::splat(0.0); 16],      // Light radius in pixels
            mesh_transform: Vec4::new(320., 180., 0., 0.),
            frame_count,
        });

        let mut materials = world.resource_mut::<Assets<CombineFramesMaterial>>();

        let combine_frames_material = materials.add(CombineFramesMaterial {
            occluder_mask: occluder_mask.clone(),
            intensity_mask: intensity_mask.clone(),
            foreground_mask: foreground_mask.clone(),
            frame_count,
            light_colors: [Vec4::new(0., 1.0, 0., 1.0); 16],
        });

        let mut materials = world.resource_mut::<Assets<FrameMaskMaterial>>();
        let frame_mask_materials = (0..16)
            .into_iter()
            .map(|i| {
                materials.add(FrameMaskMaterial {
                    frame_count_x: 4,
                    frame_count_y: 4,
                    frame_index: i,
                    color: Vec4::new(1. - (i as f32 / 16.0), 0.0, i as f32 / 16.0, 1.0),
                })
            })
            .collect::<Vec<_>>();

        let mut materials = world.resource_mut::<Assets<BlurMaterial>>();
        let blur_material = materials.add(BlurMaterial {
            image: combined_frames_image.clone(),
        });

        LightingRenderData {
            gradient_material,
            combine_frames_material,
            gradient_mesh,
            combine_frames_mesh,
            combined_frames_image,
            foreground_mask,
            background_mask,
            occluder_mask,
            intensity_mask,
            frame_mask_materials: frame_mask_materials.try_into().unwrap(),
            blur_mesh,
            blur_material,
            blurred_image,
            default_occluder_mesh,
        }
    }
}
