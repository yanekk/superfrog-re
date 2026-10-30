//! What T23's end-to-end tests share: the engine run headless beside the app, and the checks on
//! a window shot and a home's files.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use sf_core::input::FrameInput;
use sf_core::{Game, Rng, Start};
use sf_e2e::{Image, Shot};

/// The asset cache a home holds, by the name this build reads.
pub fn cache_file(home: &Path) -> PathBuf {
    home.join(format!("cache/assets-v{}.bin", sf_assets::CACHE_FORMAT))
}

/// The game data the app ran on: its home's cache, decoded.
pub fn cache_data(home: &Path) -> sf_core::GameData {
    sf_assets::cache_decode(&std::fs::read(cache_file(home)).unwrap()).expect("the home's cache")
}

/// The engine, headless, from the start a scripted run makes (power-on, seed 0 0).
pub fn headless(data: &sf_core::GameData) -> Game {
    Game::new(data, Start::PowerOn { seed: Rng::seeded(0, 0) })
}

/// The engine's pictures for steps 1..=`frames` of `input`, as the app shows them.
pub fn headless_frames(
    data: &sf_core::GameData,
    frames: u64,
    input: impl Fn(u64) -> FrameInput,
) -> Vec<Image> {
    let mut g = headless(data);
    (1..=frames).map(|n| Image::of_picture(&g.step(input(n)).picture)).collect()
}

/// The files the app writes into a home (the harness's `capture-*` and the lock aside).
pub fn app_files(home: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(home)
        .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.retain(|n| !n.starts_with("capture-") && n != "lock");
    names.sort();
    names
}

/// The shot is `frame` drawn at a whole-number scale, centred on the surface, black around it,
/// every game pixel a solid square of one colour: never smeared. Returns the scale, in pixels a
/// game pixel.
pub fn assert_integer_scaled(shot: &Image, info: &Shot, frame: &Image) -> u32 {
    assert_eq!((shot.width, shot.height), info.surface, "the shot is the whole surface");
    let (x, y, w, h) = info.viewport;
    let s = w / frame.width;
    assert!(s >= 1, "{info:?}");
    assert_eq!((w, h), (frame.width * s, frame.height * s), "a whole multiple: {info:?}");
    assert_eq!(
        (x, y),
        ((info.surface.0 - w) / 2, (info.surface.1 - h) / 2),
        "centred: {info:?}"
    );
    let mut wrong = 0;
    for py in 0..shot.height {
        for px in 0..shot.width {
            let inside = (x..x + w).contains(&px) && (y..y + h).contains(&py);
            let want = if inside { frame.rgb((px - x) / s, (py - y) / s) } else { [0, 0, 0] };
            if shot.rgb(px, py) != want {
                wrong += 1;
            }
        }
    }
    assert_eq!(wrong, 0, "{wrong} pixels of the shot are not the frame at {s}×: {info:?}");
    s
}

/// The frame has something on it besides black.
pub fn not_black(img: &Image) -> bool {
    img.rgba.chunks(4).any(|p| p[..3] != [0, 0, 0])
}
