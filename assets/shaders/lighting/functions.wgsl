#import bevy_render::view::View
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}

fn get_world_from_local(world_from_local: mat3x4<f32>) -> mat4x4<f32> {
    return affine3_to_square(world_from_local);
}

fn position_local_to_world(world_from_local: mat4x4<f32>, vertex_position: vec4<f32>) -> vec4<f32> {
    return world_from_local * vertex_position;
}

fn position_world_to_clip(world_position: vec4<f32>, view: View) -> vec4<f32> {
    return view.clip_from_world * world_position;
}

fn normal_local_to_world(
    vertex_normal: vec3<f32>, 
    local_from_world_transpose_a: mat2x4<f32>,
    local_from_world_transpose_b: f32
) -> vec3<f32> {
    return mat2x4_f32_to_mat3x3_unpack(
        local_from_world_transpose_a,
        local_from_world_transpose_b,
    ) * vertex_normal;
}
