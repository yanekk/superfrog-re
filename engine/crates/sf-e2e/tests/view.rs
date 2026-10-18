//! T27: View → CRT Filter and View → Widescreen (DESIGN §2.7), driven through the app's script
//! layer (`N: menu crt`, `N: menu widescreen`, the settings file) and checked against the engine
//! run headless.
//!
//! Widescreen draws 67 extra columns each side (16:9 of 256 lines). On a level they are the
//! level's; on every other screen (title, story intro, slot machine) each line's colour 0, the
//! border an Amiga monitor shows outside the display window (owner, 2026-10-18). The game is the
//! same in every mode: its state and the centre 320 columns of every frame are the 4:3 run's.
//!
//! The CRT filter runs after the integer scaling: the frames the game hands over are the same with
//! it on, only the window differs.

mod common;

use common::{assert_integer_scaled, cache_data, not_black};
use sf_core::input::{FrameInput, Joy};
use sf_core::{DisplayData, Game, Picture, Progress, Rng, Scene, Start, View};
use sf_e2e::{App, Captured, Image, LaunchOpts, Shot};

const WIDE: View = View {
    extra_columns_each_side: 67,
};
const N: u32 = 67;

/// Image columns `x0..x0 + w`.
fn columns(img: &Image, x0: u32, w: u32) -> Image {
    let rgba = (0..img.height)
        .flat_map(|y| {
            let at = ((y * img.width + x0) * 4) as usize;
            img.rgba[at..at + (w * 4) as usize].to_vec()
        })
        .collect();
    Image {
        width: w,
        height: img.height,
        rgba,
    }
}

/// A widescreen frame less its extra columns: the 4:3 frame's place (320 columns on a level; a
/// screen with a wider display window, like the progress map's 334, keeps its own).
fn centre(img: &Image) -> Image {
    columns(img, N, img.width - 2 * N)
}

/// W3-1 by Continue at step 10 (progress World 3). Its loading picture and map take until about
/// step 1 664; the right arrow held 1 700..1 900 then scrolls the camera mid-level (x 70 → 272).
const LEVEL_SCRIPT: &str = "10: menu continue\n1700: down ArrowRight\n1900: up ArrowRight\n";
const LEVEL_FRAMES: u64 = 2000;

fn level_input(n: u64) -> FrameInput {
    if (1700..1900).contains(&n) { FrameInput::joy(Joy::RIGHT) } else { FrameInput::NONE }
}

/// The game Continue starts at step 10 of a scripted run (seed 0 0), headless, drawn for `view`.
fn w3_1(data: &sf_core::GameData, view: View) -> Game {
    let display = std::sync::Arc::new(DisplayData::new(data));
    let mut g = Game::start_at_with(display, &Progress { world: 3, level: 1 }, Rng::seeded(0, 0));
    g.set_view(view);
    g
}

fn level_run(widescreen: bool) -> (App, Captured) {
    let mut app = App::launch(LaunchOpts {
        max_frames: LEVEL_FRAMES,
        timeout_s: 300,
        ..Default::default()
    });
    std::fs::write(app.home.join("progress.toml"), "world = 3\nlevel = 1\n").unwrap();
    let script = if widescreen { format!("5: menu widescreen\n{LEVEL_SCRIPT}") } else { LEVEL_SCRIPT.into() };
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    (app, c)
}

/// Picture pixels left of picture column `x_end` on rows `rows` are all colour index 0.
fn index_0(p: &Picture, x_end: usize, rows: std::ops::Range<usize>) -> bool {
    rows.into_iter()
        .all(|y| p.pixels[y * p.width as usize..][..x_end].iter().all(|&i| i == 0))
}

#[test]
fn widescreen_mid_level_keeps_the_centre_and_the_game_of_the_4_3_run() {
    let (app, wide) = level_run(true);
    let (_narrow_home, narrow) = level_run(false);
    let data = cache_data(&app.home);
    assert_eq!(wide.inputs, narrow.inputs);

    // Every frame from the switch on is 454 wide, its centre the 4:3 run's: the title's frames
    // before Continue, W3-1's loading picture and map, and W3-1. From Continue on, the app's
    // frames are the engine's widescreen pictures headless, and the game, state and probe alike,
    // is the 4:3 game's.
    let mut g_wide = w3_1(&data, WIDE);
    let mut g_narrow = w3_1(&data, View::default());
    let mut scrolled = false;
    for n in 0..=LEVEL_FRAMES {
        let (w, c) = (wide.read_frame(n), narrow.read_frame(n));
        if n < 5 {
            assert_eq!(w, c, "frame {n}, before the switch");
            continue;
        }
        // The display window is the screen's own, widened (the boot screens' are shorter, the
        // map's wider).
        assert_eq!((w.width, w.height), (c.width + 2 * N, c.height), "frame {n}");
        assert_eq!(centre(&w), c, "frame {n}: the centre is the 4:3 frame");
        if n < 10 {
            continue;
        }
        let pw = g_wide.step(level_input(n)).picture;
        let pn = g_narrow.step(level_input(n)).picture;
        assert_eq!(w, Image::of_picture(&pw), "frame {n} is not the engine's");
        assert_eq!(c, Image::of_picture(&pn), "frame {n}, 4:3");
        assert_eq!(g_wide.probe(), g_narrow.probe(), "frame {n}: widescreen changed the game");
        assert!(g_wide.state() == g_narrow.state(), "frame {n}: widescreen changed the state");
        // Mid-level, the side columns show the level: not all colour 0 below the panel.
        if g_wide.scene() == Scene::Level && g_wide.state().camera().x > N + 16 {
            scrolled = true;
            assert!(!index_0(&pw, N as usize, 40..pw.height as usize), "frame {n}: left side empty");
            let right = pw.pixels.chunks(pw.width as usize).skip(40).any(|r| r[(N + 320) as usize..].iter().any(|&i| i != 0));
            assert!(right, "frame {n}: right side empty");
        }
    }
    assert!(scrolled, "camera at x {}: the camera never left the level's left edge: not a mid-level check", g_wide.state().camera().x);
}

#[test]
fn widescreen_at_a_level_s_left_edge_shows_colour_0_off_the_map_and_nothing_wraps() {
    let data = cache_data(&App::launch(LaunchOpts::default()).home);
    // W1-1 entered as the original enters it: the camera at x 16.
    let mut g = Game::new(&data, Start::Level("W1-1".parse().unwrap()));
    g.set_view(WIDE);
    for n in 1..=40u64 {
        let p = g.step(FrameInput::NONE).picture;
        let x = g.state().camera().x as i32;
        assert!(x < N as i32, "frame {n}: the camera is at the left edge ({x})");
        // Picture column N shows level x `x`; everything left of level x 0 is off the map: colour
        // 0, not the map's other end. Play-field rows only, below the panel.
        let off = (N as i32 - x - 2).max(0) as usize;
        assert!(index_0(&p, off, 20..p.height as usize), "frame {n}: off the map is not colour 0");
    }
}

/// Every frame of a non-level screen: the 4:3 picture in the centre, colour 0 at the sides.
fn assert_border_sides(g: &mut Game, frames: u32, input: impl Fn(u32) -> FrameInput, what: &str) {
    let mut narrow = g.clone();
    narrow.set_view(View::default());
    g.set_view(WIDE);
    let mut seen = Vec::new();
    for n in 1..=frames {
        let pw = g.step(input(n)).picture;
        let pn = narrow.step(input(n)).picture;
        seen.push(g.scene());
        assert_eq!((pw.width, pw.left), (pn.width + 2 * N as u16, pn.left - N as i32), "{what} {n}");
        assert_eq!(pw.narrowed(WIDE), pn, "{what} {n}: the centre is the 4:3 picture");
        let side = |y: usize| {
            let row = &pw.pixels[y * pw.width as usize..][..pw.width as usize];
            row[..N as usize].iter().chain(&row[(N + pn.width as u32) as usize..]).all(|&i| i == 0)
        };
        assert!((0..pw.height as usize).all(side), "{what} {n}: a side is not colour 0");
        assert!(g.state() == narrow.state(), "{what} {n}: widescreen changed the state");
    }
    assert!(seen.iter().all(|s| *s != Scene::Level), "{what}: {seen:?}");
}

#[test]
fn widescreen_on_the_title_story_intro_and_slot_machine_shows_the_border_at_the_sides() {
    let data = cache_data(&App::launch(LaunchOpts::default()).home);
    // Power-on to the title, and fire to its menu.
    let mut g = Game::new(&data, Start::PowerOn { seed: Rng::seeded(0, 0) });
    assert_border_sides(&mut g, 300, |n| if n == 100 || n == 200 { FrameInput::joy(Joy::FIRE) } else { FrameInput::NONE }, "title");
    // The story intro (disk 3).
    let mut g = Game::new(&data, Start::Disk3 { seed: Rng::seeded(0, 0) });
    assert_border_sides(&mut g, 300, |_| FrameInput::NONE, "intro");
    // The slot machine, entered from a level as the tally enters it, fire pressed.
    let mut g = Game::new(&data, Start::Level("W1-1".parse().unwrap()));
    g.enter_slot(3);
    assert_border_sides(&mut g, 300, |n| if n % 80 == 40 { FrameInput::joy(Joy::FIRE) } else { FrameInput::NONE }, "slot");

    // The app shows the title's widescreen frames as the engine draws them.
    let mut app = App::launch(LaunchOpts {
        max_frames: 300,
        ..Default::default()
    });
    std::fs::write(app.home.join("settings.toml"), "widescreen = true\n").unwrap();
    let c = app.run_script("");
    assert!(c.exit.success(), "{}", c.stderr);
    let mut g = Game::new(&data, Start::PowerOn { seed: Rng::seeded(0, 0) });
    g.set_view(WIDE);
    assert_eq!(c.frames[0], Image::of_picture(&g.render(WIDE)));
    for n in 1..=300 {
        assert_eq!(c.frames[n], Image::of_picture(&g.step(FrameInput::NONE).picture), "frame {n}");
    }
    assert!(not_black(&c.frames[150]));
}

/// A run with `settings` in its home, a shot at 150: the app, whose home holds the shot until it is
/// dropped, and the capture. (Fifty frames after it: a run whose last step lands in the same
/// catch-up as the shot's ends before the shot is taken.)
fn shot_run(settings: &str, scale: Option<u32>) -> (App, Captured) {
    let mut app = App::launch(LaunchOpts {
        max_frames: 200,
        scale,
        ..Default::default()
    });
    std::fs::write(app.home.join("settings.toml"), settings).unwrap();
    let c = app.run_script("150: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    (app, c)
}

#[test]
fn widescreen_at_1x_2x_and_3x_is_a_16_9_window_at_a_whole_scale() {
    for n in 1..=3u32 {
        let (_app, c) = shot_run("widescreen = true\n", Some(n));
        let (img, info) = c.read_shot(150);
        let sf = info.scale_factor as u32;
        assert_eq!(info.surface, (454 * n * sf, 256 * n * sf), "{n}× widescreen: {info:?}");
        let frame = c.read_frame(info.frame);
        assert_eq!((frame.width, frame.height), (454, 256));
        assert_eq!(assert_integer_scaled(&img, &info, &frame), n * sf);
    }
}

/// Mean and largest per-channel difference between the shot's picture and `frame` scaled by `s`.
fn difference(img: &Image, info: &Shot, frame: &Image, s: u32) -> (f64, u8) {
    let (x, y, w, h) = info.viewport;
    let (mut sum, mut max, mut count) = (0u64, 0u8, 0u64);
    for py in y..y + h {
        for px in x..x + w {
            let (a, b) = (img.rgb(px, py), frame.rgb((px - x) / s, (py - y) / s));
            for ch in 0..3 {
                let d = a[ch].abs_diff(b[ch]);
                sum += d as u64;
                max = max.max(d);
                count += 1;
            }
        }
    }
    (sum as f64 / count as f64, max)
}

#[test]
fn crt_changes_only_the_scaled_picture_and_stays_legible_at_1x() {
    for n in 1..=3u32 {
        let (_plain_app, plain) = shot_run("", Some(n));
        let (_crt_app, crt) = shot_run("crt = true\n", Some(n));
        assert!(crt.frames == plain.frames, "{n}×: the CRT filter changed a game frame");
        let (pi, pinfo) = plain.read_shot(150);
        let (ci, cinfo) = crt.read_shot(150);
        assert_eq!(pinfo, cinfo, "{n}×: the same place and scale");
        assert_ne!(pi, ci, "{n}×: the CRT filter changed nothing on screen");
        let frame = crt.read_frame(cinfo.frame);
        let s = cinfo.viewport.2 / frame.width;
        let (mean, max) = difference(&ci, &cinfo, &frame, s);
        // Black around the picture stays black.
        assert_eq!(ci.rgb(0, 0), [0, 0, 0]);
        if s == 1 {
            // At 1× only the vignette: every pixel within 10% of the frame's.
            assert!(max <= 26 && mean < 6.0, "1×: mean {mean}, max {max}: not legible");
        } else {
            // From 2× scanlines: a game line's middle brighter than its top edge, on average.
            let (x, y, w, h) = cinfo.viewport;
            let (mut mid, mut edge) = (0u64, 0u64);
            for gy in 0..h / s {
                for px in x..x + w {
                    let at = |row: u32| ci.rgb(px, y + gy * s + row).iter().map(|&c| c as u64).sum::<u64>();
                    mid += at(s / 2);
                    edge += at(0);
                }
            }
            assert!(mid > edge, "{n}× ({s} pixels a game pixel): no scanlines ({mid} vs {edge})");
            assert!(mean < 40.0, "{n}×: mean difference {mean}");
        }
    }
}

#[test]
fn crt_and_widescreen_from_the_menu_are_remembered_by_the_next_launch() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 200,
        ..Default::default()
    });
    let c = app.run_script("10: menu crt\n20: menu widescreen\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!((c.frames[19].width, c.frames[20].width), (320, 454), "widescreen from step 20");
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("crt = true") && settings.contains("widescreen = true"), "{settings}");

    // The next launch opens with both on: a 16:9 window from frame 0, through the filter.
    let c = app.run_script("150: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.frames.iter().all(|f| f.width == 454), "widescreen from the start");
    let (img, info) = c.read_shot(150);
    let sf = info.scale_factor as u32;
    assert_eq!(info.surface, (454 * sf, 256 * sf));
    let frame = c.read_frame(info.frame);
    let (x, y, _, _) = info.viewport;
    assert!(not_black(&frame));
    let plain = (0..frame.height).flat_map(|gy| (0..454).map(move |gx| (gx, gy))).all(|(gx, gy)| {
        img.rgb(x + gx * sf, y + gy * sf) == frame.rgb(gx, gy)
    });
    assert!(!plain, "the CRT filter is on");

    // And off again, both remembered off.
    let c = app.run_script("5: menu crt\n6: menu widescreen\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(settings.contains("crt = false") && settings.contains("widescreen = false"), "{settings}");
    assert_eq!(c.frames.last().unwrap().width, 320);
}

#[test]
#[ignore = "takes over the display: SF_E2E_OS=1 cargo test -p sf-e2e --test view -- --ignored"]
fn fullscreen_widescreen_is_the_largest_whole_scale_of_the_16_9_picture() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "the fullscreen run takes over the display; run it with SF_E2E_OS=1"
    );
    let mut app = App::launch(LaunchOpts {
        max_frames: 200,
        extra_args: vec!["--allow-fullscreen".into()],
        ..Default::default()
    });
    std::fs::write(app.home.join("settings.toml"), "fullscreen = true\nwidescreen = true\n").unwrap();
    let c = app.run_script("150: shot\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let (img, info) = c.read_shot(150);
    let frame = c.read_frame(info.frame);
    assert_eq!((frame.width, frame.height), (454, 256));
    let s = assert_integer_scaled(&img, &info, &frame);
    let fit = (info.surface.0 / 454).min(info.surface.1 / 256);
    assert_eq!(s, fit, "the largest whole scale: {info:?}");
}

/// DESIGN §2.5 with the filter on: over 3 000 frames (a minute) at 3×, widescreen, every frame
/// stepped is presented. The shipped build's pacing, so in release (the debug engine drops frames
/// with or without the filter, more as the machine is busier); long, so outside the test block.
/// T27 ran it (FINDINGS).
#[test]
#[ignore = "a minute long: cargo test --release -p sf-e2e --test view -- --ignored crt_drops"]
fn crt_drops_no_frame_over_3000_frames() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 3000,
        timeout_s: 150,
        scale: Some(3),
        ..Default::default()
    });
    std::fs::write(app.home.join("settings.toml"), "crt = true\nwidescreen = true\n").unwrap();
    let c = app.run_script_unread("");
    assert!(c.exit.success(), "{}", c.stderr);
    let pacing = std::fs::read_to_string(c.dir.join("pacing.txt")).unwrap();
    eprintln!("{pacing}");
    assert!(pacing.contains("stepped 3000\n"), "{pacing}");
    assert!(pacing.contains("dropped 0\n"), "{pacing}");
}

