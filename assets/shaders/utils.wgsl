//  <https://www.shadertoy.com/view/Xd23Dh>
//  by Inigo Quilez
//
fn hash3(p: vec2f) -> vec3f {
    let q = vec3f(dot(p, vec2f(127.1, 311.7)),
        dot(p, vec2f(269.5, 183.3)),
        dot(p, vec2f(419.2, 371.9)));
    return fract(sin(q) * 43758.5453);
}

fn lighting_uv_to_world(frame_x: u32, frame_y: u32, uv: vec2<f32>, frame_count: vec2<u32>) -> vec2<f32> {
    let super_uv = uv * vec2<f32>(frame_count);
    return super_uv - vec2<f32>(f32(frame_x), f32(frame_y));
}

fn world_uv_to_lighting(frame_x: u32, frame_y: u32, uv: vec2<f32>, frame_count: vec2<u32>) -> vec2<f32> {
    let frame_uv_min = vec2<f32>(
        f32(frame_x) / f32(frame_count.x),
        f32(frame_y) / f32(frame_count.y),
    );
    let frame_uv_max = frame_uv_min + 1.0 / vec2<f32>(frame_count);
    return mix(frame_uv_min, frame_uv_max, uv);
}

fn voro_noise(x: vec2f, u: f32, v: f32) -> f32 {
    let p = floor(x);
    let f = fract(x);
    let k = 1. + 63. * pow(1. - v, 4.);
    var va: f32 = 0.;
    var wt: f32 = 0.;
    for (var j: i32 = -2; j <= 2; j = j + 1) {
        for (var i: i32 = -2; i <= 2; i = i + 1) {
            let g = vec2f(f32(i), f32(j));
            let o = hash3(p + g) * vec3f(u, u, 1.);
            let r = g - f + o.xy;
            let d = dot(r, r);
            let ww = pow(1. - smoothstep(0., 1.414, sqrt(d)), k);
            va = va + o.z * ww;
            wt = wt + ww;
        }
    }
    return va / wt;
}
