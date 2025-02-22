
#import bevy_render::globals::Globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(0) @binding(1) var<uniform> globals: Globals;

@group(2) @binding(0) var<uniform> frame_count_x: i32;
@group(2) @binding(1) var<uniform> frame_count_y: i32;
@group(2) @binding(2) var<uniform> frame_index: i32;
@group(2) @binding(3) var<uniform> color: vec4<f32>;




@fragment
fn fragment(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // if mesh.uv.x < 0.0 || mesh.uv.y < 0.0 || mesh.uv.x > 1.0 || mesh.uv.y > 1.0 {
    //     return vec4(0.0);
    // }

    let frame_x = frame_index % frame_count_x;
    let frame_y = frame_index / frame_count_y;

    let min_x = 320.0 * f32(frame_x);
    let min_y = 180.0 * f32(frame_y);

    let t: f32 = globals.time;



    let screen_pos = vec3<f32>(pos.x / pos.w, pos.y / pos.w, pos.z / pos.w);



    if screen_pos.x > min_x && screen_pos.x < min_x + 320.0 && screen_pos.y > min_y && screen_pos.y < min_y + 180.0 {
        return color;
    }
    return vec4<f32>(0.);
}
