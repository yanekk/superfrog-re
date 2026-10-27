//! real-widescreen T10 (DESIGN §2.1, §2.7, §2.8, §2.9): View ▸ Widescreen in the real app plays
//! the widescreen game, not only its picture. Each run captures every frame's picture and state
//! hashes (`--capture-hashes`, `sf_core::feed`) and is held to the engine headless with the same
//! mode, inputs and switches: from launch with the setting on, switched twice mid-level while
//! recording, the recording replayed by `sf-conform bug replay` (`sf_conform::bug::replay`) and by
//! the app's `--replay` with the setting off, and at W1-1's two ends.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::cache_data;
use sf_core::feed::{frame_hashes, parse_hashes};
use sf_core::input::{FrameInput, Joy};
use sf_core::level::camera::LEVEL_WIDTH;
use sf_core::frog::{FROG, f};
use sf_core::recording::LevelSetup;
use sf_core::{DisplayData, Game, Picture, Progress, Rng, Start, View};
use sf_e2e::{App, Captured, LaunchOpts};

const N: u32 = 65;
const WIDE: View = View {
    extra_columns_each_side: N as u16,
};

/// `g` played and drawn in widescreen `on`, as the app's `window::show` sets it.
fn show(g: &mut Game, on: bool) {
    g.set_widescreen(on);
    g.set_view(if on { WIDE } else { View::default() });
}

/// The app in a temporary home capturing hashes, `extra` more arguments.
fn launch(max_frames: u64, timeout_s: u64, extra: &[String]) -> App {
    let mut extra_args = vec!["--capture-hashes".to_string()];
    extra_args.extend_from_slice(extra);
    App::launch(LaunchOpts {
        max_frames,
        timeout_s,
        scale: Some(2),
        extra_args,
        ..Default::default()
    })
}

/// Every frame's `(picture, state)` hash the run wrote, frame 0 first.
fn hashes(c: &Captured) -> Vec<(u64, u64)> {
    parse_hashes(&std::fs::read_to_string(c.dir.join("hashes.txt")).unwrap()).unwrap()
}

/// The first frame from `from` at which the app's hashes are not `want`'s (`want[k]` is app frame
/// `from + k`), or the frame the app's run ends short at.
fn first_mismatch(app: &[(u64, u64)], from: usize, want: &[(u64, u64)]) -> Option<String> {
    for (k, w) in want.iter().enumerate() {
        let n = from + k;
        match app.get(n) {
            None => return Some(format!("the app's run ends before frame {n}")),
            Some(a) if a.0 != w.0 => return Some(format!("frame {n}: the picture differs")),
            Some(a) if a.1 != w.1 => return Some(format!("frame {n}: the state differs")),
            _ => {}
        }
    }
    None
}

/// Picture column `x` is colour index 0 over the play field's lower half: what the renderer draws
/// past the map's ends (sf-core `widescreen_camera.rs`).
fn column_0(p: &Picture, x: usize) -> bool {
    (p.height as usize / 2..p.height as usize).all(|y| p.pixels[y * p.width as usize + x] == 0)
}

/// Picture columns `xs` are all colour index 0 below the panel.
fn strip_0(p: &Picture, xs: std::ops::Range<usize>) -> bool {
    (40..p.height as usize).all(|y| p.pixels[y * p.width as usize..][xs.clone()].iter().all(|&i| i == 0))
}

/// W3-1 by Continue at step 10 (progress World 3): its loading picture and map until about step
/// 1 664, then the right arrow 1 700..1 900 scrolls the camera mid-level.
const W3_SCRIPT: &str = "10: menu continue\n1700: down ArrowRight\n1900: up ArrowRight\n";
const W3_FRAMES: u64 = 2000;

fn w3_input(n: u64) -> FrameInput {
    if (1700..1900).contains(&n) { FrameInput::joy(Joy::RIGHT) } else { FrameInput::NONE }
}

#[test]
fn widescreen_on_from_launch_plays_the_widescreen_game_frame_for_frame() {
    let mut app = launch(W3_FRAMES, 300, &[]);
    std::fs::write(app.home.join("progress.toml"), "world = 3\nlevel = 1\n").unwrap();
    std::fs::write(app.home.join("settings.toml"), "widescreen = true\n").unwrap();
    let c = app.run_script_unread(W3_SCRIPT);
    assert!(c.exit.success(), "{}", c.stderr);
    let data = cache_data(&app.home);
    let display = Arc::new(DisplayData::new(&data));
    let start = |wide: bool| {
        let mut g = Game::start_at_with(display.clone(), &Progress { world: 3, level: 1 }, Rng::seeded(0, 0));
        show(&mut g, wide);
        g
    };
    let (mut wide, mut narrow) = (start(true), start(false));
    let mut want = Vec::new();
    let (mut differs, mut level, mut strips) = (false, 0, 0);
    for n in 10..=W3_FRAMES {
        let pw = wide.step(w3_input(n)).picture;
        narrow.step(w3_input(n));
        want.push(frame_hashes(&wide, &pw));
        differs |= wide.probe() != narrow.probe();
        if wide.scene().is_level() {
            level += 1;
            // Mid-level, the strips show the level, never colour 0.
            if wide.state().camera().x > 2 * N {
                strips += 1;
                let w = pw.width as usize;
                assert!(!strip_0(&pw, 0..N as usize) && !strip_0(&pw, w - N as usize..w), "frame {n}: a strip is empty");
            }
        }
    }
    assert_eq!(first_mismatch(&hashes(&c), 10, &want), None, "the app is the headless widescreen game");
    assert!(level > 300 && strips > 0, "{level} level frames, {strips} scrolled");
    assert!(differs, "W3-1 in widescreen never differed from the 4:3 game: the mode did not reach it");
}

/// W3-1 by Game ▸ Start Level… after step 4 (the dialog's defaults otherwise, seed 0 0), the
/// recorder on: its fade-in over by step 81 of the level, the right arrow held from its step 100
/// for 200 steps (the camera scrolls), widescreen switched on at its step 160, mid-scroll, and off
/// again at 700, and the recording stopped at 1 000.
const START: u64 = 4;
const WALK: std::ops::Range<u64> = 100..300;
const SWITCHES: [u64; 2] = [START + 160, START + 700];
const STOP: u64 = START + 1000;

fn w3_1_setup() -> LevelSetup {
    LevelSetup {
        level: "W3-1".parse().unwrap(),
        ..LevelSetup::default()
    }
}

fn walk_input(k: u64) -> FrameInput {
    if WALK.contains(&k) { FrameInput::joy(Joy::RIGHT) } else { FrameInput::NONE }
}

fn zips(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    v.retain(|p| p.extension().is_some_and(|x| x == "zip"));
    v
}

#[test]
fn two_switches_while_recording_play_replay_and_replay_in_the_app_as_played() {
    let bugs = std::env::temp_dir().join(format!("sf-e2e-wide-bugs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bugs);
    let bugs_arg = ["--bugs-dir".to_string(), bugs.display().to_string()];
    let mut app = launch(STOP + 20, 600, &bugs_arg);
    let script = format!(
        "note w\n{}: start-level W3-1 record\n{}: down ArrowRight\n{}: up ArrowRight\n{}: menu widescreen\n\
         {}: menu widescreen\n{STOP}: menu stop-recording\n",
        START + 1,
        START + WALK.start,
        START + WALK.end,
        SWITCHES[0],
        SWITCHES[1],
    );
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains(&format!("superfrog: started W3-1 after step {START}, recording\n")), "{}", c.stderr);

    // Played: the headless game with the switches before the same steps (headless step k is app
    // step START + k).
    let data = cache_data(&app.home);
    let display = Arc::new(DisplayData::new(&data));
    let mut g = Game::start_level(display.clone(), &w3_1_setup(), Rng::seeded(0, 0));
    let mut want = Vec::new();
    let (mut wide_level, mut wide_moving) = (0, 0);
    for k in 1..STOP - START {
        let n = START + k;
        if SWITCHES.contains(&n) {
            assert!(g.scene().is_level(), "step {n}: the switch is mid-level");
            let on = !g.widescreen();
            show(&mut g, on);
        }
        let cx = g.state().camera().x;
        let p = g.step(walk_input(k)).picture;
        want.push(frame_hashes(&g, &p));
        wide_level += (g.widescreen() && g.scene().is_level()) as u32;
        wide_moving += (g.widescreen() && g.state().camera().x != cx) as u32;
    }
    assert_eq!(first_mismatch(&hashes(&c), START as usize + 1, &want), None, "the app is the headless game with its switches");
    assert!(g.scene().is_level(), "still in W3-1 at the stop");
    assert!(wide_level == 540 && wide_moving > 20, "{wide_level} widescreen level frames, {wide_moving} scrolling");

    // The zip: off at the start, a toggle before each switched step, and `bug replay` plays it to
    // its screenshot, frame for frame the app's.
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{}", c.prompts);
    let (rec, png) = sf_conform::bug::read_zip(&std::fs::read(&found[0]).unwrap()).unwrap();
    assert!(!rec.widescreen);
    assert_eq!(rec.toggles, SWITCHES.map(|s| (s - START - 1) as u32));
    assert_eq!(rec.inputs.len() as u64, STOP - START - 1);
    let mut replayed = Vec::new();
    let r = sf_conform::bug::replay(&rec, |_, game, pic| replayed.push(frame_hashes(game, pic)));
    assert_eq!(replayed, want, "`bug replay` plays the game the app played");
    assert_eq!(sf_conform::bug::screenshot_differs(&png, &r.last), Ok(0), "screenshot equal");

    // `--replay` with the setting off: it plays and draws the recording's modes, a menu Widescreen
    // is refused, and the setting stays as it was.
    std::fs::write(app.home.join("settings.toml"), "widescreen = false\n").unwrap();
    let settings = std::fs::read(app.home.join("settings.toml")).unwrap();
    let frames = rec.inputs.len() as u64;
    app.opts.max_frames = frames + 5;
    app.opts.extra_args = vec!["--capture-hashes".into(), "--replay".into(), found[0].display().to_string()];
    let c = app.run_script_unread(&format!("{}: menu widescreen\n", SWITCHES[0] + 100));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: widescreen is disabled during a replay"), "{}", c.stderr);
    for (s, on) in SWITCHES.iter().zip(["on", "off"]) {
        let line = format!("superfrog: replay widescreen {on} before step {}\n", s - START);
        assert!(c.stderr.contains(&line), "{line}{}", c.stderr);
    }
    assert_eq!(first_mismatch(&hashes(&c), 1, &replayed), None, "the app's --replay is `bug replay`'s");
    assert_eq!(std::fs::read(app.home.join("settings.toml")).unwrap(), settings, "the setting is untouched");
    let _ = std::fs::remove_dir_all(&bugs);
}

/// This build's name, from the app's refusal of a state from another (as `widescreen_frog.rs`).
fn our_build(app: &mut App, data: &sf_core::GameData) -> String {
    let other = Game::new(data, Start::Level("W1-1".parse().unwrap()));
    std::fs::create_dir_all(app.home.join("states")).unwrap();
    std::fs::write(app.home.join("states/slot-5.sfstate"), other.save("not this build", &sf_assets::cache_hash(data))).unwrap();
    let c = app.run_script("2: menu load state 5\n");
    std::fs::remove_file(app.home.join("states/slot-5.sfstate")).unwrap();
    let line = c.prompts.lines().next().unwrap_or_else(|| panic!("no refusal: {}", c.stderr));
    let ours = line.split("; this is ").nth(1).and_then(|s| s.strip_suffix(". It was not loaded."));
    ours.unwrap_or_else(|| panic!("{line}")).to_string()
}

fn word(g: &Game, a: u32) -> u16 {
    let b = g.peek(a, 2);
    u16::from_be_bytes([b[0], b[1]])
}

#[test]
fn at_w1_1_s_left_end_and_right_end_the_strips_show_no_colour_0_from_off_the_map() {
    // The left end: W1-1 by Start Level with the setting on, played as the headless widescreen
    // game; the camera at widescreen's clamp, so the left strip begins at level x 16.
    const FRAMES: u64 = 1000;
    let mut app = launch(FRAMES, 300, &[]);
    std::fs::write(app.home.join("settings.toml"), "widescreen = true\n").unwrap();
    let c = app.run_script_unread(&format!("{}: menu start-level\nform record off\nform start\n", START + 1));
    assert!(c.exit.success(), "{}", c.stderr);
    let data = cache_data(&app.home);
    let display = Arc::new(DisplayData::new(&data));
    let mut g = Game::start_level(display.clone(), &LevelSetup::default(), Rng::seeded(0, 0));
    show(&mut g, true);
    let (mut want, mut level) = (Vec::new(), 0);
    for _ in START + 1..=FRAMES {
        let p = g.step(FrameInput::NONE).picture;
        want.push(frame_hashes(&g, &p));
        if g.scene().is_level() {
            level += 1;
            assert_eq!(g.state().camera().x as u32, 16 + N, "the camera at widescreen's left clamp");
            assert!(!column_0(&p, 0), "level frame {level}: the left strip's first column is colour 0");
        }
    }
    assert_eq!(first_mismatch(&hashes(&c), START as usize + 1, &want), None);
    assert!(level > 100, "{level} level frames");

    // The right end: W1-1 played headless in widescreen with the frog put near its end, saved as a
    // state, loaded into the app with the setting on and walked on into the right strip.
    let build = our_build(&mut app, &data);
    let mut end = Game::new(&data, Start::Level("W1-1".parse().unwrap()));
    end.set_widescreen(true);
    for _ in 0..3 {
        end.step(FrameInput::NONE);
    }
    let x = word(&end, LEVEL_WIDTH) - 0x200;
    end.poke(FROG + f::X, &x.to_be_bytes());
    for _ in 0..600 {
        end.step(FrameInput::joy(Joy::RIGHT));
    }
    let bytes = end.save(&build, &sf_assets::cache_hash(&data));
    std::fs::write(app.home.join("states/slot-1.sfstate"), &bytes).unwrap();
    const LOAD: u64 = 5;
    const WALK: u64 = 300;
    app.opts.max_frames = LOAD + WALK;
    let c = app.run_script_unread(&format!("{LOAD}: menu load-state-1\n{}: down ArrowRight\n", LOAD + 1));
    assert!(c.exit.success(), "{}", c.stderr);
    let mut g = Game::load(&data, &bytes, &build, &sf_assets::cache_hash(&data)).unwrap();
    show(&mut g, true);
    let mut want = Vec::new();
    for k in 1..=WALK {
        let p = g.step(FrameInput::joy(if k == 1 { Joy::NONE } else { Joy::RIGHT })).picture;
        want.push(frame_hashes(&g, &p));
        assert!(g.scene().is_level(), "step {k} after the load");
        assert!(!column_0(&p, p.width as usize - 1), "step {k}: the right strip's last column is colour 0");
    }
    assert_eq!(first_mismatch(&hashes(&c), LOAD as usize, &want), None, "the loaded game plays as headless");
    let cx = g.state().camera().x as u32;
    assert_eq!(cx, word(&g, LEVEL_WIDTH) as u32 - 0x168 - N, "the camera at widescreen's right clamp");
    assert!(word(&g, FROG + f::X) as u32 - cx > 334, "the frog in the right strip");
}
