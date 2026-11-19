//! Translator T37 (DESIGN §2.10): the app runs the translated program from power-on, level 1's
//! hand engine mounted, the same as a headless run of it frame for frame: its title with no flags,
//! Start by its key into level 1, and `--level 1` straight to level 1's first frame. The headless
//! run is the conformance runner's (`fd_conform::translated_engine::TranslatedEngine`), on the
//! inputs the capture says the app fed.
//!
//! Full: needs the owner's disks and the generated tree (`translate.py --build`), and fails naming
//! the command without it.

mod common;

use std::time::{Duration, Instant};

use amiga_input::FrameInput;
use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_e2e::{App, Captured, Image, LaunchOpts};
use fd_translated::TranslatedGame;

/// Chip RAM's size, where slow RAM starts in the engine's memory.
const CHIP: usize = 512 << 10;

/// `--level 1`'s run to level 1's first frame, launch included (about 3 s measured, T37).
const LEVEL_1_LIMIT: Duration = Duration::from_secs(30);

fn require_tree() {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
}

/// The conformance runner's translated program from power-on, level 1 mounted, as the app runs it.
fn headless() -> TranslatedEngine {
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut e = TranslatedEngine::power_on(&disks).unwrap_or_else(|e| panic!("{e}"));
    e.mount_level_1(common::data());
    e
}

/// The state hash the app writes, of the headless run's last frame.
fn state(e: &TranslatedEngine) -> String {
    let m = e.ram().expect("a frame has run");
    format!("{:016x}", fd_app::memory_hash(&m[..CHIP], &m[CHIP..]))
}

/// The headless run's last picture as the app shows it.
fn picture(e: &TranslatedEngine) -> Image {
    let p = fd_app::canvas(&FrameEngine::picture(e).expect("a frame has run"));
    let mut rgba = vec![0; p.width as usize * p.height as usize * 4];
    fd_core::render::rgba(&p, &mut rgba);
    Image {
        width: p.width as u32,
        height: p.height as u32,
        rgba,
    }
}

/// Step `e` on the app's input for steps `from..=to`, its state equal to the app's at each, and
/// its picture at each step in `pictures`.
fn follow(e: &mut TranslatedEngine, c: &Captured, from: u64, to: u64, pictures: &[u64]) {
    let fed = common::inputs(c);
    let hashes = common::hashes(c);
    assert_eq!(hashes.len() as u64, to + 1, "a hash a frame");
    for n in from..=to {
        e.frame(fed.get(&n).copied().unwrap_or_default());
        assert_eq!(e.stopped(), None, "the headless run stopped at step {n}");
        assert_eq!(state(e), hashes[n as usize], "state at step {n}");
        if pictures.contains(&n) {
            assert!(picture(e) == c.read_frame(n), "picture at step {n}");
        }
    }
}

/// The step `fd-app: level 1 begins at step N` names.
fn level_begins(c: &Captured) -> u64 {
    c.stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: level 1 begins at step "))
        .unwrap_or_else(|| panic!("level 1 never began: {}", c.stderr))
        .parse()
        .unwrap()
}

/// Fire, the default keys' Left Option, pressed at the disk 2 prompt: the title follows.
const PAST_THE_PROMPT: &str = "20: down AltLeft\n30: up AltLeft\n";

/// The step the title is up by after [`PAST_THE_PROMPT`] (from about step 184, T37's probe).
const TITLE: u64 = 300;

#[test]
fn launched_with_no_flags_it_shows_the_title_as_the_headless_run_does() {
    require_tree();
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: TITLE,
        timeout_s: 120,
        ..Default::default()
    });
    let c = app.run_script_unread(PAST_THE_PROMPT);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains("fd-app: playing the translated program from power-on"),
        "{}",
        c.stderr
    );
    let mut e = headless();
    follow(&mut e, &c, 1, TITLE, &[1, 10, 100, 200, TITLE]);
    // The prompt and the title are different screens.
    assert!(c.read_frame(10) != c.read_frame(TITLE), "the title is up");
}

/// Start, then fire on the SCENA 1 screen, by key, after [`PAST_THE_PROMPT`]: fire held ten steps
/// in twenty from the title on, the presses starting at or before step `until`.
fn start_by_key(until: u64) -> String {
    let mut s = String::from(PAST_THE_PROMPT);
    for at in (TITLE..=until).step_by(20) {
        s += &format!("{at}: down AltLeft\n{}: up AltLeft\n", at + 10);
    }
    s
}

/// Play's 600 steps after level 1 begins: run right, jump, run left.
fn play(from: u64) -> String {
    let at = |d: u64| from + d;
    format!(
        "{}: down ArrowRight\n{}: down Space\n{}: up Space\n{}: up ArrowRight\n{}: down ArrowLeft\n{}: up ArrowLeft\n",
        at(20),
        at(120),
        at(140),
        at(300),
        at(320),
        at(500)
    )
}

#[test]
fn start_by_its_key_begins_level_1_and_it_plays_as_the_headless_run_for_600_frames() {
    require_tree();
    // Where play begins on these keys, read off a first run so the second's play follows it.
    let mut probe = App::launch_translated(LaunchOpts {
        max_frames: TITLE + 600,
        timeout_s: 120,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let first = probe.run_script_unread(&format!(
        "fast-until {}\n{}",
        TITLE + 600,
        start_by_key(TITLE + 600)
    ));
    let begins = level_begins(&first);
    drop(probe);
    let end = begins + 600;
    // The press play begins on is read in the step it lands before (measured, T37), so it stays.
    let keys = start_by_key(begins);
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: end,
        timeout_s: 180,
        ..Default::default()
    });
    // The title fast, play at the game's pace.
    let c = app.run_script_unread(&format!("fast-until {begins}\n{keys}{}", play(begins)));
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(level_begins(&c), begins, "the same keys begin play at the same step");
    let mut e = headless();
    let shots: Vec<u64> = (begins..=end).step_by(50).chain([TITLE, end]).collect();
    follow(&mut e, &c, 1, end, &shots);
}

#[test]
fn level_1_starts_at_its_first_frame_run_headless_through_the_title() {
    require_tree();
    let t = Instant::now();
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: 1,
        timeout_s: 60,
        extra_args: vec!["--level".into(), "1".into()],
        ..Default::default()
    });
    let c = app.run_script_unread("");
    let took = t.elapsed();
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(took < LEVEL_1_LIMIT, "level 1's first frame took {took:?}");
    let frames: u64 = c
        .stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: level 1 reached headless after "))
        .and_then(|l| l.split(' ').next())
        .unwrap_or_else(|| panic!("{}", c.stderr))
        .parse()
        .unwrap();
    // The conformance run on the same way through the title.
    let mut e = headless();
    for n in 0..frames {
        e.frame(fd_app::to_level_1_input(n));
        assert_eq!(e.stopped(), None, "stopped at frame {n}");
    }
    let m = e.ram().unwrap();
    assert!(fd_app::in_level_1(&m[..CHIP], &m[CHIP..]), "headless is in level 1");
    let mut before = headless();
    for n in 0..frames - 1 {
        before.frame(fd_app::to_level_1_input(n));
    }
    let m = before.ram().unwrap();
    assert!(!fd_app::in_level_1(&m[..CHIP], &m[CHIP..]), "its first frame: not a frame later");
    let hashes = common::hashes(&c);
    assert_eq!(state(&e), hashes[0], "the app's first frame is the conformance run's");
    assert!(picture(&e) == c.read_frame(0), "and so is its picture");
    // And play goes on from there as headless.
    let fed = common::inputs(&c);
    e.frame(fed.get(&1).copied().unwrap_or(FrameInput::default()));
    assert_eq!(state(&e), hashes[1]);
}

/// The host samples the pacing clock had counted at each step (`clock.txt`).
fn clock(c: &Captured) -> std::collections::BTreeMap<u64, u64> {
    std::fs::read_to_string(c.dir.join("clock.txt"))
        .expect("clock.txt")
        .lines()
        .map(|l| {
            let (n, s) = l.split_once(' ').expect("`N samples`");
            (n.parse().unwrap(), s.parse().unwrap())
        })
        .collect()
}

/// Game ▸ Start Level… under the translated program (coordinator's answer to T37): it starts as
/// `--level 1` does, says the recorder is not available yet and starts without it, and play then
/// goes on at the game's pace from level 1's first frame, not in a burst making up the seconds the
/// run through the title took.
#[test]
fn start_level_restarts_as_level_1_does_without_the_recorder_and_at_the_games_pace() {
    require_tree();
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: 120,
        timeout_s: 120,
        ..Default::default()
    });
    let c = app.run_script_unread(
        "20: menu start-level\nform level W1-1\nform record on\nform start\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("not available yet; starting without it"), "{}", c.stderr);
    let after: u64 = c
        .stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: started W1-1 after step "))
        .unwrap_or_else(|| panic!("never restarted: {}", c.stderr))
        .parse()
        .unwrap_or_else(|_| panic!("recording after all: {}", c.stderr));
    let frames: u64 = c
        .stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: level 1 reached headless after "))
        .and_then(|l| l.split(' ').next())
        .unwrap_or_else(|| panic!("{}", c.stderr))
        .parse()
        .unwrap();
    let mut e = headless();
    for n in 0..frames {
        e.frame(fd_app::to_level_1_input(n));
    }
    let hashes = common::hashes(&c);
    let fed = common::inputs(&c);
    for n in after + 1..=120 {
        e.frame(fed.get(&n).copied().unwrap_or_default());
        assert_eq!(state(&e), hashes[n as usize], "state at step {n}");
    }
    // 50 steps at 50 Hz take about a second, 48 000 samples; a catch-up burst takes none. Bites in
    // release only: a debug build capturing every frame steps slower than the clock throughout.
    let clock = clock(&c);
    let took = clock[&(after + 51)] - clock[&(after + 1)];
    assert!(took > 40_000, "50 steps after the restart took {took} samples: a burst");
}

/// The window's size in points, as `resize` asks for it.
fn points(s: &fd_e2e::Shot) -> (u32, u32) {
    let p = |v: u32| (v as f64 / s.scale_factor).round() as u32;
    (p(s.surface.0), p(s.surface.1))
}

/// Every `shot` from `from` to `to`, every 10 steps: the window shows the frame it names whole,
/// centred and equal to the capture's; the last has the size `want`, if any.
fn shown_whole(c: &Captured, from: u64, to: u64, want: Option<(u32, u32)>) -> fd_e2e::Shot {
    let (_, w, h) = (0, fd_app::CANVAS.2 as u32, fd_app::CANVAS.3 as u32);
    for n in (from..=to).step_by(10) {
        let (frame, img) = common::shown(c, n, w, h);
        assert!(img == c.read_frame(frame), "step {n}: the window shows frame {frame} as captured");
        let (_, shot) = c.read_shot(n);
        let (x, y, vw, vh) = shot.viewport;
        assert!(
            (shot.surface.0 - vw) / 2 == x && (shot.surface.1 - vh) / 2 == y,
            "step {n}: {shot:?} is not centred"
        );
    }
    let (_, last) = c.read_shot(to);
    if let Some(want) = want {
        assert_eq!(points(&last), want, "not resized by step {to}: {last:?}");
    }
    last
}

/// At `fd-e2e`'s drill sizes (`drill_sizes.rs`): the default scale, twice, then 1000×700; scale 1,
/// then 500×500. The translated picture keeps one size (`fd_app::CANVAS`) from the title to play.
#[test]
fn the_translated_picture_is_whole_and_centred_at_the_drill_sizes() {
    require_tree();
    let (w, h) = (fd_app::CANVAS.2 as u32, fd_app::CANVAS.3 as u32);
    let shots: String = (30..=400).step_by(10).map(|n| format!("{n}: shot\n")).collect();
    for (scale, size, k) in [(None, (1000, 700), 2), (Some(1), (500, 500), 1)] {
        let mut app = App::launch_translated(LaunchOpts {
            max_frames: 410,
            timeout_s: 180,
            scale,
            ..Default::default()
        });
        // The disk 2 prompt, the title from about step 184, the resize asked for on the prompt.
        let c = app.run_script_unread(&format!(
            "{PAST_THE_PROMPT}10: shot\n20: resize {} {}\n{shots}",
            size.0, size.1
        ));
        assert!(c.exit.success(), "{}", c.stderr);
        let (_, first) = c.read_shot(10);
        assert_eq!(points(&first), (w * k, h * k), "{scale:?}: {first:?}");
        let last = shown_whole(&c, 30, 400, Some(size));
        assert_eq!(
            last.viewport.2 * size.0,
            last.surface.0 * k * w,
            "{scale:?}: {k}×, the largest whole multiple that fits: {last:?}"
        );
    }
}
