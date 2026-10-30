//! real-widescreen T06 (DESIGN §2.7, D2): widescreen switched mid-level. Switched on, the next step
//! wakes what is in the wide view at once (the start scan, widened); switched off, the extra slots
//! run on until the 4:3 despawn frees them. The same switches at the same frames give the same
//! game. Engine only: there is no original to compare widescreen with.

use sf_core::input::{FrameInput, Joy};
use sf_core::level::camera::{CAMERA_X, CAMERA_Y};
use sf_core::level::objects::{SLOTS, WIDE_LIVE, WIDE_SLOTS, slot_at};
use sf_core::{Game, LevelId, Start};

/// `coin_init` (h0+$23f60): a slot running it is a coin.
const COIN_INIT: u32 = sf_core::layout::h0(0x23f60);
/// The frog's hearts.
const HEARTS: u32 = sf_core::layout::h0(0x830);

fn start(level: &str) -> Game {
    Game::with_display(sf_conform::display_data(), Start::Level(level.parse::<LevelId>().unwrap()))
}

/// Frame `f`'s input: a walk right with jumps, so the camera scrolls and objects spawn.
fn walk(f: u32) -> FrameInput {
    let run = if f % 90 < 60 { Joy::RIGHT } else { Joy::NONE };
    FrameInput::joy(run | if f % 37 < 4 { Joy::UP } else { Joy::NONE })
}

fn w(g: &Game, a: u32) -> u16 {
    u16::from_be_bytes(g.peek(a, 2).try_into().unwrap())
}

fn l(g: &Game, a: u32) -> u32 {
    u32::from_be_bytes(g.peek(a, 4).try_into().unwrap())
}

/// Every live slot, the extras included: (its x from the camera, its map cell, its routine).
fn live(g: &Game) -> Vec<(i32, u32, u32)> {
    let cx = w(g, CAMERA_X + 2) as i32;
    (0..SLOTS + WIDE_SLOTS)
        .map(slot_at)
        .filter(|&s| l(g, s + 0x40) != 0)
        .map(|s| (w(g, s + 8) as i32 - cx, l(g, s + 0x40), l(g, s)))
        .collect()
}

/// In the right strip: past the original's screen (334 pixels) and inside widescreen's (399), short
/// of the cells `map_spawn_scan_edges`' wide column reaches (from about 416), and in the screen's
/// rows, clear of its wide row (32 pixels above the camera or 272 below), so only the switch's scan
/// can have woken it.
fn in_right_strip(x: i32, y: i32) -> bool {
    (334..400).contains(&x) && (0..240).contains(&y)
}

/// The slot's y from the camera.
fn y_of(g: &Game, cell: u32) -> i32 {
    let s = (0..SLOTS + WIDE_SLOTS).map(slot_at).find(|&s| l(g, s + 0x40) == cell).unwrap();
    w(g, s + 0xa) as i32 - w(g, CAMERA_Y + 2) as i32
}

/// Not a coin, and not the cell the 4:3 game holds: an enemy (or a platform) woken in the strip.
fn woken_in_strip(narrow: &Game, wide: &Game) -> Vec<(i32, u32, u32)> {
    let had: Vec<u32> = live(narrow).iter().map(|o| o.1).collect();
    live(wide)
        .into_iter()
        .filter(|o| in_right_strip(o.0, y_of(wide, o.1)) && o.2 != COIN_INIT && !had.contains(&o.1))
        .collect()
}

/// Switched on mid-W1-1, at a moment when an enemy's cell is in the right strip: it is live after
/// that one step, and the 4:3 game, stepped the same, does not have it.
#[test]
fn switching_on_mid_level_wakes_an_enemy_in_the_right_strip() {
    let mut g = start("W1-1");
    let mut found = None;
    for f in 0..1500 {
        if f >= 50 && f % 10 == 0 {
            let (mut narrow, mut wide) = (g.clone(), g.clone());
            wide.set_widescreen(true);
            narrow.step(walk(f));
            wide.step(walk(f));
            let woken = woken_in_strip(&narrow, &wide);
            if !woken.is_empty() {
                found = Some((f, woken));
                break;
            }
        }
        g.step(walk(f));
    }
    let (f, woken) = found.expect("no frame of W1-1 with an enemy in the right strip");
    eprintln!("W1-1 switched on at frame {f}: woke {woken:x?}");
}

/// Switched off with extra slots live, the 4:3 game runs them on, never spawns into an extra, and
/// frees them by its despawn as the camera leaves them; then it walks the 20 and leaves the extras'
/// range still. Nothing panics where the 4:3 loops meet a live extra. The frog is a ghost of W3-1's
/// bench route played in 4:3 (as `widescreen_capacity.rs`'s), so the camera travels the level
/// whatever the enemies do.
#[test]
fn switching_off_lets_the_extras_drain_by_the_4_3_rules() {
    use sf_core::frog::FROG;
    let route = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../../plans/smart-planner/bench/routes/app/W3-1.txt"
    ))
    .expect("the bench's W3-1 route");
    let script = sf_conform::InputScript::parse(&route).expect("a route");
    let begin = || {
        Game::start_level(
            sf_conform::display_data(),
            &sf_core::recording::LevelSetup { level: "W3-1".parse().unwrap(), ..Default::default() },
            sf_core::Rng::seeded(1, 1),
        )
    };
    let (mut lead, mut g) = (begin(), begin());
    g.set_widescreen(true);
    let mut f = 0u32;
    // One frame of both; the frames run so far.
    let mut step = |g: &mut Game| {
        lead.step_unseen(script.input_at(f));
        g.step_unseen(script.input_at(f));
        g.poke(FROG, &lead.peek(FROG, 0x100));
        g.poke(HEARTS, &lead.peek(HEARTS, 2));
        f += 1;
        assert!(lead.scene().is_level() && g.scene().is_level(), "the level ended at frame {f}");
        f
    };
    // Widescreen until more than one extra slot is live.
    while w(&g, WIDE_LIVE) < 2 {
        assert!(step(&mut g) < 4000, "no two extra slots were ever live in W3-1");
    }
    // Off while an extra is well inside the wide view and the 4:3 game would not yet free it.
    let held = |g: &Game| extras(g).into_iter().filter(|o| (-0x30..0x170).contains(&o.1)).count();
    while held(&g) == 0 {
        assert!(step(&mut g) < 4000, "no extra slot inside the 4:3 margins in W3-1");
    }
    let at_off = w(&g, WIDE_LIVE);
    let seen = extras(&g);
    g.set_widescreen(false);
    let mut last = at_off;
    let mut drained = None;
    let mut ran = false;
    for k in 0..3000 {
        step(&mut g);
        // An extra the 4:3 game keeps moves on (its update runs).
        ran |= extras(&g).iter().any(|o| seen.iter().any(|p| p.0 == o.0 && (p.1 != o.1 || p.2 != o.2)));
        let n = w(&g, WIDE_LIVE);
        assert!(n <= last, "a 4:3 step spawned into an extra slot ({last} → {n}, {k} after the switch)");
        last = n;
        if n == 0 {
            drained = Some(k);
            break;
        }
    }
    let k = drained.expect("the extras never drained");
    eprintln!("W3-1: extras at the switch off {seen:x?}, drained {k} frames after it");
    assert!(ran, "no extra moved after the switch off");
    // Drained, the 4:3 game walks the 20 again and the extras' range stays still.
    let range = |g: &Game| g.peek(slot_at(SLOTS), WIDE_SLOTS * 0x7c);
    let before = range(&g);
    for _ in 0..200 {
        step(&mut g);
    }
    assert_eq!(range(&g), before, "a drained 4:3 game left the extra slots alone");
}

/// Each live extra slot: (its address, its x from the camera, its y).
fn extras(g: &Game) -> Vec<(u32, i32, u16)> {
    let cx = w(g, CAMERA_X + 2) as i32;
    (SLOTS..SLOTS + WIDE_SLOTS)
        .map(slot_at)
        .filter(|&s| l(g, s + 0x40) != 0)
        .map(|s| (s, w(g, s + 8) as i32 - cx, w(g, s + 0xa)))
        .collect()
}

/// Two runs switched at the same frames are the same game for 3 000 steps, frame by frame.
#[test]
fn the_same_switches_give_the_same_game() {
    let switches = [(120, true), (700, false), (950, true), (1800, false), (2300, true)];
    let run = || {
        let mut g = start("W1-2");
        let mut states = Vec::new();
        for f in 0..3000u32 {
            if let Some(&(_, on)) = switches.iter().find(|s| s.0 == f) {
                g.set_widescreen(on);
            }
            g.step_unseen(walk(f));
            if f % 100 == 99 {
                states.push(g.state().clone());
            }
        }
        (states, g.state().clone())
    };
    let (a, a_end) = run();
    let (b, b_end) = run();
    assert!(a == b && a_end == b_end, "two runs with the same switches differ");
}

/// The level from where `level_init` has run on the front end (a restart after the frog's death)
/// until the loop has run `after` frames: switched on `before` frames before the level scene
/// begins (during `level_start`'s fade-in), or with `before` 0 as the loop's first step is taken.
fn restart(before: u32, after: u32) -> (Game, Game) {
    let mut off = start("W1-1");
    // No heart left: the first touch costs a life.
    off.poke(sf_core::frog::FROG + 0x5a, &[0, 0]);
    let mut f = 0;
    while off.scene().is_level() {
        off.step(walk(f));
        f += 1;
        assert!(f < 1500, "the frog never died");
    }
    // How many frames until the level's loop runs again.
    let mut ahead = off.clone();
    let mut to_level = 0;
    while !ahead.scene().is_level() {
        ahead.step(FrameInput::default());
        to_level += 1;
        assert!(to_level < 2000, "the level never restarted");
    }
    assert!(to_level > before, "the restart is {to_level} frames, the switch {before} before");
    for _ in 0..to_level - before {
        off.step(FrameInput::default());
    }
    let mut on = off.clone();
    on.set_widescreen(true);
    for _ in 0..before + after {
        off.step(FrameInput::default());
        on.step(FrameInput::default());
    }
    assert!(on.scene().is_level() && off.scene().is_level());
    (off, on)
}

/// What is live in the left strip (from the original's left despawn margin out to widescreen's).
fn left_strip(g: &Game) -> usize {
    live(g).iter().filter(|o| o.0 < -0x40).count()
}

/// What is live in the right strip, short of `map_spawn_scan_edges`' wide column: the edge scans
/// alone leave it empty (the left one runs here, the camera last scrolled left), so only the
/// switch's scan wakes it.
fn right_strip(g: &Game) -> usize {
    live(g).iter().filter(|o| (334..400).contains(&o.0)).count()
}

/// Switched on during the fade-in, after `level_init`'s start scan ran the original's: the loop's
/// first step wakes the strips.
#[test]
fn switching_on_during_the_fade_in_wakes_the_strips() {
    let (off, on) = restart(3, 1);
    assert_eq!(left_strip(&off) + right_strip(&off), 0, "4:3: {:?}", live(&off));
    assert!(left_strip(&on) > 0 && right_strip(&on) > 0, "widescreen: {:?}", live(&on));
}

/// Switched on just as the loop is entered: the next step, the loop's first, wakes the strips.
/// (Normal play carries no pass over here; a switch mid-pass is
/// `switching_on_mid_pass_wakes_the_right_strip`.)
#[test]
fn switching_on_on_the_levels_first_frame_wakes_the_strips() {
    let (off, on) = restart(0, 1);
    assert_eq!(left_strip(&off) + right_strip(&off), 0, "4:3: {:?}", live(&off));
    assert!(left_strip(&on) > 0 && right_strip(&on) > 0, "widescreen: {:?}", live(&on));
    // A level started directly (W6-1: its start has objects in the strips), its first step taken,
    // then switched.
    let mut off = start("W6-1");
    off.step(FrameInput::default());
    let mut on = off.clone();
    on.set_widescreen(true);
    off.step(FrameInput::default());
    on.step(FrameInput::default());
    let strips = |g: &Game| live(g).iter().filter(|o| o.0 < -0x40 || o.0 >= 334).count();
    assert_eq!(strips(&off), 0, "4:3: {:?}", live(&off));
    assert!(strips(&on) > 0, "widescreen: {:?}", live(&on));
}

/// P pauses the level; switched on while paused, the strips are woken in the paused step, and the
/// objects run once P resumes.
#[test]
fn switching_on_while_paused_wakes_the_strips() {
    const P: u8 = 0x19;
    let mut off = start("W1-1");
    for f in 0..200 {
        off.step(walk(f));
    }
    // P down and up, then a few frames paused.
    off.step(FrameInput::key(P));
    for _ in 0..10 {
        off.step(FrameInput::default());
    }
    let mut on = off.clone();
    on.set_widescreen(true);
    let frozen = live(&off);
    off.step(FrameInput::default());
    on.step(FrameInput::default());
    assert_eq!(live(&off), frozen, "the game is paused");
    let had: Vec<u32> = live(&off).iter().map(|o| o.1).collect();
    let woken: Vec<_> = live(&on).into_iter().filter(|o| !had.contains(&o.1)).collect();
    assert!(!woken.is_empty(), "nothing woken while paused: {:?}", live(&on));
    assert!(woken.iter().all(|o| o.0 < 0 || o.0 >= 334), "woken outside the strips: {woken:?}");
    // P again: the woken objects are among the live as play goes on.
    on.step(FrameInput::key(P));
    for f in 0..10 {
        on.step(walk(f));
    }
    assert!(on.scene().is_level());
}


/// Switched on with a pass carried over from the last step (`Game::mid_pass`, a frame cut on the
/// original's schedule, as `alone.rs` cuts them): the pass runs again from its start, the switch's
/// scan in that start, so what is in the right strip is live after the step (T06 review: no other
/// test switched mid-pass).
#[test]
fn switching_on_mid_pass_wakes_the_right_strip() {
    let late = sf_core::Pacing::Late {
        writes: 500,
        line: 40,
        hpos: 100,
        phase: 1,
        step: 4,
        point: 0,
        blanks: 0,
    };
    let mut g = start("W1-1");
    let mut found = None;
    for f in 0..1500 {
        g.step(walk(f));
        if f >= 50 && f % 10 == 0 {
            let mut cut = g.clone();
            cut.step_paced(walk(f), late);
            if !cut.mid_pass() {
                continue;
            }
            let (mut narrow, mut wide) = (cut.clone(), cut);
            wide.set_widescreen(true);
            narrow.step(walk(f));
            wide.step(walk(f));
            let woken = woken_in_strip(&narrow, &wide);
            if !woken.is_empty() {
                found = Some((f, woken));
                break;
            }
        }
    }
    let (f, woken) = found.expect("no switch mid-pass with an enemy in the right strip");
    eprintln!("W1-1 switched on mid-pass at frame {f}: woke {woken:x?}");
}
