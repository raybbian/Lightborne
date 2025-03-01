#import bevy_render::view::View
#import bevy_render::globals::Globals
#import "shaders/lighting/functions.wgsl" as light_functions
#import "shaders/lighting/point_light.wgsl"::PointLight2d

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

struct Occluder2d {
    world_from_local: mat3x4<f32>,
    local_from_world_transpose_a: mat2x4<f32>,
    local_from_world_transpose_b: f32,
    half_size: vec2<f32>,
}

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var<uniform> globals: Globals;
@group(2) @binding(0) var<uniform> light: PointLight2d;
@group(3) @binding(0) var<uniform> occluder: Occluder2d;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let light_world_from_local = light_functions::get_world_from_local(light.world_from_local);
    let light_center_world_pos = light_functions::position_local_to_world(
        light_world_from_local,
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
    let light_radius = vec4<f32>(light.radius, light.radius, 0.0, 0.0);
    let light_max_world_pos = light_center_world_pos + light_radius;
    let light_min_world_pos = light_center_world_pos - light_radius;

    let world_from_local = light_functions::get_world_from_local(occluder.world_from_local);
    let new_position = vertex.position * vec3<f32>(occluder.half_size, 1.0);
    var world_position = light_functions::position_local_to_world(
        world_from_local,
        vec4<f32>(new_position, 1.0)
    );

#ifndef OCCLUDER_CUTOUT
    let point_to_light = normalize(light_center_world_pos.xy - world_position.xy);
    let dot_product = dot(point_to_light, vertex.normal.xy);

    if dot_product < 0.0 {
        world_position += vec4<f32>(-400.0 * point_to_light, 0.0, 0.0);
    }
#endif

    var output: VertexOutput;
    output.position = light_functions::position_world_to_clip(world_position, view);
    return output;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef OCCLUDER_CUTOUT
    return vec4<f32>(0.0, 0.0, 0.0, 0.5);
#else
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
#endif
}
