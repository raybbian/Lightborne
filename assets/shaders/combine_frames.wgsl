#import bevy_render::globals::Globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import "shaders/utils.wgsl"::world_uv_to_lighting

@group(2) @binding(0) var occluder_mask: texture_2d<f32>;
@group(2) @binding(1) var occluder_sampler: sampler;
@group(2) @binding(2) var intensity_mask: texture_2d<f32>;
@group(2) @binding(3) var intensity_sampler: sampler;
@group(2) @binding(4) var foreground_mask: texture_2d<f32>;
@group(2) @binding(5) var foreground_sampler: sampler;
@group(2) @binding(6) var<uniform> light_colors: array<vec4<f32>, 16>; // RGB color of the light
@group(2) @binding(7) var<uniform> frame_count: vec2<u32>; // RGB color of the light

fn color_at_uv(uv: vec2<f32>) -> vec4<f32> {
    var final_opacity = 0.6;

    for (var i: u32 = 0; i < frame_count.x; i++) {
        for (var j: u32 = 0; j < frame_count.y; j++) {
            let mesh_uv: vec2<f32> = uv;
            let frame_uv = world_uv_to_lighting(i, j, mesh_uv, frame_count);

            // if on foreground + occluder, then render at lower intensity
            // if on only occluder, do not render
            // if on nothing, then render at standard intensity

            let occluder_val = textureSample(occluder_mask, occluder_sampler, frame_uv).a;
            let intensity_val = textureSample(intensity_mask, intensity_sampler, frame_uv).x;
            let foregound_val = textureSample(foreground_mask, foreground_sampler, mesh_uv).a;

            var light_intensity: f32 = 0.0;
            if foregound_val > 0.01 && occluder_val > 0.01 {
                light_intensity = 1.0 - pow(1.0 - max(0.0, intensity_val - 0.5), 9.0);
            } else if occluder_val > 0.01 {
                light_intensity = 0.0;
            } else {
                light_intensity = 1.0 - pow(1.0 - intensity_val, 6.0);
            }

            final_opacity = min(final_opacity, 1 - light_intensity);
        }
    }

    return vec4<f32>(0.0, 0.0, 0.0, final_opacity);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let mesh_uv: vec2<f32> = mesh.uv;
    return color_at_uv(mesh_uv);
}
