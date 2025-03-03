#import bevy_render::view::View
#import bevy_render::globals::Globals
#import "shaders/lighting/functions.wgsl" as light_functions
#import "shaders/lighting/line_light.wgsl"::LineLight2d

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
@group(2) @binding(0) var<uniform> light: LineLight2d;
@group(3) @binding(0) var<uniform> occluder: Occluder2d;

// Returns the point on the infinite line (through a and b) that is closest to p
fn closest_point_on_line(a: vec2<f32>, b: vec2<f32>, p: vec2<f32>) -> vec2<f32> {
    let d = vec2<f32>(b.x - a.x, b.y - a.y);
    var t = dot(p - a, d) / dot(d, d);
    t = clamp(t, 0.0, 1.0);
    return a + t * d;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let light_world_from_local = light_functions::get_world_from_local(light.world_from_local);
    let light_center = light_functions::position_local_to_world(
        light_world_from_local,
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
    let light_a = light_functions::position_local_to_world(
        light_world_from_local,
        vec4<f32>(-light.half_length, 0.0, 0.0, 1.0),
    );
    let light_b = light_functions::position_local_to_world(
        light_world_from_local,
        vec4<f32>(light.half_length, 0.0, 0.0, 1.0),
    );

    let world_from_local = light_functions::get_world_from_local(occluder.world_from_local);
    let new_position = vertex.position * vec3<f32>(occluder.half_size, 1.0);
    var world_position = light_functions::position_local_to_world(
        world_from_local,
        vec4<f32>(new_position, 1.0)
    );

#ifndef OCCLUDER_CUTOUT
    let closest_point = closest_point_on_line(light_a.xy, light_b.xy, world_position.xy);
    if distance(closest_point, world_position.xy) < light.radius {
        let point_to_light = normalize(closest_point - world_position.xy);
        let dot_product = dot(point_to_light, vertex.normal.xy);
        if dot_product < 0.0 {
            world_position += vec4<f32>(-400.0 * point_to_light, 0.0, 0.0);
        }
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
