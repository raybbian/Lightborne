#import bevy_render::globals::Globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var texture_image: texture_2d<f32>;
@group(2) @binding(1) var texture_sampler: sampler;
@group(2) @binding(2) var<uniform> light_colors: array<vec4<f32>, 16>; // RGB color of the light

const frame_size: vec2<f32> = vec2<f32>(320.0, 180.0);
const texture_size: vec2<f32> = vec2<f32>(1280.0, 720.0);
const frame_count: vec2<i32> = vec2<i32>(4, 4);

fn calculate_uv(frame_index: i32, uv: vec2<f32>) -> vec2<f32> {
    let frame_uv_min = vec2<f32>(
        f32(frame_index % frame_count.x) * frame_size.x / texture_size.x,
        f32(frame_index / frame_count.x) * frame_size.y / texture_size.y
    );

    let frame_uv_max = frame_uv_min + frame_size / texture_size;

    // Map UV to the current frame's region
    return mix(frame_uv_min, frame_uv_max, uv);
}

fn color_at_uv(uv: vec2<f32>) -> vec4<f32> {
    var final_color = vec4<f32>(0.0); // Initialize as transparent black

    for (var i = 0; i < frame_count.x * frame_count.y; i++) {
        let mesh_uv: vec2<f32> = uv;
        let frame_uv = calculate_uv(i, mesh_uv);
        let frame_color = textureSample(texture_image, texture_sampler, frame_uv);

        // Additive blending (adjust as needed)
        final_color += vec4f(frame_color.y * light_colors[i].xyz * 0.1, 1.0);
    }

    return final_color;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let mesh_uv: vec2<f32> = mesh.uv;
    return color_at_uv(mesh_uv);
}
