// View → CRT Filter (DESIGN §2.7): the scaled picture as a monitor shows it. Appended to
// `video.rs`'s present shader, whose vertex stage, frame texture and sampler it reads. It runs
// after integer scaling and changes presentation only: the game's frame comes in untouched.
//
// Each effect grows with the scale, so at 1× (no room inside a game pixel for a scanline or a
// mask) only the gentle vignette is left and the picture stays legible.

struct Crt {
    // The picture's place on the surface, in surface pixels: its top-left corner.
    origin: vec2<f32>,
    // Surface pixels a game pixel.
    scale: f32,
    pad: f32,
};

@group(0) @binding(2) var<uniform> crt: Crt;

@fragment
fn fs_crt(o: Out) -> @location(0) vec4<f32> {
    let colour = textureSample(frame, pick, o.uv).rgb;
    let s = crt.scale;
    let at = floor(o.pos.xy - crt.origin);

    // Scanlines: each game line brightest across its middle, darker at its top and bottom edge.
    let row = at.y - s * floor(at.y / s);
    let fy = (row + 0.5) / s;
    let line_depth = 0.45 * clamp((s - 1.0) / 3.0, 0.0, 1.0);
    let line = 1.0 - line_depth * pow(abs(fy - 0.5) * 2.0, 2.0);

    // An aperture-grille mask: screen columns in red, green, blue triads, from 3× up.
    let mask_depth = 0.15 * clamp((s - 2.0) / 4.0, 0.0, 1.0);
    let triad = i32(at.x - 3.0 * floor(at.x / 3.0));
    var mask = vec3<f32>(1.0 - mask_depth);
    mask[triad] = 1.0;

    // The light the scanlines and the mask take away, given back on average.
    let gain = 1.0 / ((1.0 - line_depth / 3.0) * (1.0 - mask_depth * 2.0 / 3.0));

    // The tube's corners a little darker.
    let d = o.uv - vec2<f32>(0.5, 0.5);
    let vignette = 1.0 - 0.2 * dot(d, d);

    return vec4<f32>(clamp(colour * line * mask * gain * vignette, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
