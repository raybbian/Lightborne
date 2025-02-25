#import bevy_render::view::View
#import bevy_render::globals::Globals
#import "shaders/lighting/functions.wgsl" as light_functions

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct PointLight2d {
    world_from_local: mat3x4<f32>,
    local_from_world_transpose_a: mat2x4<f32>,
    local_from_world_transpose_b: f32,
    color: vec4<f32>,
    radius: f32,
    volumetric_intensity: f32,
}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;

@group(1) @binding(0) var<uniform> light: PointLight2d;

@group(2) @binding(0) var unlit_image: texture_2d<f32>;
@group(2) @binding(1) var unlit_sampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    out.uv = vertex.uv;

    let world_from_local = light_functions::get_world_from_local(light.world_from_local);
    let new_position = vertex.position * light.radius;
    out.world_position = light_functions::position_local_to_world(
        world_from_local,
        vec4<f32>(new_position, 1.0)
    );
    out.position = light_functions::position_world_to_clip(out.world_position, view);

    return out;
}

fn point_light_color(uv: vec2<f32>, screen_uv: vec2<f32>) -> vec4<f32> {
    let one_tex_uv = uv * 2.0 - vec2<f32>(1.0); // -1 to 1

    let distance = min(length(one_tex_uv), 1.0);
    let angle = abs(atan2(one_tex_uv.y, one_tex_uv.x));

    let radial_fall_off = pow(1.0 - distance, 2.0);
    let angular_fall_off = smoothstep(-3.14159, 3.14159, angle);
    let normal_fall_off = 1.0;
    let intensity = light.color.a;

    let final_intensity = intensity * radial_fall_off * angular_fall_off * normal_fall_off;
    let light_color = final_intensity * light.color.rgb;
    let base_color = textureSample(unlit_image, unlit_sampler, screen_uv).rgb;
    let shaded_color = base_color * light_color + light_color * light.volumetric_intensity;

    return vec4<f32>(shaded_color, 1.0);
}

@fragment
fn fragment(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    let screen_uv = in.position.xy / view.viewport.zw;
    return point_light_color(in.uv, screen_uv);
}
