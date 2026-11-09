//! The engine's picture against the original's over whole runs (plans/forest-dumb-level-1 T44):
//! fd-core's `Game` drawing the play area, the character's sprites and the status bar
//! (`conform/cases/picture-*.toml`), judged within the owner's tolerance, every frame paired with
//! the pass it shows. Also the status bar's numbers across the digits each one shows, set in both
//! sides at once, and the coverage of T10's routines. Needs the disks and the Kickstart: absent, it
//! fails naming where it looked.

use std::path::Path;

use amiga_chips::{Picture, to_rgb12};
use amiga_conform::Tolerance;
use amiga_conform::compare::{PictureClass, PictureJudge, classify};
use amiga_oracle::{Amiga, FrameInput, OracleConfig, Rgb12Frame};
use fd_conform::sync::{DISK_2_SWAP, LEVEL_1_SYNC, level_1_script, reach_level_1};
use fd_conform::{
    Case, Engine, HUNK_BASES, Inputs, Live, Options, OracleMem, Outcome, PASS_END, PLAY_WINDOW,
    Verdict, coverage, disks, game_data, reach_case_counting, run_with,
};
use fd_core::character::anim::{WALK_LEFT, WALK_RIGHT};
use fd_core::{Event, Game, Mem, PORTED, Start, Transplant, Var, layout_var};

const CASES: [&str; 2] = ["picture-first-frame", "picture-scroll"];

/// The routines T10 registered (`fd-core/src/ported.rs`, "T10, the renderer").
const T10: [&str; 16] = [
    "h0_3ab4",
    "char_sprite_position",
    "h0_94ba",
    "h0_94fe",
    "h0_9550",
    "h0_9754",
    "char_sprite_copy",
    "h0_6a96",
    "h0_6ae8",
    "h0_6b70",
    "h0_6b78",
    "h0_6bc0",
    "h0_6c20",
    "h0_6c80",
    "h0_9a34",
    "h0_988a",
];
/// The blitter wait: only the spring's, the creatures' and the animated cells' blits call it
/// (`h0+$241a`, `$61ec`.., `h0_62b6`), T25's and T52's; 0 hits on every W1-1 route measured, unfrozen
/// included (2026-11-06). Reached by T46's and T47's cases; T48 checks every ported routine.
const NOT_HERE: &str = "h0_3ab4";
/// Run at start-up, before the level sync point: shown by a boot ([`the_start_up_routines_run_on_the_way_to_play`]).
const START_UP: [&str; 2] = ["h0_9a34", "h0_988a"];

fn case(name: &str) -> Case {
    Case::load(&Path::new("cases").join(format!("{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

fn game() -> Game {
    Game::new(game_data().unwrap_or_else(|e| panic!("{e}")), Start::Level1)
}

/// The level engine, recording what each pass left on screen: the character's sprite frame and the
/// camera.
struct Watched {
    game: Game,
    frames: Vec<Option<u32>>,
    cameras: Vec<(u16, u16)>,
}

impl Transplant for Watched {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Watched {
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn thaw(&mut self, offset: u32) {
        Engine::thaw(&mut self.game, offset);
    }
    fn pass(&mut self, input: FrameInput) {
        self.game.pass(input);
        self.frames.push(self.game.display().sprite.frame);
        let c = &self.game.state().camera;
        self.cameras.push((c.x, c.y));
    }
    fn picture(&self) -> Option<Picture> {
        Engine::picture(&self.game)
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.events()
    }
}

fn run(name: &str) -> (Case, Verdict, Watched) {
    let c = case(name);
    let mut w = Watched {
        game: game(),
        frames: Vec::new(),
        cameras: Vec::new(),
    };
    let v = run_with(&c, &mut w, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    (c, v, w)
}

fn hits(v: &Verdict, name: &str) -> u32 {
    v.coverage
        .hits
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |(_, h)| *h)
}

#[test]
fn each_picture_case_matches_with_nothing_differing_but_its_exclusions() {
    for name in CASES {
        let (c, v, _) = run(name);
        assert!(c.goal.is_some(), "{name} states no goal");
        assert!(c.compare.picture, "{name} compares no picture");
        assert_eq!(v.outcome, Outcome::Same, "{v}");
        assert!(v.goal_met.is_some(), "{v}");
        // Nothing tolerated: every frame not excluded is the original's to the pixel.
        assert_eq!(v.tolerated, [0, 0, 0], "{v}");
        // Nor a bar accepted as a cut redraw draws it (T71).
        assert_eq!(v.cut_redraws, 0, "{v}");
        // A picture a frame, but for the first (the transplant's step) and a frame now and then
        // that shows a pass whose end comes later.
        assert!(v.pictures + 3 >= c.frames, "{v}");
        // Every exclusion names its task (the parser requires it) and hid something.
        for (e, n) in &v.excluded {
            assert!(*n > 0, "{name}: {} hid nothing", e.what);
        }
    }
}

#[test]
fn the_scroll_case_scrolls_right_left_up_and_down() {
    let (_, v, w) = run("picture-scroll");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    let steps: Vec<(i32, i32)> = w
        .cameras
        .windows(2)
        .map(|p| (p[1].0 as i32 - p[0].0 as i32, p[1].1 as i32 - p[0].1 as i32))
        .collect();
    let count = |f: &dyn Fn(&(i32, i32)) -> bool| steps.iter().filter(|s| f(s)).count();
    let (right, left) = (count(&|s| s.0 > 0), count(&|s| s.0 < 0));
    let (down, up) = (count(&|s| s.1 > 0), count(&|s| s.1 < 0));
    // Measured: the view scrolls each way over tens of passes.
    assert!(
        right > 20 && left > 20 && down > 20 && up > 20,
        "right {right} left {left} down {down} up {up}"
    );
    let far = w.cameras.iter().map(|c| c.0).max().unwrap();
    assert!(far >= 0x90, "camera_x reached only {far:#x}");
}

#[test]
fn the_character_is_drawn_at_every_walk_frame_both_ways() {
    let (_, v, w) = run("picture-scroll");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    // The walk tables (`h0_4c28` right, `char_step_4be6` left): five frames each, `char_walk_t >> 3`
    // being 0 to 4 in play (docs/09, T51).
    for (way, table) in [("right", WALK_RIGHT), ("left", WALK_LEFT)] {
        for k in 0..5 {
            let frame = data.anim.long(table + 4 * k);
            assert!(
                w.frames.contains(&Some(frame)),
                "walking {way}: frame {k} ({frame:#x}) never drawn"
            );
        }
    }
}

/// Every routine of T10 but [`NOT_HERE`] and [`START_UP`] ran on the original in the cases here or
/// in the status bar's run below.
#[test]
fn every_t10_routine_ran_in_these_cases_but_the_ones_named() {
    let mut reached = vec![0u32; T10.len()];
    for name in CASES {
        let (_, v, _) = run(name);
        assert_eq!(v.outcome, Outcome::Same, "{v}");
        for (n, r) in T10.iter().zip(&mut reached) {
            *r += hits(&v, n);
        }
    }
    let status = status_bar_run();
    for (n, r) in T10.iter().zip(&mut reached) {
        *r += status.iter().find(|(m, _)| m == n).map_or(0, |(_, h)| *h);
    }
    let none: Vec<&str> = T10
        .iter()
        .zip(&reached)
        .filter(|(n, h)| **h == 0 && **n != NOT_HERE && !START_UP.contains(n))
        .map(|(n, _)| *n)
        .collect();
    assert!(none.is_empty(), "never reached: {none:?}");
    // The list is the register's: a routine T10 added would be missed otherwise.
    for n in T10 {
        assert!(PORTED.iter().any(|p| p.name == n), "{n} is not registered");
    }
    let mut t10: Vec<&str> = PORTED
        .iter()
        .filter(|p| T10.contains(&p.name))
        .map(|p| p.name)
        .collect();
    t10.sort_unstable();
    t10.dedup();
    assert_eq!(t10.len(), T10.len());
}

// --- the status bar's numbers --------------------------------------------------------------------

/// Memory with some variables replaced: what the engine is transplanted from, as the original
/// holds it once the same values are poked in.
struct Over<'a> {
    mem: &'a dyn Mem,
    with: Vec<(&'static Var, Vec<u8>)>,
}

impl Mem for Over<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        let mut b = self.mem.read(hunk, offset, len);
        for (v, bytes) in &self.with {
            for (k, &x) in bytes.iter().enumerate() {
                let at = v.offset + k as u32;
                if v.hunk == hunk && at >= offset && at < offset + len as u32 {
                    b[(at - offset) as usize] = x;
                }
            }
        }
        b
    }
}

/// One set of the bar's numbers, BCD as the variables hold them.
#[derive(Debug, Clone, Copy)]
struct Numbers {
    countdown: u16,
    lives: u16,
    items: u16,
    score: u32,
}

/// Every digit 0 to 9 in every place of the countdown, the items and the score's six places shown,
/// lives with each remainder by three (the icons) and threes from 0 to 33 (the two digits), and
/// T00's starting values (countdown 999, lives 32, items 282, score 0).
const SETS: [Numbers; 6] = [
    Numbers {
        countdown: 0x123,
        lives: 0x01,
        items: 0x456,
        score: 0x0012_3456,
    },
    Numbers {
        countdown: 0x456,
        lives: 0x02,
        items: 0x789,
        score: 0x0078_9012,
    },
    Numbers {
        countdown: 0x789,
        lives: 0x03,
        items: 0x012,
        score: 0x0034_5678,
    },
    Numbers {
        countdown: 0x050,
        lives: 0x10,
        items: 0x000,
        score: 0x0090_1234,
    },
    Numbers {
        countdown: 0x999,
        lives: 0x99,
        items: 0x999,
        score: 0x0099_9999,
    },
    Numbers {
        countdown: 0x999,
        lives: 0x32,
        items: 0x282,
        score: 0,
    },
];

/// Add a step's `pc_hits` (ascending by address) to `hits`, in the order of `pcs`.
fn add_hits(hits: &mut [u32], pcs: &[u32], step: &[(u32, u32)]) {
    for (h, pc) in hits.iter_mut().zip(pcs) {
        *h += step.iter().find(|(a, _)| a == pc).map_or(0, |(_, n)| *n);
    }
}

fn var(name: &str) -> &'static Var {
    layout_var(name).unwrap()
}

/// The bytes each set puts in memory. The countdown's digits are its first word; its second is left.
/// `countdown_tick` starts from 0, so the countdown does not tick in the window (63 passes away).
fn bytes(n: &Numbers) -> Vec<(&'static Var, Vec<u8>)> {
    vec![
        (var("countdown_tick"), vec![0; 4]),
        (var("countdown"), n.countdown.to_be_bytes().to_vec()),
        (var("lives"), n.lives.to_be_bytes().to_vec()),
        (var("items_left"), n.items.to_be_bytes().to_vec()),
        (var("score"), n.score.to_be_bytes().to_vec()),
    ]
}

/// The status bar's picture rows (194 to 249) of the engine's and the original's frame differ in
/// this many pixels.
fn bar_differs(ours: &Picture, theirs: &Rgb12Frame) -> usize {
    let c = to_rgb12(ours);
    let w = theirs.width as usize;
    (194 * w..250 * w)
        .filter(|&k| c[k] != theirs.pixels[k])
        .count()
}

/// For each of [`SETS`]: put the numbers into the original and the engine together, four passes
/// before `main_loop`'s next redraw (every 32 interrupts), and compare every frame until three
/// passes after it, the whole picture, strictly. Returns T10's routines' hits on the original.
fn status_bar_run() -> Vec<(&'static str, u32)> {
    let frames = 40 * SETS.len() as u32 + 40;
    let mut am = reach_level_1(frames).unwrap_or_else(|e| panic!("{e}"));
    let register: Vec<_> = PORTED
        .iter()
        .filter(|p| T10.contains(&p.name))
        .copied()
        .collect();
    let pcs: Vec<u32> = register.iter().map(coverage::address).collect();
    am.watch_pcs(&pcs);
    am.watch_write(Some(PASS_END));
    let irq = var("irq_count");
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut hits = vec![0u32; register.len()];
    let step = |am: &mut Amiga, hits: &mut Vec<u32>| {
        am.step(FrameInput::NONE).unwrap_or_else(|e| panic!("{e}"));
        add_hits(hits, &pcs, am.pc_hits());
    };
    for set in SETS {
        // The pass end four passes before a redraw.
        let (mut g, start) = loop {
            step(&mut am, &mut hits);
            let Some(mem) = am.watched_memory() else {
                continue;
            };
            let ended = OracleMem(&mem).value(irq);
            if ended & 31 == 28 && Live(&am).value(irq) & 31 != 0 {
                let mut g = Game::new(data.clone(), Start::Level1);
                g.transplant(&Over {
                    mem: &OracleMem(&mem),
                    with: bytes(&set),
                });
                for (v, b) in bytes(&set) {
                    am.poke(HUNK_BASES[v.hunk as usize] + v.offset, &b);
                }
                break (g, ended);
            }
        };
        let redraw = start + 4;
        let before = g.display().status.bytes.clone();
        let mut drawn: Vec<(u32, Picture)> = vec![(start, g.picture())];
        let mut last = start;
        let mut compared = 0;
        while compared == 0 || drawn.iter().all(|(n, _)| *n <= redraw + 3) {
            step(&mut am, &mut hits);
            if let Some(mem) = am.watched_memory() {
                let ended = OracleMem(&mem).value(irq);
                for n in last + 1..=ended {
                    g.step(FrameInput::NONE);
                    drawn.push((n, g.picture()));
                }
                last = last.max(ended);
            }
            let shown = Live(&am).value(irq) - 1;
            if let Some((_, ours)) = drawn.iter().find(|(n, _)| *n == shown) {
                let theirs = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
                assert_eq!(
                    bar_differs(ours, &theirs),
                    0,
                    "{set:x?}: pass {shown}, frame {}",
                    am.frame()
                );
                assert!(
                    to_rgb12(ours) == theirs.pixels,
                    "{set:x?}: pass {shown}, frame {}: the play area",
                    am.frame()
                );
                if shown > redraw {
                    compared += 1;
                }
            }
        }
        assert!(
            compared >= 2,
            "{set:x?}: {compared} frames after the redraw"
        );
        // The engine redrew the bar: the numbers are drawn, not left from before.
        assert_ne!(g.display().status.bytes, before, "{set:x?}");
        assert_eq!(
            g.display().status.shown.countdown,
            set.countdown,
            "{set:x?}"
        );
    }
    register.iter().map(|p| p.name).zip(hits).collect()
}

#[test]
fn the_status_bar_draws_every_digit_and_the_lives_icons_as_the_original() {
    let hits = status_bar_run();
    // Each number's own routine ran on the original, the score's digits through h0_6b70.
    for n in [
        "h0_6a96", "h0_6ae8", "h0_6b70", "h0_6b78", "h0_6bc0", "h0_6c20", "h0_6c80",
    ] {
        let h = hits.iter().find(|(m, _)| *m == n).map_or(0, |(_, h)| *h);
        assert!(h > 0, "{n} never ran");
    }
}

// --- the start-up's routines ---------------------------------------------------------------------

/// `h0_9a34` and `h0_988a` set the copper list's sprite and status bar pointers once, at start-up:
/// counted on a boot from power-on to the level sync point, where `picture-first-frame` shows the
/// engine's pointers drawing the original's frame.
#[test]
#[ignore = "boots from power-on, about 20 s: the full set"]
fn the_start_up_routines_run_on_the_way_to_play() {
    let kickstart = amiga_oracle::kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let dir = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    let disks = disks::read(&dir).unwrap_or_else(|e| panic!("{e}"));
    let (_, sc) = level_1_script().unwrap_or_else(|e| panic!("{e}"));
    let cfg = OracleConfig::new(kickstart, disks, LEVEL_1_SYNC);
    let mut am = Amiga::power_on(&cfg).unwrap_or_else(|e| panic!("{e}"));
    let register: Vec<_> = PORTED
        .iter()
        .filter(|p| START_UP.contains(&p.name))
        .copied()
        .collect();
    assert_eq!(register.len(), START_UP.len());
    let pcs: Vec<u32> = register.iter().map(coverage::address).collect();
    am.watch_pcs(&pcs);
    let mut hits = vec![0u32; pcs.len()];
    for frame in 0..LEVEL_1_SYNC {
        if frame == DISK_2_SWAP {
            am.insert_disk(0, 1);
        }
        am.step(amiga_conform::script::input_at(&sc.lines, frame))
            .unwrap_or_else(|e| panic!("{e}"));
        add_hits(&mut hits, &pcs, am.pc_hits());
    }
    for (p, h) in register.iter().zip(&hits) {
        assert!(*h > 0, "{} never ran before play", p.name);
    }
}

// --- the pairing -------------------------------------------------------------------------------

/// The runner's earlier pairing (T06 to T44): pass `n`'s picture against the frame of the step its
/// end was seen in. Judged as the runner judges, over `picture-scroll` with its exclusion.
fn by_pass_end(c: &Case) -> (u32, Result<(), String>) {
    let inputs = Inputs::load(c).unwrap_or_else(|e| panic!("{e}"));
    let (mut am, mut read) = reach_case_counting(c, 0, &[]).unwrap_or_else(|e| panic!("{e}"));
    am.watch_write(Some(PASS_END));
    let irq = var("irq_count");
    let mut g = game();
    let mut judge = PictureJudge::new(Tolerance::OWNER, c.frames);
    let mut at: Option<u32> = None;
    let mut glitches = 0;
    for step in 0..c.frames {
        let input = inputs.at(c.sync_frame + step);
        am.step(input).unwrap_or_else(|e| panic!("{e}"));
        read.step(&am, input).unwrap_or_else(|e| panic!("{e}"));
        let Some(mem) = am.watched_memory() else {
            continue;
        };
        let ended = OracleMem(&mem).value(irq);
        let Some(last) = at else {
            g.transplant(&OracleMem(&mem));
            read.drop_through(ended);
            at = Some(ended);
            continue;
        };
        if ended <= last {
            continue;
        }
        for n in last + 1..=ended {
            g.step(read.take(n).unwrap());
        }
        at = Some(ended);
        let mut theirs = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
        // The countdown's box, as the case excludes it: the engine's colours there.
        let ours = g.picture();
        let colours = to_rgb12(&ours);
        let w = theirs.width as usize;
        for y in 223..231 {
            for x in 144..168 {
                theirs.pixels[y * w + x] = colours[y * w + x];
            }
        }
        let class = classify(&ours, &theirs, &[], &Tolerance::OWNER);
        if matches!(class, PictureClass::Glitch(_)) {
            glitches += 1;
        }
        if let Err(e) = judge.frame(am.frame(), class) {
            return (glitches, Err(e));
        }
    }
    (glitches, Ok(()))
}

#[test]
fn pairing_a_pass_with_the_frame_its_end_was_seen_in_fails_the_scroll() {
    // Once passes end after the frame does (from frame 6334 here), the frame of the step a pass
    // ended in shows the pass before it: while the view scrolls every frame is a scroll apart from
    // the engine's. The runner pairs a frame with the pass started at its vertical blank, and the
    // same run matches to the pixel (each_picture_case_matches_...).
    let c = case("picture-scroll");
    let (glitches, judged) = by_pass_end(&c);
    assert!(
        judged.is_err(),
        "the old pairing passed: {glitches} glitches"
    );
    let (_, v, _) = run("picture-scroll");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.tolerated, [0, 0, 0], "{v}");
}
