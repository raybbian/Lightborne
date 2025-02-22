#import bevy_render::globals::Globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import "shaders/utils.wgsl"::voro_noise

@group(0) @binding(1) var<uniform> globals: Globals;
@group(2) @binding(0) var<uniform> material_color: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let v = clamp(mesh.uv.y, 0.01, 0.99);
    let smoothness = 10.0;
    let thickness = 20.0;

    let opacity = pow(1 - (pow(v, smoothness) + pow(1.0 - v, smoothness)), thickness);

    let voronoi = voro_noise(vec2(globals.time, globals.time), 1.0, 1.0) * opacity;

    return vec4(material_color.xyz, voronoi);
}

