
#import bevy_render::globals::Globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// @group(2) @binding(0) var texture_image: texture_2d<f32>;
// @group(2) @binding(1) var texture_sampler: sampler;
// @group(2) @binding(2) var<uniform> light_colors: array<vec4<f32>, 16>; // RGB color of the light


@group(2) @binding(0) var texture_image: texture_2d<f32>;
@group(2) @binding(1) var texture_sampler: sampler;
@group(2) @binding(0) var light_image: texture_2d<f32>;
@group(2) @binding(1) var light_sampler: sampler;



@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let offset = vec2<f32>(1.0 / 320.0, 1.0 / 180.0);  // Texture resolution
    let kernel: array<f32, 25> = array<f32, 25>(
        1.0, 4.0, 6.0, 4.0, 1.0,
        4.0, 16.0, 24.0, 16.0, 4.0,
        6.0, 24.0, 36.0, 24.0, 6.0,
        4.0, 16.0, 24.0, 16.0, 4.0,
        1.0, 4.0, 6.0, 4.0, 1.0
    );

    // Normalize the kernel
    let kernel_sum: f32 = 256.0;

    var blurred_color: vec4<f32> = vec4<f32>(0.0);
    var index: i32 = 0;

    // Apply Gaussian kernel (5x5 blur)
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let mesh_uv: vec2f = mesh.uv;
            var sample_coords = mesh_uv + vec2<f32>(f32(x), f32(y)) * offset;

            let color_sample = textureSample(texture_image, texture_sampler, sample_coords);

            blurred_color += color_sample * kernel[index];
            index += 1;
        }
    }

    let final_color = blurred_color / kernel_sum;
    // let current_brightness = dot(final_color.xyz, vec3<f32>(0.2126, 0.7152, 0.0722));
    // let target_brightness = 0.8;
    // let ret = final_color.xyz * (target_brightness / current_brightness);
    // var brightness = current_brightness * 3.1;
    return vec4<f32>(final_color.xyz, 1.0);
}