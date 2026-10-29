// What RetroArch hands a single-pass slang shader, for the libretro ports (`crt_lottes.wgsl`,
// `crt_geom.wgsl`, `crt_easymode.wgsl`). Appended to `video.rs`'s present shader after `crt.wgsl`,
// whose `Crt` uniform it reads. Each port's `.slangp` is one pass, `filter_linear0 = false`,
// `scale_type0 = viewport`: the app's frame texture, its nearest sampler and its viewport pass.

struct Port {
    // `vTexCoord` as RetroArch interpolates it: 0 to 1 across the viewport, at the fragment's centre.
    uv: vec2<f32>,
    // The same, a quarter game pixel lower at exactly 2 surface pixels a game pixel (brief,
    // plan review): there the two rows of a game line sit at 0.25 and 0.75 of it, symmetric
    // about its centre for all three ports' scanline weights, so as shipped they light both rows
    // equally and draw no scanlines. Nudged, one row is on the line's centre and the other in
    // the gap. Used for the picture's sampling only; each port's mask reads `uv`.
    tc: vec2<f32>,
    // `SourceSize.xy`: the game frame, in game pixels.
    source: vec2<f32>,
    // `OutputSize.xy`: the viewport, in surface pixels.
    output: vec2<f32>,
};

fn port(o: Out) -> Port {
    let source = vec2<f32>(textureDimensions(frame));
    let output = source * crt.scale;
    let uv = (o.pos.xy - crt.origin) / output;
    let nudge = select(0.0, 0.25 / source.y, crt.scale == 2.0);
    return Port(uv, uv + vec2<f32>(0.0, nudge), source, output);
}

// `texture(Source, c)` as RetroArch samples it here: nearest, and black outside the frame
// (slang's default wrap mode, clamp_to_border), which the curved ports rely on at the corners.
// `select`, not an early return, so callers stay in uniform control flow for `fwidth`.
fn port_texel(c: vec2<f32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(frame));
    let i = vec2<i32>(floor(c * vec2<f32>(size)));
    let inside = all(i >= vec2<i32>(0)) && all(i < size);
    let v = textureLoad(frame, clamp(i, vec2<i32>(0), size - vec2<i32>(1)), 0);
    return select(vec4<f32>(0.0), v, inside);
}

// GLSL's `mod`: the remainder towards minus infinity (WGSL's `%` truncates).
fn port_mod(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}
