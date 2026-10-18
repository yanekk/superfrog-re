//! T23: the picture in the window (DESIGN §2.7): integer scaling at 1×, 2× and 3×, a resized
//! window, the minimum size, and fullscreen by its shortcut. Each check reads the window back as
//! it was presented (the script's `shot`) and compares it with the frame the app captured.
//!
//! The fullscreen run takes over the display, so like the OS layer it runs only under
//! `SF_E2E_OS=1` (README.md); the test block checks that a scripted run cannot go fullscreen
//! without `--allow-fullscreen`.

mod common;

use common::{assert_integer_scaled, not_black};
use sf_e2e::{App, Captured, LaunchOpts};

/// The frame with the crack intro's eyes on it (power-on, seed 0 0, no input): not black.
const SHOT: u64 = 150;

fn shot(c: &Captured, n: u64) -> u32 {
    let (img, info) = c.read_shot(n);
    let frame = c.read_frame(info.frame);
    assert!(not_black(&frame), "frame {} is black: a weak check", info.frame);
    let s = assert_integer_scaled(&img, &info, &frame);
    let sf = info.scale_factor.round() as u32;
    assert!(sf >= 1 && info.scale_factor == sf as f64, "{info:?}");
    s
}

#[test]
fn at_1x_2x_and_3x_the_window_is_that_multiple_and_the_picture_fills_it_unsmeared() {
    for n in 1..=3u32 {
        let mut app = App::launch(LaunchOpts {
            max_frames: SHOT + 5,
            scale: Some(n),
            ..Default::default()
        });
        let c = app.run_script(&format!("{SHOT}: shot\n"));
        assert!(c.exit.success(), "{n}×: {}", c.stderr);
        let (_, info) = c.read_shot(SHOT);
        let sf = info.scale_factor as u32;
        assert_eq!(info.surface, (320 * n * sf, 256 * n * sf), "{n}× window: {info:?}");
        assert_eq!(shot(&c, SHOT), n * sf, "{n}×");
    }
}

#[test]
fn a_resized_window_keeps_a_whole_scale_centred_and_stops_at_1x() {
    // Auto: the largest multiple that fits, as the window is dragged larger, then too small.
    let mut app = App::launch(LaunchOpts {
        max_frames: SHOT + 40,
        scale: None,
        ..Default::default()
    });
    let c = app.run_script(&format!(
        "5: resize 1000 700\n{SHOT}: shot\n{}: resize 100 100\n{}: shot\n",
        SHOT + 5,
        SHOT + 30
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    let (_, big) = c.read_shot(SHOT);
    let sf = big.scale_factor as u32;
    assert_eq!(big.surface, (1000 * sf, 700 * sf), "{big:?}");
    let fit = (1000 * sf / 320).min(700 * sf / 256);
    assert_eq!(shot(&c, SHOT), fit, "Auto fills the window by whole multiples");
    let (_, small) = c.read_shot(SHOT + 30);
    assert_eq!(small.surface, (320 * sf, 256 * sf), "the window stops at its 1× minimum");
    assert_eq!(shot(&c, SHOT + 30), sf, "and shows the whole picture at 1×");

    // A fixed 2× stays 2× in a larger window, centred with black around it.
    let mut app = App::launch(LaunchOpts {
        max_frames: SHOT + 5,
        scale: Some(2),
        ..Default::default()
    });
    let c = app.run_script(&format!("5: resize 1000 700\n{SHOT}: shot\n"));
    assert!(c.exit.success(), "{}", c.stderr);
    let (_, info) = c.read_shot(SHOT);
    let sf = info.scale_factor as u32;
    assert_eq!(info.surface, (1000 * sf, 700 * sf));
    assert_eq!(shot(&c, SHOT), 2 * sf);
}

/// ⌃⌘F, as the keys arrive.
const FULLSCREEN_KEYS: &str = "down ControlLeft\n{n}: down SuperLeft\n{n}: press KeyF\n{n}: up SuperLeft\n{n}: up ControlLeft\n";

fn fullscreen_at(n: u64) -> String {
    format!("{n}: {}", FULLSCREEN_KEYS.replace("{n}", &n.to_string()))
}

#[test]
fn a_scripted_run_cannot_go_fullscreen_without_allow_fullscreen() {
    let mut app = App::launch(LaunchOpts {
        max_frames: SHOT + 5,
        ..Default::default()
    });
    let c = app.run_script(&format!("{}{SHOT}: shot\n", fullscreen_at(20)));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("fullscreen refused"), "{}", c.stderr);
    let (_, info) = c.read_shot(SHOT);
    let sf = info.scale_factor as u32;
    assert_eq!(info.surface, (320 * sf, 256 * sf), "still the 1× window");
    // Ctrl is fire, so step 20 saw fire; the F pressed with ⌘ never reached the game.
    assert_eq!(c.inputs, "20 10 \n", "{}", c.inputs);
    let settings = std::fs::read_to_string(app.home.join("settings.toml")).unwrap_or_default();
    assert!(!settings.contains("fullscreen = true"), "{settings}");
}

#[test]
#[ignore = "takes over the display: SF_E2E_OS=1 cargo test -p sf-e2e --test window -- --ignored"]
fn fullscreen_on_and_off_by_its_shortcut_shows_the_same_frames() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "the fullscreen run takes over the display; run it with SF_E2E_OS=1"
    );
    // On at 20, a shot once the change has settled, off at 200, a shot at 290 (≤ 300 frames).
    let script = format!("{}{SHOT}: shot\n{}290: shot\n", fullscreen_at(20), fullscreen_at(200));
    let opts = LaunchOpts {
        max_frames: 300,
        timeout_s: 60,
        extra_args: vec!["--allow-fullscreen".into()],
        ..Default::default()
    };
    let mut app = App::launch(opts.clone());
    let c = app.run_script(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    let (_, full) = c.read_shot(SHOT);
    let sf = full.scale_factor as u32;
    assert!(
        full.surface.0 > 320 * sf * 2 && full.surface.1 > 256 * sf * 2,
        "fullscreen fills the display: {full:?}"
    );
    let s = shot(&c, SHOT);
    assert_eq!(s, (full.surface.0 / 320).min(full.surface.1 / 256), "the largest whole scale");
    let (_, back) = c.read_shot(290);
    assert_eq!(back.surface, (320 * sf, 256 * sf), "back to the 1× window");
    shot(&c, 290);

    // The frames are the windowed run's.
    let mut windowed = App::launch(LaunchOpts {
        extra_args: vec![],
        ..opts
    });
    let w = windowed.run_script(&script);
    assert!(w.exit.success(), "{}", w.stderr);
    assert!(w.stderr.contains("fullscreen refused"));
    assert_eq!(c.frames.len(), w.frames.len());
    assert!(c.frames == w.frames, "fullscreen changed a frame");
    assert_eq!(c.inputs, w.inputs);
}
