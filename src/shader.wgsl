struct Uniforms {
    center_x: f32,
    center_y: f32,
    scale: f32,
    width: f32,
    height: f32,
    max_iter: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    var out: VsOut;
    out.pos = vec4<f32>(positions[idx], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let px = in.pos.x;
    let py = in.pos.y;

    let aspect = u.width / u.height;
    let x_span = u.scale;
    let y_span = x_span / aspect;

    let left = u.center_x - x_span * 0.5;
    let top  = u.center_y - y_span * 0.5;

    let a = left + (px / u.width) * x_span;
    let b = top  + (py / u.height) * y_span;

    var z_a: f32 = 0.0;
    var z_b: f32 = 0.0;
    var iter: u32 = u.max_iter;

    let x_minus_quarter = a - 0.25;
    let q = x_minus_quarter * x_minus_quarter + b * b;
    let in_cardioid = q * (q + x_minus_quarter) <= 0.25 * b * b;
    let in_period_two_bulb = (a + 1.0) * (a + 1.0) + b * b <= 0.0625;

    if !in_cardioid && !in_period_two_bulb {
        for (var i: u32 = 0u; i < u.max_iter; i = i + 1u) {
            if (z_a * z_a + z_b * z_b) > 4.0 {
                iter = i;
                break;
            }
            let new_z_a = (z_a * z_a) - (z_b * z_b) + a;
            let new_z_b = 2.0 * z_a * z_b + b;
            z_a = new_z_a;
            z_b = new_z_b;
        }
    }

    if iter == u.max_iter {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let c_u8: u32 = (iter * 5u) & 255u;
    let r = f32((c_u8 * 2u) & 255u) / 255.0;
    let g = f32((c_u8 * 5u) & 255u) / 255.0;
    let bl = 1.0;

    return vec4<f32>(r, g, bl, 1.0);
}