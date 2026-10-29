// PUBLIC DOMAIN CRT STYLED SCAN-LINE SHADER
//
//   by Timothy Lottes
//
// This is more along the style of a really good CGA arcade monitor.
// With RGB inputs instead of NTSC.
// The shadow mask example has the mask rotated 90 degrees for less chromatic aberration.
//
// Left it unoptimized to show the theory behind the algorithm.
//
// It is an example what I personally would want as a display option for pixel art games.
// Please take and use, change, or whatever.

// View → Filter ▸ Lottes (DESIGN §2.7): a WGSL port of libretro slang-shaders
// `crt/shaders/crt-lottes.slang` (master, 2026-10-20), its `#pragma parameter` defaults fixed as
// constants and its compiled-in choices kept (DO_BLOOM on, SIMPLE_LINEAR_GAMMA off). The one
// departure is `port.wgsl`'s quarter-pixel nudge at 2×. Appended after `port.wgsl`.

const LOTTES_HARD_SCAN: f32 = -8.0;
const LOTTES_HARD_PIX: f32 = -3.0;
const LOTTES_WARP_X: f32 = 0.031;
const LOTTES_WARP_Y: f32 = 0.041;
const LOTTES_MASK_DARK: f32 = 0.5;
const LOTTES_MASK_LIGHT: f32 = 1.5;
// `scaleInLinearGamma` is 1: the linear conversions below are always on.
// `shadowMask` is 3: the stretched VGA style mask, the only one kept.
const LOTTES_BRIGHT_BOOST: f32 = 1.0;
const LOTTES_HARD_BLOOM_PIX: f32 = -1.5;
const LOTTES_HARD_BLOOM_SCAN: f32 = -2.0;
const LOTTES_BLOOM_AMOUNT: f32 = 0.15;
const LOTTES_SHAPE: f32 = 2.0;

fn lottes_to_linear1(c: f32) -> f32 {
    return select(pow((c + 0.055) / 1.055, 2.4), c / 12.92, c <= 0.04045);
}

fn lottes_to_linear(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(lottes_to_linear1(c.r), lottes_to_linear1(c.g), lottes_to_linear1(c.b));
}

fn lottes_to_srgb1(c: f32) -> f32 {
    return select(1.055 * pow(c, 0.41666) - 0.055, c * 12.92, c < 0.0031308);
}

fn lottes_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(lottes_to_srgb1(c.r), lottes_to_srgb1(c.g), lottes_to_srgb1(c.b));
}

// Nearest emulated sample given floating point position and texel offset.
// Also zero's off screen.
fn lottes_fetch(pos: vec2<f32>, off: vec2<f32>, src: vec2<f32>) -> vec3<f32> {
    let p = (floor(pos * src + off) + vec2<f32>(0.5, 0.5)) / src;
    return lottes_to_linear(LOTTES_BRIGHT_BOOST * port_texel(p).rgb);
}

// Distance in emulated pixels to nearest texel.
fn lottes_dist(pos: vec2<f32>, src: vec2<f32>) -> vec2<f32> {
    let p = pos * src;
    return -((p - floor(p)) - vec2<f32>(0.5));
}

// 1D Gaussian.
fn lottes_gaus(pos: f32, scale: f32) -> f32 {
    return exp2(scale * pow(abs(pos), LOTTES_SHAPE));
}

// 3-tap Gaussian filter along horz line.
fn lottes_horz3(pos: vec2<f32>, off: f32, src: vec2<f32>) -> vec3<f32> {
    let b = lottes_fetch(pos, vec2<f32>(-1.0, off), src);
    let c = lottes_fetch(pos, vec2<f32>(0.0, off), src);
    let d = lottes_fetch(pos, vec2<f32>(1.0, off), src);
    let dst = lottes_dist(pos, src).x;
    // Convert distance to weight.
    let scale = LOTTES_HARD_PIX;
    let wb = lottes_gaus(dst - 1.0, scale);
    let wc = lottes_gaus(dst + 0.0, scale);
    let wd = lottes_gaus(dst + 1.0, scale);
    // Return filtered sample.
    return (b * wb + c * wc + d * wd) / (wb + wc + wd);
}

// 5-tap Gaussian filter along horz line.
fn lottes_horz5(pos: vec2<f32>, off: f32, src: vec2<f32>) -> vec3<f32> {
    let a = lottes_fetch(pos, vec2<f32>(-2.0, off), src);
    let b = lottes_fetch(pos, vec2<f32>(-1.0, off), src);
    let c = lottes_fetch(pos, vec2<f32>(0.0, off), src);
    let d = lottes_fetch(pos, vec2<f32>(1.0, off), src);
    let e = lottes_fetch(pos, vec2<f32>(2.0, off), src);
    let dst = lottes_dist(pos, src).x;
    // Convert distance to weight.
    let scale = LOTTES_HARD_PIX;
    let wa = lottes_gaus(dst - 2.0, scale);
    let wb = lottes_gaus(dst - 1.0, scale);
    let wc = lottes_gaus(dst + 0.0, scale);
    let wd = lottes_gaus(dst + 1.0, scale);
    let we = lottes_gaus(dst + 2.0, scale);
    // Return filtered sample.
    return (a * wa + b * wb + c * wc + d * wd + e * we) / (wa + wb + wc + wd + we);
}

// 7-tap Gaussian filter along horz line.
fn lottes_horz7(pos: vec2<f32>, off: f32, src: vec2<f32>) -> vec3<f32> {
    let a = lottes_fetch(pos, vec2<f32>(-3.0, off), src);
    let b = lottes_fetch(pos, vec2<f32>(-2.0, off), src);
    let c = lottes_fetch(pos, vec2<f32>(-1.0, off), src);
    let d = lottes_fetch(pos, vec2<f32>(0.0, off), src);
    let e = lottes_fetch(pos, vec2<f32>(1.0, off), src);
    let f = lottes_fetch(pos, vec2<f32>(2.0, off), src);
    let g = lottes_fetch(pos, vec2<f32>(3.0, off), src);
    let dst = lottes_dist(pos, src).x;
    // Convert distance to weight.
    let scale = LOTTES_HARD_BLOOM_PIX;
    let wa = lottes_gaus(dst - 3.0, scale);
    let wb = lottes_gaus(dst - 2.0, scale);
    let wc = lottes_gaus(dst - 1.0, scale);
    let wd = lottes_gaus(dst + 0.0, scale);
    let we = lottes_gaus(dst + 1.0, scale);
    let wf = lottes_gaus(dst + 2.0, scale);
    let wg = lottes_gaus(dst + 3.0, scale);
    // Return filtered sample.
    return (a * wa + b * wb + c * wc + d * wd + e * we + f * wf + g * wg) / (wa + wb + wc + wd + we + wf + wg);
}

// Return scanline weight.
fn lottes_scan(pos: vec2<f32>, off: f32, src: vec2<f32>) -> f32 {
    let dst = lottes_dist(pos, src).y;
    return lottes_gaus(dst + off, LOTTES_HARD_SCAN);
}

// Return scanline weight for bloom.
fn lottes_bloom_scan(pos: vec2<f32>, off: f32, src: vec2<f32>) -> f32 {
    let dst = lottes_dist(pos, src).y;
    return lottes_gaus(dst + off, LOTTES_HARD_BLOOM_SCAN);
}

// Allow nearest three lines to effect pixel.
fn lottes_tri(pos: vec2<f32>, src: vec2<f32>) -> vec3<f32> {
    let a = lottes_horz3(pos, -1.0, src);
    let b = lottes_horz5(pos, 0.0, src);
    let c = lottes_horz3(pos, 1.0, src);
    let wa = lottes_scan(pos, -1.0, src);
    let wb = lottes_scan(pos, 0.0, src);
    let wc = lottes_scan(pos, 1.0, src);
    return a * wa + b * wb + c * wc;
}

// Small bloom.
fn lottes_bloom(pos: vec2<f32>, src: vec2<f32>) -> vec3<f32> {
    let a = lottes_horz5(pos, -2.0, src);
    let b = lottes_horz7(pos, -1.0, src);
    let c = lottes_horz7(pos, 0.0, src);
    let d = lottes_horz7(pos, 1.0, src);
    let e = lottes_horz5(pos, 2.0, src);
    let wa = lottes_bloom_scan(pos, -2.0, src);
    let wb = lottes_bloom_scan(pos, -1.0, src);
    let wc = lottes_bloom_scan(pos, 0.0, src);
    let wd = lottes_bloom_scan(pos, 1.0, src);
    let we = lottes_bloom_scan(pos, 2.0, src);
    return a * wa + b * wb + c * wc + d * wd + e * we;
}

// Distortion of scanlines, and end of screen alpha.
fn lottes_warp(pos_in: vec2<f32>) -> vec2<f32> {
    var pos = pos_in * 2.0 - 1.0;
    pos *= vec2<f32>(1.0 + (pos.y * pos.y) * LOTTES_WARP_X, 1.0 + (pos.x * pos.x) * LOTTES_WARP_Y);
    return pos * 0.5 + 0.5;
}

// Shadow mask: `shadowMask` 3, the stretched VGA style shadow mask (same as prior shaders).
fn lottes_mask(pos: vec2<f32>) -> vec3<f32> {
    var mask = vec3<f32>(LOTTES_MASK_DARK, LOTTES_MASK_DARK, LOTTES_MASK_DARK);
    let x = fract((pos.x + pos.y * 3.0) * 0.166666666);
    if x < 0.333 {
        mask.r = LOTTES_MASK_LIGHT;
    } else if x < 0.666 {
        mask.g = LOTTES_MASK_LIGHT;
    } else {
        mask.b = LOTTES_MASK_LIGHT;
    }
    return mask;
}

@fragment
fn fs_lottes(o: Out) -> @location(0) vec4<f32> {
    let p = port(o);
    let pos = lottes_warp(p.tc);
    var out_color = lottes_tri(pos, p.source);
    // Add Bloom
    out_color += lottes_bloom(pos, p.source) * LOTTES_BLOOM_AMOUNT;
    // `vTexCoord.xy / OutputSize.zw`: the fragment's place in surface pixels.
    out_color *= lottes_mask(p.uv * p.output * 1.000001);
    return vec4<f32>(lottes_to_srgb(out_color), 1.0);
}
