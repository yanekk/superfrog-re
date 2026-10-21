/*
    CRT-interlaced

    Copyright (C) 2010-2012 cgwg, Themaister and DOLLS

    This program is free software; you can redistribute it and/or modify it
    under the terms of the GNU General Public License as published by the Free
    Software Foundation; either version 2 of the License, or (at your option)
    any later version.

    (cgwg gave their consent to have the original version of this shader
    distributed under the GPL in this message:

    http://board.byuu.org/viewtopic.php?p=26075#p26075

    "Feel free to distribute my shaders under the GPL. After all, the
    barrel distortion code was taken from the Curvature shader, which is
    under the GPL."
    )
    This shader variant is pre-configured with screen curvature
*/

// View → Filter ▸ Geom (DESIGN §2.7): a WGSL port of libretro slang-shaders
// `crt/shaders/crt-geom.slang` (master, 2026-10-20), its `#pragma parameter` defaults fixed as
// constants and its compiled-in choices kept (LINEAR_PROCESSING and OVERSAMPLE on, USEGAUSSIAN
// off). Only the paths those defaults take are ported: horizontal scanlines, curvature on, no
// tilt, mask pattern 1. The one departure is `port.wgsl`'s quarter-pixel nudge at 2×. Appended
// after `port.wgsl`.

const GEOM_CRT_GAMMA: f32 = 2.4;
const GEOM_MONITOR_GAMMA: f32 = 2.2;
const GEOM_D: f32 = 1.5;
const GEOM_R: f32 = 2.0;
const GEOM_CORNER_SIZE: f32 = 0.03;
const GEOM_CORNER_SMOOTH: f32 = 1000.0;
const GEOM_OVERSCAN: vec2<f32> = vec2<f32>(100.0, 100.0);
const GEOM_DOTMASK: f32 = 0.3;
const GEOM_DOTMASK_BRIGHTBOOST: f32 = 0.0;
const GEOM_SHARPER: f32 = 1.0;
const GEOM_SCANLINE_WEIGHT: f32 = 0.3;
const GEOM_LUM: f32 = 0.0;
// `x_tilt`, `y_tilt` 0.
const GEOM_SINANGLE: vec2<f32> = vec2<f32>(0.0, 0.0);
const GEOM_COSANGLE: vec2<f32> = vec2<f32>(1.0, 1.0);
// The original's `vec2(invert_aspect > 0.5 ? (0.75, 1.0) : (1.0, 0.75))` reads its parentheses
// as GLSL's comma operator, so with `invert_aspect` 0 it is `vec2(0.75)`: what RetroArch runs.
const GEOM_ASPECT: vec2<f32> = vec2<f32>(0.75, 0.75);
const GEOM_PI: f32 = 3.141592653589;

fn geom_fix(c: f32) -> f32 {
    return max(abs(c), 1e-5);
}

// `TEX2D` with LINEAR_PROCESSING.
fn geom_tex2d(c: vec2<f32>) -> vec4<f32> {
    return pow(port_texel(c), vec4<f32>(GEOM_CRT_GAMMA));
}

fn geom_intersect(xy: vec2<f32>) -> f32 {
    let A = dot(xy, xy) + GEOM_D * GEOM_D;
    let B = 2.0 * (GEOM_R * (dot(xy, GEOM_SINANGLE) - GEOM_D * GEOM_COSANGLE.x * GEOM_COSANGLE.y) - GEOM_D * GEOM_D);
    let C = GEOM_D * GEOM_D + 2.0 * GEOM_R * GEOM_D * GEOM_COSANGLE.x * GEOM_COSANGLE.y;
    return (-B - sqrt(B * B - 4.0 * A * C)) / (2.0 * A);
}

fn geom_bkwtrans(xy: vec2<f32>) -> vec2<f32> {
    let c = geom_intersect(xy);
    let point = (vec2<f32>(c, c) * xy - vec2<f32>(-GEOM_R, -GEOM_R) * GEOM_SINANGLE) / vec2<f32>(GEOM_R, GEOM_R);
    let poc = point / GEOM_COSANGLE;
    let tang = GEOM_SINANGLE / GEOM_COSANGLE;
    let A = dot(tang, tang) + 1.0;
    let B = -2.0 * dot(poc, tang);
    let C = dot(poc, poc) - 1.0;
    let a = (-B + sqrt(B * B - 4.0 * A * C)) / (2.0 * A);
    let uv = (point - a * GEOM_SINANGLE) / GEOM_COSANGLE;
    let r = geom_fix(GEOM_R * acos(a));
    return uv * r / sin(r / GEOM_R);
}

fn geom_fwtrans(uv_in: vec2<f32>) -> vec2<f32> {
    let r = geom_fix(sqrt(dot(uv_in, uv_in)));
    let uv = uv_in * sin(r / GEOM_R) / r;
    let x = 1.0 - cos(r / GEOM_R);
    let D = GEOM_D / GEOM_R + x * GEOM_COSANGLE.x * GEOM_COSANGLE.y + dot(uv, GEOM_SINANGLE);
    return GEOM_D * (uv * GEOM_COSANGLE - x * GEOM_SINANGLE) / D;
}

// The original computes this once in its vertex stage; here each fragment does.
fn geom_maxscale() -> vec3<f32> {
    let c = geom_bkwtrans(-GEOM_R * GEOM_SINANGLE / (1.0 + GEOM_R / GEOM_D * GEOM_COSANGLE.x * GEOM_COSANGLE.y));
    let a = vec2<f32>(0.5, 0.5) * GEOM_ASPECT;
    let lo = vec2<f32>(geom_fwtrans(vec2<f32>(-a.x, c.y)).x, geom_fwtrans(vec2<f32>(c.x, -a.y)).y) / GEOM_ASPECT;
    let hi = vec2<f32>(geom_fwtrans(vec2<f32>(a.x, c.y)).x, geom_fwtrans(vec2<f32>(c.x, a.y)).y) / GEOM_ASPECT;
    return vec3<f32>((hi + lo) * GEOM_ASPECT * 0.5, max(hi.x - lo.x, hi.y - lo.y));
}

// Calculate the influence of a scanline on the current pixel.
//
// 'distance' is the distance in texture coordinates from the current
// pixel to the scanline in question.
// 'color' is the colour of the scanline at the horizontal location of
// the current pixel.
fn geom_scanline_weights(distance: f32, color: vec4<f32>) -> vec4<f32> {
    // The non-gaussian beam profile (USEGAUSSIAN off).
    let wid = 2.0 + 2.0 * pow(color, vec4<f32>(4.0));
    let weights = vec4<f32>(distance / GEOM_SCANLINE_WEIGHT);
    return (GEOM_LUM + 1.4) * exp(-pow(weights * inverseSqrt(0.5 * wid), wid)) / (0.6 + 0.2 * wid);
}

fn geom_transform(coord_in: vec2<f32>, stretch: vec3<f32>) -> vec2<f32> {
    let coord = (coord_in - vec2<f32>(0.5, 0.5)) * GEOM_ASPECT * stretch.z + stretch.xy;
    return geom_bkwtrans(coord) / (GEOM_OVERSCAN / 100.0) / GEOM_ASPECT + vec2<f32>(0.5, 0.5);
}

fn geom_corner(coord_in: vec2<f32>) -> f32 {
    var coord = (coord_in - vec2<f32>(0.5)) * (GEOM_OVERSCAN / 100.0) + vec2<f32>(0.5, 0.5);
    coord = min(coord, vec2<f32>(1.0) - coord) * GEOM_ASPECT;
    let cdist = vec2<f32>(GEOM_CORNER_SIZE);
    coord = cdist - min(coord, cdist);
    let dist = sqrt(dot(coord, coord));
    return clamp((cdist.x - dist) * GEOM_CORNER_SMOOTH, 0.0, 1.0);
}

// Masks adapted from cgwg's crt-geom-deluxe LUTs: `mask_type` 1 only, the classic aperture for
// RGB panels (aperture_1_2_bgr), with `mask_intensity` 1. Its alpha is 3/6.
fn geom_mask_weights(coord: vec2<f32>) -> vec3<f32> {
    let magenta = vec3<f32>(1.0, 0.0, 1.0);
    let green = vec3<f32>(0.0, 1.0, 0.0);
    return mix(magenta, green, floor(port_mod(coord.x, 2.0)));
}

@fragment
fn fs_geom(o: Out) -> @location(0) vec4<f32> {
    let p = port(o);
    // The vertex stage's `TexCoord * vec2(1.00001)`.
    let tex_coord = p.tc * 1.00001;
    let mask_coord = p.uv * 1.00001;
    let stretch = geom_maxscale();
    let texture_size = vec2<f32>(GEOM_SHARPER * p.source.x, p.source.y);
    // `ilfac`: 1 unless the frame is 400 lines or more (`interlace_detect`), and Superfrog's
    // frames are at most 261, so `ilvec` is 0 and the frame count is never read.
    let ilfac = vec2<f32>(1.0, 1.0);
    // The size of one texel, in texture-coordinates.
    let one = ilfac / texture_size;

    // Texture coordinates of the texel containing the active pixel.
    var xy = geom_transform(tex_coord, stretch);
    let cval = geom_corner(xy);

    // Of all the pixels that are mapped onto the texel we are
    // currently rendering, which pixel are we currently rendering?
    let ratio_scale = (xy * texture_size - vec2<f32>(0.5, 0.5)) / ilfac;
    var uv_ratio = fract(ratio_scale);

    // Snap to the center of the underlying texel.
    xy = (floor(ratio_scale) * ilfac + vec2<f32>(0.5, 0.5)) / texture_size;

    // Calculate Lanczos scaling coefficients describing the effect
    // of various neighbour texels in a scanline on the current
    // pixel.
    var coeffs = GEOM_PI * vec4<f32>(1.0 + uv_ratio.x, uv_ratio.x, 1.0 - uv_ratio.x, 2.0 - uv_ratio.x);
    // Prevent division by zero.
    coeffs = max(abs(coeffs), vec4<f32>(1e-5));
    // Lanczos2 kernel.
    coeffs = 2.0 * sin(coeffs) * sin(coeffs / 2.0) / (coeffs * coeffs);
    // Normalize.
    coeffs /= dot(coeffs, vec4<f32>(1.0));

    // Calculate the effective colour of the current and next
    // scanlines at the horizontal location of the current pixel,
    // using the Lanczos coefficients above.
    let col = clamp(
        mat4x4<f32>(
            geom_tex2d(xy + vec2<f32>(-one.x, 0.0)),
            geom_tex2d(xy),
            geom_tex2d(xy + vec2<f32>(one.x, 0.0)),
            geom_tex2d(xy + vec2<f32>(2.0 * one.x, 0.0)),
        ) * coeffs,
        vec4<f32>(0.0),
        vec4<f32>(1.0),
    );
    let col2 = clamp(
        mat4x4<f32>(
            geom_tex2d(xy + vec2<f32>(-one.x, one.y)),
            geom_tex2d(xy + vec2<f32>(0.0, one.y)),
            geom_tex2d(xy + one),
            geom_tex2d(xy + vec2<f32>(2.0 * one.x, one.y)),
        ) * coeffs,
        vec4<f32>(0.0),
        vec4<f32>(1.0),
    );

    // Calculate the influence of the current and next scanlines on
    // the current pixel.
    var weights = geom_scanline_weights(uv_ratio.y, col);
    var weights2 = geom_scanline_weights(1.0 - uv_ratio.y, col2);

    // OVERSAMPLE.
    let filter_ = fwidth(ratio_scale.y);
    uv_ratio.y = uv_ratio.y + 1.0 / 3.0 * filter_;
    weights = (weights + geom_scanline_weights(uv_ratio.y, col)) / 3.0;
    weights2 = (weights2 + geom_scanline_weights(abs(1.0 - uv_ratio.y), col2)) / 3.0;
    uv_ratio.y = uv_ratio.y - 2.0 / 3.0 * filter_;
    weights = weights + geom_scanline_weights(abs(uv_ratio.y), col) / 3.0;
    weights2 = weights2 + geom_scanline_weights(abs(1.0 - uv_ratio.y), col2) / 3.0;

    let mul_res = (col * weights + col2 * weights2).rgb * vec3<f32>(cval);

    // Shadow mask: the subpixel mask code instead of LUTs.
    let mask = geom_mask_weights(mask_coord * p.output);
    let mask_alpha = 3.0 / 6.0;
    // `u_tex_size1`: surface pixels a game pixel.
    let tex_size1 = p.output / p.source;

    // count of total bright pixels is encoded in the mask's alpha channel
    let nbright = 255.0 - 255.0 * mask_alpha;
    // fraction of bright pixels in the mask
    let fbright = nbright / (tex_size1.x * tex_size1.y);
    // average darkening factor of the mask
    let aperture_average = mix(1.0 - GEOM_DOTMASK * (1.0 - GEOM_DOTMASK_BRIGHTBOOST), 1.0, fbright);
    // colour of dark mask pixels
    let clow = vec3<f32>(1.0 - GEOM_DOTMASK) * mul_res + vec3<f32>(GEOM_DOTMASK * GEOM_DOTMASK_BRIGHTBOOST) * mul_res * mul_res;
    let ifbright = 1.0 / fbright;
    // colour of bright mask pixels
    let chi = vec3<f32>(ifbright * aperture_average) * mul_res - vec3<f32>(ifbright - 1.0) * clow;
    let cout = mix(clow, chi, mask); // mask texture selects dark vs bright

    // Convert the image gamma for display on our output device.
    return vec4<f32>(pow(cout, vec3<f32>(1.0 / GEOM_MONITOR_GAMMA)), 1.0);
}
