//! Title plan T11, the title's drill (DESIGN §2.2, §2.5, §2.6; pir-e2e § 3): what a person does on
//! the title and with Start Level… that the path tests (`title_paths.rs`, `title_paths_short.rs`,
//! `start_level_worlds.rs`, `translated_drill.rs`) did not drive. The title at the default scale,
//! 400×300 and 1200×900; a key held across a row change and digits rolled; focus lost with a digit
//! held on KODY and the window resized mid-entry; Start Level… opened on the title and in play, with
//! the recorder asking; game over back to the title and START again. The app's state is a headless
//! run's every step (`tests/title/mod.rs`), every shot shows its frame whole and centred, and the
//! screens are copied to `games/forest-dumb/build/drill/title/` for the drill's reading.
//!
//! Full: the owner's disks and the generated tree (`translate.py --build`); fails naming the command
//! without it. What the drill saw is in the plan's FINDINGS (2026-11-20), worker-driven.

mod common;
mod title;

use std::path::PathBuf;

use amiga_input::{FrameInput, Joy, KeySet};
use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_core::recording::{LevelId, StartPoint};
use fd_e2e::{App, Captured, LaunchOpts};
use title::{InApp, Run, TITLE, digit, key};

/// The translated picture, as the app shows it (`fd_app::CANVAS`).
const W: u32 = fd_app::CANVAS.2 as u32;
const H: u32 = fd_app::CANVAS.3 as u32;
/// The recording bar's height in points (`amiga_app::recording_bar::BAR_HEIGHT`; fd-e2e does not
/// depend on `amiga-app`).
const BAR_HEIGHT: u32 = 34;
/// Chip RAM's size, where slow RAM starts in the engine's memory.
const CHIP: usize = 512 << 10;

/// The sizes the drill drives: the app's default scale, then the window resized at step 5.
const SIZES: [(&str, Option<(u32, u32)>); 3] = [
    ("default", None),
    ("400x300", Some((400, 300))),
    ("1200x900", Some((1200, 900))),
];

/// The step by which step 5's resize has landed: a resize lands when the window server gets to
/// it, within a few steps alone and after 30 or more on a loaded machine (seen, T11).
const SIZED: u64 = 100;

/// Where the drill's screens go: the game's `build/`, never the repo's tracked tree.
fn screens(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../build/drill/title")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Every window shot of `c` copied to `screens(name)`, named by step and `label`.
fn save(c: &Captured, name: &str, labels: &[(u64, &str)]) {
    let to = screens(name);
    for &(n, label) in labels {
        let from = c.dir.join(format!("window-{n:05}.png"));
        std::fs::copy(&from, to.join(format!("{n:05}-{label}.png")))
            .unwrap_or_else(|e| panic!("{}: {e}", from.display()));
    }
}

/// The shot at step `n` shows the frame it names whole, centred, as captured; its window's size
/// in points. While recording the recording bar takes `BAR_HEIGHT` points below the picture.
fn shown_whole(c: &Captured, n: u64) -> (u32, u32) {
    shown_whole_over(c, n, false)
}

fn shown_whole_over(c: &Captured, n: u64, bar: bool) -> (u32, u32) {
    let (frame, img) = common::shown(c, n, W, H);
    assert!(img == c.read_frame(frame), "step {n}: the window shows frame {frame} as captured");
    let (_, shot) = c.read_shot(n);
    let (x, y, vw, vh) = shot.viewport;
    let strip = if bar {
        (BAR_HEIGHT as f64 * shot.scale_factor).round() as u32
    } else {
        0
    };
    assert!(
        (shot.surface.0 - vw) / 2 == x && (shot.surface.1 - strip - vh) / 2 == y,
        "step {n}: {shot:?} is not centred"
    );
    let p = |v: u32| (v as f64 / shot.scale_factor).round() as u32;
    (p(shot.surface.0), p(shot.surface.1))
}

/// Shots at `labels`' steps, every one whole and centred and, from step [`SIZED`] on, at `size`.
fn shots_whole(c: &Captured, labels: &[(u64, &str)], size: Option<(u32, u32)>) {
    for &(n, label) in labels {
        let points = shown_whole(c, n);
        if let Some(s) = size.filter(|_| n >= SIZED) {
            assert_eq!(points, s, "{label} at step {n}: the window's size");
        }
    }
}

/// The script lines resizing at step 5 and shooting at each label's step.
fn extra(size: Option<(u32, u32)>, labels: &[(u64, &str)]) -> String {
    let mut s = size.map_or(String::new(), |(w, h)| format!("5: resize {w} {h}\n"));
    for (n, _) in labels {
        s.push_str(&format!("{n}: shot\n"));
    }
    s
}

/// Any sample not silent in `audio.wav` from step `from` on: the title's song is heard.
fn heard_from(c: &Captured, from: u64) -> bool {
    let wav = std::fs::read(&c.audio_wav).expect("audio.wav");
    let frames = std::fs::read_to_string(c.dir.join("audio-frames.txt")).expect("audio-frames.txt");
    let mut at = 44usize;
    for l in frames.lines() {
        let mut w = l.split(' ').map(|v| v.parse::<u64>().unwrap());
        let n = w.next().unwrap();
        let count = w.sum::<u64>() as usize * 4;
        if n >= from && wav[at..at + count].iter().any(|&b| b != 0) {
            return true;
        }
        at += count;
    }
    false
}

/// The whole title at each size: the disk 2 prompt, the idle title with its song, KODY and
/// KONIEC, an entry half typed, 7531 typed, back on START, world 5's load and its play. Every shot
/// whole and centred at its size; the state headless every step.
#[test]
fn the_title_at_three_sizes_is_drawn_whole_and_plays_as_headless() {
    let mut r = Run::to_the_title();
    let (down, up) = (FrameInput::joy(Joy::DOWN), FrameInput::joy(Joy::UP));
    r.press(down, 10, 20);
    let kody = r.steps();
    r.press(down, 10, 20);
    let koniec = r.steps();
    r.press(up, 10, 20);
    r.press(FrameInput::joy(Joy::FIRE), 10, 10);
    for d in [7, 5] {
        r.press(key(digit(d)), 6, 6);
    }
    let half = r.steps();
    for d in [3, 1] {
        r.press(key(digit(d)), 6, 6);
    }
    assert_eq!(r.now("code_digits"), 0x0705_0301);
    r.press(up, 10, 10);
    let typed = r.steps();
    let play = r.start_and_play();
    assert_eq!(r.at(play, "world_index"), 4, "world 5");
    r.hold(FrameInput::NONE, 30);
    let labels = [
        (25, "prompt"),
        (TITLE, "title"),
        (kody, "kody"),
        (koniec, "koniec"),
        (half, "kody-75"),
        (typed, "start-7531"),
        (typed + 60, "loading"),
        (play + 20, "world-5"),
    ];
    for (name, size) in SIZES {
        let (_app, c) = r.in_the_app_with(InApp {
            frames: true,
            fast: false,
            scale: None,
            extra: extra(size, &labels),
            ..InApp::default()
        });
        shots_whole(&c, &labels, size);
        assert!(heard_from(&c, 200), "{name}: the title's song is heard");
        save(&c, &format!("tour-{name}"), &labels);
    }
}

/// Unasked on the rows and KODY: Down held while key 2 goes down and Down let go with it held
/// (the row moves, no digit: no entry is on); fire held into KODY with a digit still down; then
/// digits rolled, each key let go in the frame the next goes down. The original counts only the
/// releases sent last in a frame (`title-code-rolled`, the same rule): 2424 rolled gives two 4s.
#[test]
fn a_key_held_across_a_row_change_and_digits_rolled_play_as_the_original_counts_them() {
    let mut r = Run::to_the_title();
    let with = |a: FrameInput, k: u8| FrameInput {
        joy: a.joy,
        keys: KeySet(a.keys.0 | 1u128 << k),
    };
    let down = FrameInput::joy(Joy::DOWN);
    r.hold(down, 6);
    r.hold(with(down, digit(2)), 6);
    let release = r.steps() + 1;
    r.hold(key(digit(2)), 6);
    assert_eq!(r.at(release, "title_cursor"), 1, "Down let go with key 2 held: KODY");
    r.hold(FrameInput::NONE, 10);
    assert_eq!(r.now("title_selected"), 0, "no entry: the key did nothing");
    // Fire with key 4 down; fire let go first: the entry starts with the key held.
    let fire = FrameInput::joy(Joy::FIRE);
    r.hold(fire, 6);
    r.hold(with(fire, digit(4)), 4);
    r.hold(key(digit(4)), 6);
    assert_ne!(r.now("title_selected"), 0, "the entry is on");
    // Rolled: 4 up and 2 down, 2 up and 4 down, 4 up and 2 down, 2 up and 4 down, 4 up.
    for d in [2, 4, 2, 4] {
        r.hold(key(digit(d)), 6);
    }
    r.hold(FrameInput::NONE, 20);
    // Key 4's release as the entry's first; then each frame's changes in code order: 2 down
    // before 4 up leaves the release last (a 4), 2 up before 4 down leaves the press last (lost);
    // 4 let go alone last. Three 4s, the entry still on.
    assert_eq!(
        (r.now("code_digit_index"), r.now("code_digits")),
        (3, 0x0404_0406),
        "the digits rolled keys enter"
    );
    let labels = [(r.steps() - 5, "rolled")];
    let (_app, c) = r.in_the_app_with(InApp {
        frames: true,
        fast: false,
        extra: extra(None, &labels),
        ..InApp::default()
    });
    shots_whole(&c, &labels, None);
    save(&c, "rolled", &labels);
}

/// Focus lost on KODY with key 5 held: the app lets go of it, so the digit counts at the loss
/// as a release; the window resized between digits and with key 1 held; the entry ends as 7531
/// and START plays world 5.
#[test]
fn focus_lost_with_a_digit_held_and_a_resize_mid_entry_still_enter_the_code() {
    let mut r = Run::to_the_title();
    r.press(FrameInput::joy(Joy::DOWN), 10, 20);
    r.press(FrameInput::joy(Joy::FIRE), 10, 10);
    r.press(key(digit(7)), 6, 6);
    r.hold(key(digit(5)), 20);
    let blur = r.steps() + 1;
    r.hold(FrameInput::NONE, 20);
    assert_eq!(r.now("code_digit_index"), 2, "5 counted on the loss of focus");
    // Resized after the loss of focus; the window takes some steps to reach a size asked for.
    let between = blur + 5;
    r.press(key(digit(3)), 6, 6);
    let resized = r.steps();
    r.hold(key(digit(1)), 4);
    let held = r.steps();
    r.hold(key(digit(1)), 4);
    r.hold(FrameInput::NONE, 10);
    assert_eq!(r.now("code_digits"), 0x0705_0301);
    assert_eq!(r.now("title_selected"), 0, "the fourth digit ended the entry");
    r.press(FrameInput::joy(Joy::UP), 10, 10);
    let play = r.start_and_play();
    assert_eq!(r.at(play, "world_index"), 4, "world 5");
    r.hold(FrameInput::NONE, 20);
    let labels = [(blur + 2, "blurred"), (resized, "resized"), (held + 2, "held-resized"), (play + 10, "world-5")];
    let (_app, c) = r.in_the_app_with(InApp {
        frames: true,
        fast: false,
        scale: None,
        blur_at: vec![blur],
        extra: format!("{between}: resize 640 480\n{held}: resize 900 700\n{}", extra(None, &labels)),
        ..InApp::default()
    });
    // A resize lands when the window server gets to it ([`SIZED`]): mid-entry the shot is whole
    // at either size; the entry is held by the state above, every step.
    let mid = shown_whole(&c, resized);
    assert!(mid == (640, 480) || mid == (640, 512), "resized mid-entry: {mid:?}");
    assert_eq!(shown_whole(&c, play + 10), (900, 700), "resized with key 1 held");
    shots_whole(&c, &labels, None);
    save(&c, "focus-resize", &labels);
}

/// The conformance runner's translated program from power-on to `level`'s first frame as
/// Start Level… starts it (`start_level_worlds.rs`'s `headless_world`).
fn headless_world(level: LevelId, cheat: bool) -> TranslatedEngine {
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
        assert_eq!(e.stopped(), None, "headless stopped on its way to {level}");
        let m = e.ram().unwrap();
        if fd_app::start_reached(start, &m[..CHIP], &m[CHIP..]) {
            return e;
        }
    }
    panic!("headless did not reach {level}");
}

fn state(e: &TranslatedEngine) -> String {
    let m = e.ram().expect("a frame has run");
    format!("{:016x}", fd_app::memory_hash(&m[..CHIP], &m[CHIP..]))
}

/// `e` on the app's inputs for steps `from..=to`, its state the app's at each.
fn follow(e: &mut TranslatedEngine, c: &Captured, from: u64, to: u64) {
    let fed = common::inputs(c);
    let hashes = common::hashes(c);
    for n in from..=to {
        e.frame(fed.get(&n).copied().unwrap_or_default());
        assert_eq!(e.stopped(), None, "headless stopped at step {n}");
        assert_eq!(state(e), hashes[n as usize], "state at step {n}");
    }
}

/// Start Level… opened on the title (W4-1, the cheat on, the recorder on), then in world 4's play
/// with the recording running: the recorder asks first and the recording is discarded, W6-1 starts
/// with the cheat off; Cancel in W6-1's play changes nothing; then W2-1 with the cheat off and no
/// recorder. Each start's play is headless's from that world's first frame.
#[test]
fn start_level_on_the_title_and_in_play_with_the_recorder_asking_starts_each_world() {
    title::require_tree();
    const END: u64 = 520;
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: END,
        timeout_s: 240,
        scale: None,
        extra_args: vec![],
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    let c = app.run_script_unread(
        "20: down AltLeft\n30: up AltLeft\n\
         250: menu start-level\nform level W4-1\nform cheat on\nform record on\nform start\n\
         260: down ArrowRight\n300: up ArrowRight\n\
         320: menu start-level\nanswer-recording discard\nform level W6-1\nform cheat off\nform record off\nform start\n\
         330: down ArrowLeft\n360: up ArrowLeft\n\
         380: menu start-level\nform level W2-1\nform cheat on\nform cancel\n\
         400: menu start-level\nform level W2-1\nform record off\nform start\n\
         410: down ArrowRight\n450: up ArrowRight\n\
         255: shot\n325: shot\n385: shot\n405: shot\n500: shot\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    for (want, what) in [
        ("fd-app: started W4-1 after step 249, cheat on, recording\n", "W4-1 on the title"),
        ("fd-app: recording discarded\n", "the recorder asked"),
        ("fd-app: started W6-1 after step 319\n", "W6-1 in play"),
        ("fd-app: started W2-1 after step 399\n", "W2-1 in play"),
    ] {
        assert!(c.stderr.contains(want), "{what}: {}", c.stderr);
    }
    assert!(c.prompts.contains("start-level: -> cancel"), "{}", c.prompts);
    assert_eq!(c.stderr.matches("fd-app: started").count(), 3, "Cancel started nothing: {}", c.stderr);
    assert!(
        std::fs::read_dir(&bugs).map_or(true, |d| d.count() == 0),
        "the discarded recording saved no zip"
    );
    for (level, cheat, from, to) in [
        (LevelId::W4_1, true, 250, 319),
        (LevelId::W6_1, false, 320, 399),
        (LevelId::W2_1, false, 400, END),
    ] {
        let mut e = headless_world(level, cheat);
        follow(&mut e, &c, from, to);
    }
    let labels = [(255, "w4-cheat"), (325, "w6"), (385, "w6-cancelled"), (405, "w2"), (500, "w2-play")];
    for (n, label) in labels {
        shown_whole_over(&c, n, label == "w4-cheat");
    }
    save(&c, "start-level", &labels);
}

/// World 3 by its code, Esc in play to give up, the game over screen and the high scores let go by
/// fire, the title back and START again into world 3, at 1200×900: shots of each screen whole, the
/// state headless every step (`title_paths.rs` drives the same path at scale 1).
#[test]
fn game_over_back_to_the_title_and_start_again_at_1200x900() {
    let mut r = Run::to_the_title();
    r.type_code([2, 4, 2, 4]);
    r.start_and_play();
    r.hold(FrameInput::NONE, 50);
    let esc = r.steps();
    r.press(key(title::ESC), 3, 60);
    let over = r.steps();
    let fire = FrameInput::joy(Joy::FIRE);
    r.press(fire, 10, 100);
    let scores = r.steps();
    r.press(fire, 10, 100);
    let after = r.steps();
    let back = r.until(
        "the title after the high scores",
        3_000,
        |n| if (n - after) % 40 < 10 { FrameInput::joy(Joy::DOWN) } else { FrameInput::NONE },
        |r| r.now("title_cursor") != 0,
    );
    r.hold(FrameInput::NONE, 10);
    r.press(FrameInput::joy(Joy::UP), 10, 10);
    assert_eq!(r.now("title_cursor"), 0);
    let play = r.start_and_play();
    assert_eq!(r.at(play, "world_index"), 2, "world 3 again");
    r.hold(FrameInput::NONE, 20);
    assert!(over > esc);
    let labels = [
        (over, "game-over"),
        (scores, "high-scores"),
        (back + 5, "title-again"),
        (play + 10, "world-3-again"),
    ];
    let size = Some((1200, 900));
    let (_app, c) = r.in_the_app_with(InApp {
        frames: true,
        fast: false,
        scale: None,
        timeout_s: 300,
        extra: extra(size, &labels),
        ..InApp::default()
    });
    shots_whole(&c, &labels, size);
    save(&c, "game-over", &labels);
}
