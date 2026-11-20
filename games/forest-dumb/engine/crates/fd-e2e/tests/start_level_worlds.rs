//! Title plan T09 (DESIGN §2.5): Game ▸ Start Level… and `--level N [--cheat]` start worlds 1 to 6
//! on the translated program, the cheat as chosen, the one way `fd-conform bug replay` starts them:
//! from power-on through the title on `to_level_1_input`, `world_index` and `cheat_flag` written as
//! `load_world` is entered. The app's state is held to a headless run of the conformance runner's
//! translated program started that way, every step; a recording from a world start replays to the
//! app's last state; and the real window (OS layer, `FD_E2E_OS=1`) offers six levels and the cheat
//! box, unclipped, its Cancel leaving play as it was.
//!
//! Needs the owner's disks and the generated tree (`translate.py --build`); fails naming the command
//! without it. The OS-layer test: `FD_E2E_OS=1 cargo test --release -p fd-e2e --test
//! start_level_worlds -- --ignored --test-threads 1`.

mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_core::layout_var;
use fd_core::recording::{LevelId, StartPoint};
use fd_e2e::{App, Captured, Key, LaunchOpts};
use fd_translated::TranslatedGame;

/// Chip RAM's size, where slow RAM starts in the engine's memory.
const CHIP: usize = 512 << 10;
/// Where slow RAM starts in the A500's address space.
const SLOW_BASE: u32 = 0xC0_0000;

fn require_tree() {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
}

/// The conformance runner's translated program from power-on, level 1 mounted, run headless to
/// `level`'s first frame as `fd-app`'s `Translated::to_world` runs it: the writes at `load_world`
/// for a world start (`fd_app::world_start`), none for level 1's old start. The frames it took.
fn headless_world(level: LevelId, cheat: bool) -> (TranslatedEngine, u64) {
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut e = TranslatedEngine::power_on(&disks).unwrap_or_else(|e| panic!("{e}"));
    e.mount_level_1(common::data());
    let start = fd_app::world_start(level, cheat);
    if let StartPoint::World { level, cheat } = start {
        for (addr, bytes) in fd_app::world_start_bytes(level, cheat) {
            FrameEngine::write_at(&mut e, fd_app::LOAD_WORLD, addr, &bytes).unwrap();
        }
    }
    for n in 0..fd_app::TO_LEVEL_1_FRAMES {
        e.frame(fd_app::to_level_1_input(n));
        assert_eq!(e.stopped(), None, "headless stopped on its way to {level}, frame {n}");
        let m = e.ram().unwrap();
        if fd_app::start_reached(start, &m[..CHIP], &m[CHIP..]) {
            return (e, n + 1);
        }
    }
    panic!("headless did not reach {level}");
}

/// The state hash the app writes, of the headless run's last frame.
fn state(e: &TranslatedEngine) -> String {
    let m = e.ram().expect("a frame has run");
    format!("{:016x}", fd_app::memory_hash(&m[..CHIP], &m[CHIP..]))
}

/// A variable of `LAYOUT` in the headless run's RAM.
fn var(e: &TranslatedEngine, name: &str) -> u32 {
    let ram = e.ram().expect("a frame has run");
    let v = layout_var(name).unwrap();
    let a = fd_conform::HUNK_BASES[v.hunk as usize] + v.offset;
    let i = match a.checked_sub(SLOW_BASE) {
        Some(s) => s as usize + CHIP,
        None => a as usize,
    };
    ram[i..i + v.width.bytes()]
        .iter()
        .fold(0, |acc, &b| acc << 8 | b as u32)
}

/// The level the headless run plays.
fn level_of(e: &TranslatedEngine) -> Option<LevelId> {
    let m = e.ram().unwrap();
    fd_app::level_in_ram(&m[..CHIP], &m[CHIP..])
}

/// Step `e` on the app's input for steps `from..=to`, its state equal to the app's at each.
fn follow(e: &mut TranslatedEngine, c: &Captured, from: u64, to: u64) {
    let fed = common::inputs(c);
    let hashes = common::hashes(c);
    assert_eq!(hashes.len() as u64, to + 1, "a hash a frame");
    for n in from..=to {
        e.frame(fed.get(&n).copied().unwrap_or_default());
        assert_eq!(e.stopped(), None, "the headless run stopped at step {n}");
        assert_eq!(state(e), hashes[n as usize], "state at step {n}");
    }
}

/// Play's keys after a start at step 1: right held, a jump, then left.
const PLAY: &str = "20: down ArrowRight\n80: down Space\n95: up Space\n150: up ArrowRight\n\
                    160: down ArrowLeft\n220: up ArrowLeft\n";

fn launch(max_frames: u64, args: &[&str]) -> App {
    App::launch_translated(LaunchOpts {
        max_frames,
        timeout_s: 180,
        extra_args: args.iter().map(|a| a.to_string()).collect(),
        ..Default::default()
    })
}

#[test]
fn start_level_world_3_with_the_cheat_plays_as_headless_every_step() {
    require_tree();
    const END: u64 = 240;
    let mut app = launch(END, &["--capture-hashes"]);
    let c = app.run_script_unread(&format!(
        "1: menu start-level\nform level W3-1\nform cheat on\nform record off\nform start\n{PLAY}"
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.prompts.contains("start-level: -> W3-1 cheat\n"), "{}", c.prompts);
    assert!(
        c.stderr.contains("fd-app: W3-1 reached headless after ")
            && c.stderr.contains("fd-app: started W3-1 after step 0, cheat on\n"),
        "{}",
        c.stderr
    );
    let (mut e, _) = headless_world(LevelId::W3_1, true);
    assert_eq!(level_of(&e), Some(LevelId::W3_1));
    assert_eq!(var(&e, "world_index"), 2);
    assert_eq!(var(&e, "cheat_flag"), 1, "the cheat set");
    follow(&mut e, &c, 1, END);
    assert_eq!(level_of(&e), Some(LevelId::W3_1), "still in world 3's play");
}

/// `--level 4` and `--level 4 --cheat`: the window opens on world 4's first frame, equal to
/// headless; then a few steps of play equal too.
#[test]
fn level_4_opens_on_world_4_s_first_frame_with_and_without_the_cheat() {
    require_tree();
    const END: u64 = 60;
    for cheat in [false, true] {
        let mut args = vec!["--level", "4", "--capture-hashes"];
        if cheat {
            args.push("--cheat");
        }
        let mut app = launch(END, &args);
        let c = app.run_script_unread("10: down ArrowRight\n50: up ArrowRight\n");
        assert!(c.exit.success(), "{}", c.stderr);
        let cheat_on = if cheat { ", cheat on" } else { "" };
        assert!(
            c.stderr
                .contains(&format!("fd-app: playing the translated program from W4-1{cheat_on}\n")),
            "{}",
            c.stderr
        );
        let (mut e, frames) = headless_world(LevelId::W4_1, cheat);
        assert!(
            c.stderr.contains(&format!(
                "fd-app: W4-1 reached headless after {frames} frames from power-on{cheat_on}\n"
            )),
            "{}",
            c.stderr
        );
        assert_eq!(level_of(&e), Some(LevelId::W4_1));
        assert_eq!(var(&e, "world_index"), 3);
        if cheat {
            assert_eq!(var(&e, "cheat_flag"), 1, "the cheat set");
        }
        assert_eq!(state(&e), common::hashes(&c)[0], "world 4's first frame, cheat {cheat}");
        follow(&mut e, &c, 1, END);
    }
}

/// `--level N --cheat` for every world: the app's first frame is headless `to_world(N, cheat)`'s,
/// in world N's play with `world_index` N-1 and `cheat_flag` set.
#[test]
fn every_world_1_to_6_starts_with_the_cheat_at_its_first_frame_as_headless() {
    require_tree();
    for (k, level) in LevelId::ALL.into_iter().enumerate() {
        let n = (k + 1).to_string();
        let mut app = launch(1, &["--level", &n, "--cheat", "--capture-hashes"]);
        let c = app.run_script_unread("");
        assert!(c.exit.success(), "{level}: {}", c.stderr);
        let (e, _) = headless_world(level, true);
        assert_eq!(level_of(&e), Some(level));
        assert_eq!(var(&e, "world_index"), k as u32, "{level}");
        assert_eq!(var(&e, "cheat_flag"), 1, "{level}: the cheat set");
        assert_eq!(state(&e), common::hashes(&c)[0], "{level}'s first frame");
    }
}

/// Start Level… W1-1, the cheat off, is `--level 1`'s start: level 1's old start, no write at
/// `load_world`, played on level 1's hand engine through the seam, as headless with it mounted.
#[test]
fn start_level_w1_1_is_level_1_through_the_seam_as_level_1_starts_it() {
    require_tree();
    const END: u64 = 240;
    let mut app = launch(END, &["--capture-hashes"]);
    let c = app.run_script_unread(&format!(
        "1: menu start-level\nform level W1-1\nform record off\nform start\n{PLAY}"
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: level 1 reached headless after "), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: started W1-1 after step 0\n"), "{}", c.stderr);
    let (mut e, _) = headless_world(LevelId::W1_1, false);
    follow(&mut e, &c, 1, END);
    assert_eq!(level_of(&e), Some(LevelId::W1_1));
}

/// Cancel leaves play as it was: the state runs on from power-on, as headless with no restart.
#[test]
fn start_level_cancelled_leaves_the_title_running_as_it_was() {
    require_tree();
    const END: u64 = 120;
    let mut app = launch(END, &["--capture-hashes"]);
    let c = app.run_script_unread("50: menu start-level\nform level W5-1\nform cheat on\nform cancel\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.prompts.contains("start-level: -> cancel"), "{}", c.prompts);
    assert!(!c.stderr.contains("fd-app: started"), "{}", c.stderr);
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut e = TranslatedEngine::power_on(&disks).unwrap();
    e.mount_level_1(common::data());
    follow(&mut e, &c, 1, END);
}

/// The recorder on from a world 5 start: a few hundred steps, Stop Recording, the zip names a
/// world 5 start, and `fd-conform bug replay` reaches the app's last state and its screenshot.
#[test]
fn a_recording_from_a_world_5_start_replays_to_the_app_s_last_state() {
    require_tree();
    const LAST: u64 = 300;
    let mut app = launch(LAST + 10, &["--capture-hashes"]);
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    let c = app.run_script_unread(&format!(
        "note world five\n1: menu start-level\nform level W5-1\nform start\n{PLAY}\
         240: down ArrowRight\n290: up ArrowRight\n{}: menu stop-recording\n",
        LAST + 1
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.prompts.contains("start-level: -> W5-1 record\n"), "{}", c.prompts);
    let zips: Vec<PathBuf> = std::fs::read_dir(&bugs).unwrap().flatten().map(|e| e.path()).collect();
    assert_eq!(zips.len(), 1, "{}", c.prompts);
    let name = zips[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("bug-W5-1-"), "{name}");
    let (rec, _) =
        amiga_record::read_zip::<fd_core::recording::ForestDumb>(&std::fs::read(&zips[0]).unwrap())
            .unwrap();
    assert_eq!(rec.start, StartPoint::World { level: LevelId::W5_1, cheat: false });
    assert_eq!(rec.inputs.len() as u64, LAST, "steps 1 to {LAST}");

    // The library's replay: its RAM is the app's at the last recorded step.
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let r = fd_conform::bug::replay_translated(&disks, common::data(), &rec)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.frames as u64, LAST);
    let hash = format!("{:016x}", fd_app::memory_hash(&r.ram[..CHIP], &r.ram[CHIP..]));
    assert_eq!(hash, common::hashes(&c)[LAST as usize], "the app's state at step {LAST}");

    // And the real command, as the owner runs it.
    let out = std::process::Command::new(fd_e2e::conform_binary())
        .args(["bug", "replay"])
        .arg(&zips[0])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains(&format!("frames {LAST}\n")), "{stdout}");
    assert!(stdout.contains("screenshot equal\n"), "{stdout}");
    let place = |key: &str| {
        stdout
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .unwrap_or_else(|| panic!("no {key}: {stdout}"))
            .to_string()
    };
    assert_eq!(place("stopped-at "), place("replay-at "));
    assert!(place("replay-at ").starts_with("W5-1:"), "{stdout}");
}

// The OS layer: the real Start Level window.

fn os_layer() {
    assert!(
        std::env::var_os("FD_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests bring up real windows and click them; run them with FD_E2E_OS=1"
    );
    fd_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
}

fn until(what: &str, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(30), "{what}: not within 30 s");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn start_level_open(app: &App) -> bool {
    let b = app.os_buttons();
    b.iter().any(|t| t == "Start") && b.iter().any(|t| t == "Cancel")
}

/// Size the game window to `w`×`h` points through System Events, as a drag of its corner would.
fn resize(app: &App, w: u32, h: u32) {
    let pid = app.pid().expect("running");
    let script = format!(
        "const p = Application('System Events').processes.whose({{unixId: {pid}}})[0]; \
         p.windows[0].size = [{w}, {h}];"
    );
    let out = std::process::Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .expect("osascript");
    assert!(out.status.success(), "resizing: {}", String::from_utf8_lossy(&out.stderr));
}

/// Whether `inner` lies inside `outer`, both `[x, y, w, h]` in points.
fn inside(inner: [f64; 4], outer: [i32; 4]) -> bool {
    let o = outer.map(|v| v as f64);
    inner[2] > 0.0
        && inner[3] > 0.0
        && inner[0] >= o[0]
        && inner[1] >= o[1]
        && inner[0] + inner[2] <= o[0] + o[2]
        && inner[1] + inner[3] <= o[1] + o[3]
}

/// At the default scale, 400×300 and 1200×900: the window lists W1-1 to W6-1, the cheat box
/// unticked and titled whole, every control inside the window on screen; Cancel leaves the title
/// playing with no start.
#[test]
#[ignore = "OS layer"]
fn os_layer_start_level_shows_six_levels_and_the_cheat_box_unclipped_and_cancel_keeps_play() {
    os_layer();
    require_tree();
    let shots = std::env::temp_dir().join("fd-start-level-worlds");
    std::fs::create_dir_all(&shots).unwrap();
    for size in [None, Some((400, 300)), Some((1200, 900))] {
        let mut app = App::launch_translated(LaunchOpts {
            max_frames: 30_000,
            timeout_s: 240,
            scale: None,
            ..Default::default()
        });
        app.start_os();
        app.os_window();
        std::thread::sleep(Duration::from_millis(500));
        if let Some((w, h)) = size {
            resize(&app, w, h);
            until("the size", || {
                let b = app.os_window().1;
                (b[2] - w as i32).abs() <= 1 && b[3] >= h as i32 - 1
            });
        }
        app.os_menu(&["Game", "Start Level…"]);
        until("the window", || start_level_open(&app));
        let windows = app.os_windows();
        assert_eq!(windows.len(), 2, "{size:?}: the game and Start Level windows");
        let (id, panel) = windows[0];
        let path = shots.join(format!("start-level-{}.png", size.map_or("default".into(), |(w, h)| format!("{w}x{h}"))));
        let ok = std::process::Command::new("screencapture")
            .args(["-x", "-o", &format!("-l{id}")])
            .arg(&path)
            .status()
            .unwrap();
        assert!(ok.success(), "capturing the window");
        eprintln!("captured {} {panel:?}", path.display());
        assert!(panel[0] >= 0 && panel[1] >= 0, "{size:?}: the window off screen: {panel:?}");
        for c in ["level", "cheat", "record", "start", "cancel"] {
            let f = app.os_frame_id(c);
            assert!(inside(f, panel), "{size:?}: {c} at {f:?} outside the window {panel:?}");
        }
        assert_eq!(app.os_text_id("cheat"), "Cheat on");
        assert!(!app.os_checked_id("cheat"), "the cheat is off when the window opens");
        assert!(app.os_checked_id("record"), "the recorder is on");
        assert_eq!(
            app.os_popup_titles("level"),
            ["W1-1", "W2-1", "W3-1", "W4-1", "W5-1", "W6-1"]
        );
        app.os_choose_id("level", "W6-1");
        app.os_press_id("cheat");
        app.os_press_button("Cancel");
        until("closed by Cancel", || !start_level_open(&app));
        std::thread::sleep(Duration::from_millis(500));
        app.os_activate();
        app.os_key(Key::Escape);
        let c = app.finish_unread();
        assert!(c.exit.success(), "{}", c.stderr);
        assert!(!c.stderr.contains("fd-app: started"), "{size:?}: Cancel started nothing: {}", c.stderr);
        assert!(!c.stderr.contains("reached headless"), "{size:?}: {}", c.stderr);
        assert!(
            c.stderr.contains("fd-app: playing the translated program from power-on"),
            "{}",
            c.stderr
        );
    }
}
