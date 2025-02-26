#import bevy_render::globals::Globals
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(0) @binding(1) var<uniform> globals: Globals;

@group(2) @binding(0) var<uniform> light_points: array<vec4<f32>, 16>; // Center of the light (in normalized screen space)
@group(2) @binding(1) var<uniform> light_radiuses: array<vec4<f32>, 16>; // Radius of the light gradient
@group(2) @binding(2) var<uniform> mesh_transform: vec4<f32>; // Radius of the light gradient

fn vector_to_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    // Vector from A to P
    let ap = p - a;

    // Vector from A to B
    let ab = b - a;

    // Squared length of AB
    let ab_length_squared = dot(ab, ab);

    // Handle degenerate case: A and B are the same point
    if ab_length_squared == 0.0 {
        return -ap; // The vector directly towards A (or B, since they're the same)
    }

    // Projection of P onto the infinite line defined by A and B
    let t = clamp(dot(ap, ab) / ab_length_squared, 0.0, 1.0); // Clamp t to [0, 1]

    // Closest point on the segment
    let closest_point = a + t * ab;

    // Return vector towards the segment
    return closest_point - p;
}


@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let mesh_x: f32 = mesh.uv.x;
    let mesh_y: f32 = mesh.uv.y;

    var frame_i = 0;
    if mesh_x > 0.75 {
        frame_i = 3;
    } else if mesh_x > 0.5 {
        frame_i = 2;
    } else if mesh_x > 0.25 {
        frame_i = 1;
    }

    var frame_j = 0;
    if mesh_y > 0.75 {
        frame_j = 3;
    } else if mesh_y > 0.5 {
        frame_j = 2;
    } else if mesh_y > 0.25 {
        frame_j = 1;
    }

    let light_index = frame_i + frame_j * 4;
    let light = light_points[light_index];
    let light_radius = light_radiuses[light_index].x;

    if light_radius == 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let frame_x: f32 = (mesh.uv.x - f32(frame_i) * 0.25) * 4.0;
    let frame_y: f32 = (mesh.uv.y - f32(frame_j) * 0.25) * 4.0;

    let pos_x: f32 = mesh_transform.x * frame_x + mesh_transform.z - mesh_transform.x / 2.0;
    let pos_y: f32 = mesh_transform.y * (1.0 - frame_y) + mesh_transform.w - mesh_transform.y / 2.0;
    let pos = vec2<f32>(pos_x, pos_y);

    // Calculate distance from the fragment to the light center
    let pos_to_segment = vector_to_segment(pos, light.xy, light.zw);

    var intensity: f32 = 0.0;
    let dist = length(pos_to_segment);
    // Normalize the distance based on the light radius (0 at center, 1 at the edge)

    let t: f32 = globals.time;
    let radius: f32 = max(0.0, light_radius + sin(t * 2.0) * 5.0);
    let dist_normal = 1.0 - clamp(dist / radius, 0.0, 1.0);



    intensity = pow(dist_normal, 1.8);


    // if intensity < 0.1 {
    //     intensity = 0.1;
    // } else if intensity < 0.2 {
    //     intensity = 0.2;
    // } else if intensity < 0.3 {
    //     intensity = 0.3;
    // } else if intensity < 0.4 {
    //     intensity = 0.4;
    // } else if intensity < 0.5 {
    //     intensity = 0.5;
    // } else if intensity < 0.6 {
    //     intensity = 0.6;
    // } else if intensity < 0.7 {
    //     intensity = 0.7;
    // } else if intensity < 0.8 {
    //     intensity = 0.8;
    // } else if intensity < 0.9 {
    //     intensity = 0.9;
    // } else {
    //     intensity = 1.0;
    // }

    // Apply a smooth fade-off using intensity
    let light_color = vec3f(1.0);

    // Return the gradient color with transparency (optional)
    return vec4<f32>(light_color.xyz * intensity, 1.0); // Use intensity for alpha
}
