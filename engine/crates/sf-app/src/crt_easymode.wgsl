/*
    CRT Shader by EasyMode
    License: GPL

    A flat CRT shader ideally for 1080p or higher displays.

    Recommended Settings:

    Video
    - Aspect Ratio:  4:3
    - Integer Scale: Off

    Shader
    - Filter: Nearest
    - Scale:  Don't Care

    Example RGB Mask Parameter Settings:

    Aperture Grille (Default)
    - Dot Width:  1
    - Dot Height: 1
    - Stagger:    0

    Lottes' Shadow Mask
    - Dot Width:  2
    - Dot Height: 1
    - Stagger:    3
*/

// View → Filter ▸ Easymode (DESIGN §2.7): a WGSL port of libretro slang-shaders
// `crt/shaders/crt-easymode.slang` (master, 2026-10-20), its `#pragma parameter` defaults fixed
// as constants and its compiled-in choice kept (ENABLE_LANCZOS 1). The one departure is
// `port.wgsl`'s quarter-pixel nudge at 2×. Appended after `port.wgsl`.

const EASYMODE_BRIGHT_BOOST: f32 = 1.2;
const EASYMODE_DILATION: f32 = 1.0;
const EASYMODE_GAMMA_INPUT: f32 = 2.0;
const EASYMODE_GAMMA_OUTPUT: f32 = 1.8;
const EASYMODE_MASK_SIZE: f32 = 1.0;
const EASYMODE_MASK_STAGGER: f32 = 0.0;
const EASYMODE_MASK_STRENGTH: f32 = 0.3;
const EASYMODE_MASK_DOT_HEIGHT: f32 = 1.0;
const EASYMODE_MASK_DOT_WIDTH: f32 = 1.0;
const EASYMODE_SCANLINE_CUTOFF: f32 = 400.0;
const EASYMODE_SCANLINE_BEAM_WIDTH_MAX: f32 = 1.5;
const EASYMODE_SCANLINE_BEAM_WIDTH_MIN: f32 = 1.5;
const EASYMODE_SCANLINE_BRIGHT_MAX: f32 = 0.65;
const EASYMODE_SCANLINE_BRIGHT_MIN: f32 = 0.35;
const EASYMODE_SCANLINE_STRENGTH: f32 = 1.0;
const EASYMODE_SHARPNESS_H: f32 = 0.5;
const EASYMODE_SHARPNESS_V: f32 = 1.0;
const EASYMODE_PI: f32 = 3.141592653589;

fn easymode_dilate(col: vec4<f32>) -> vec4<f32> {
    let x = mix(vec4<f32>(1.0), col, EASYMODE_DILATION);
    return col * x;
}

// `TEX2D`.
fn easymode_tex2d(c: vec2<f32>) -> vec4<f32> {
    return easymode_dilate(port_texel(c));
}

fn easymode_curve_distance(x: f32, sharp: f32) -> f32 {
    /*
        apply half-circle s-curve to distance for sharper (more pixelated) interpolation
        single line formula for Graph Toy:
        0.5 - sqrt(0.25 - (x - step(0.5, x)) * (x - step(0.5, x))) * sign(0.5 - x)
    */
    let x_step = step(0.5, x);
    let curve = 0.5 - sqrt(0.25 - (x - x_step) * (x - x_step)) * sign(0.5 - x);
    return mix(x, curve, sharp);
}

fn easymode_color_matrix(co: vec2<f32>, dx: vec2<f32>) -> mat4x4<f32> {
    return mat4x4<f32>(easymode_tex2d(co - dx), easymode_tex2d(co), easymode_tex2d(co + dx), easymode_tex2d(co + 2.0 * dx));
}

fn easymode_filter_lanczos(coeffs: vec4<f32>, color_matrix: mat4x4<f32>) -> vec3<f32> {
    let col = color_matrix * coeffs;
    let sample_min = min(color_matrix[1], color_matrix[2]);
    let sample_max = max(color_matrix[1], color_matrix[2]);
    return clamp(col, sample_min, sample_max).rgb;
}

@fragment
fn fs_easymode(o: Out) -> @location(0) vec4<f32> {
    let p = port(o);
    let size_zw = 1.0 / p.source;
    let dx = vec2<f32>(size_zw.x, 0.0);
    let dy = vec2<f32>(0.0, size_zw.y);
    let pix_co = p.tc * p.source - vec2<f32>(0.5, 0.5);
    let tex_co = (floor(pix_co) + vec2<f32>(0.5, 0.5)) * size_zw;
    let dist = fract(pix_co);

    // ENABLE_LANCZOS.
    let curve_x = easymode_curve_distance(dist.x, EASYMODE_SHARPNESS_H * EASYMODE_SHARPNESS_H);
    var coeffs = EASYMODE_PI * vec4<f32>(1.0 + curve_x, curve_x, 1.0 - curve_x, 2.0 - curve_x);
    coeffs = max(abs(coeffs), vec4<f32>(1e-5));
    coeffs = 2.0 * sin(coeffs) * sin(coeffs * 0.5) / (coeffs * coeffs);
    coeffs /= dot(coeffs, vec4<f32>(1.0));
    var col = easymode_filter_lanczos(coeffs, easymode_color_matrix(tex_co, dx));
    var col2 = easymode_filter_lanczos(coeffs, easymode_color_matrix(tex_co + dy, dx));

    col = mix(col, col2, easymode_curve_distance(dist.y, EASYMODE_SHARPNESS_V));
    col = pow(col, vec3<f32>(EASYMODE_GAMMA_INPUT / (EASYMODE_DILATION + 1.0)));

    let luma = dot(vec3<f32>(0.2126, 0.7152, 0.0722), col);
    let bright = (max(col.r, max(col.g, col.b)) + luma) * 0.5;
    let scan_bright = clamp(bright, EASYMODE_SCANLINE_BRIGHT_MIN, EASYMODE_SCANLINE_BRIGHT_MAX);
    let scan_beam = clamp(bright * EASYMODE_SCANLINE_BEAM_WIDTH_MAX, EASYMODE_SCANLINE_BEAM_WIDTH_MIN, EASYMODE_SCANLINE_BEAM_WIDTH_MAX);
    // `cos(vTexCoord.y * 2.0 * PI * SourceSize.y)`, its argument first reduced to one period:
    // Metal's `cos` of the unreduced argument (up to 2π × 261) is off enough to band the 1×
    // picture, where every row should sit at the cosine's flat minimum. And the base kept off
    // the rounding just below 0 there, where `pow` is undefined (black on Metal).
    let scan_base = max(cos(fract(p.tc.y * p.source.y) * 2.0 * EASYMODE_PI) * 0.5 + 0.5, 0.0);
    var scan_weight = 1.0 - pow(scan_base, scan_beam) * EASYMODE_SCANLINE_STRENGTH;

    let mask = 1.0 - EASYMODE_MASK_STRENGTH;
    let mod_fac = floor(p.uv * p.output * p.source / (p.source * vec2<f32>(EASYMODE_MASK_SIZE, EASYMODE_MASK_DOT_HEIGHT * EASYMODE_MASK_SIZE)));
    let dot_no = i32(port_mod((mod_fac.x + port_mod(mod_fac.y, 2.0) * EASYMODE_MASK_STAGGER) / EASYMODE_MASK_DOT_WIDTH, 3.0));
    var mask_weight: vec3<f32>;
    if dot_no == 0 {
        mask_weight = vec3<f32>(1.0, mask, mask);
    } else if dot_no == 1 {
        mask_weight = vec3<f32>(mask, 1.0, mask);
    } else {
        mask_weight = vec3<f32>(mask, mask, 1.0);
    }

    if p.source.y >= EASYMODE_SCANLINE_CUTOFF {
        scan_weight = 1.0;
    }

    col2 = col;
    col *= vec3<f32>(scan_weight);
    col = mix(col, col2, scan_bright);
    col *= mask_weight;
    col = pow(col, vec3<f32>(1.0 / EASYMODE_GAMMA_OUTPUT));

    return vec4<f32>(col * EASYMODE_BRIGHT_BOOST, 1.0);
}
