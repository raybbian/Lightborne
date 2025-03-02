#import bevy_render::globals::Globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var light_image: texture_2d<f32>;
@group(2) @binding(1) var light_sampler: sampler;
@group(2) @binding(2) var background_image: texture_2d<f32>;
@group(2) @binding(3) var background_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let mesh_uv: vec2<f32> = mesh.uv;

    let background_color = textureSample(background_image, background_sampler, mesh_uv).xyz;
    let light_color = textureSample(light_image, light_sampler, mesh_uv).xyz;

    let ambient_light = vec3(1., 1., 1.);
    let light = light_color * 25.0 + ambient_light;
    let blended = background_color * light + light_color * 0.2;
    return vec4<f32>(blended, 1.0);
}
