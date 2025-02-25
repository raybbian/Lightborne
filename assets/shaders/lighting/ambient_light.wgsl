#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct AmbientLight2d {
    color: vec4<f32>,
}

@group(0) @binding(0) var unlit_texture: texture_2d<f32>;
@group(0) @binding(1) var unlit_sampler: sampler;
@group(0) @binding(2) var<uniform> ambient_light: AmbientLight2d;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let intensity = ambient_light.color.a;
    let color = ambient_light.color.rgb;

    let base_color = textureSample(unlit_texture, unlit_sampler, in.uv).rgb;
    let shaded_color = base_color * color * intensity;

    return vec4<f32>(shaded_color, 1.0);
}



