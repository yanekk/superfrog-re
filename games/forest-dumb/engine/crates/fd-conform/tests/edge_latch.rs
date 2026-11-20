//! When the next pass cuts `main_loop`'s edge redraws, on the original (plans/forest-dumb-level-1 T73,
//! T74; docs/09-play-measured.md, "The edge redraws and the next pass").
//!
//! The redraws' blits wait for the raster to be on lines 103 to 230 (`h0_3ac2`), and the next pass
//! comes at line 233. A row's 23 tiles always fit; its bobs, about 5 lines each, decide whether the
//! row's redraw returns to `main_loop` before that pass. When it does not, the latches after it in
//! `main_loop`'s order (left, right) are still set during the pass. The engine alone stops its redraw
//! half after a row of `ROW_BOBS_CUT` bobs or more (`fd_core::level::main_loop_redraws`). With the
//! animated cells and the creatures running passes are longer and cut rows and columns as the cycles
//! decide, so a case's runner hands the engine each measured cut ([`fd_conform::edges_cut`], T74).
//! Measured on the first 1 300 steps of T24's planner route (`edge-latch-route`), creatures live
//! (T79). T73 and T74 measured it with creature starts frozen, where every creature cell the scan
//! met stayed in the map and was cut as a bob: its rows of 8 and 9 bobs were that, and are gone.

use std::collections::BTreeMap;
use std::path::Path;

use amiga_chips::Picture;
use amiga_conform::Mode;
use amiga_oracle::{Amiga, FrameInput};
use fd_conform::case::Exclusion;
use fd_conform::{
    Case, Engine, Inputs, Live, Options, Outcome, PASS_END, STATUS_REDRAW, Sampler, Verdict,
    edges_cut, game_data, interrupted_pc, reach_case, records_edge_cuts, run, run_with,
};
use fd_core::level::{EdgesCut, Event, ROW_BOBS_CUT};
use fd_core::{Game, HUNK_BASES, Mem, Transplant, Var, layout_var};

const CASE: &str = "cases/edge-latch-route.toml";

/// A chip RAM byte to change on the sampled side. Any would do: the comparison stops at the step
/// it changes in.
const CHANGED_BYTE: u32 = 0x7_F000;

/// The case without its goal: the measurements below run the original a little short of its frames.
fn route() -> Case {
    Case {
        goal: None,
        ..Case::load(Path::new(CASE)).unwrap()
    }
}

/// `h0_774`, the tile blit, and `h0_3ac2`'s `rts`: a raster wait done, before a tile's or a bob's
/// blit.
const TILE: u32 = 0x774;
const WAITED: u32 = 0x3ada;

/// A window in which the foreground started a row's redraw (23 tiles): the pass that started in the
/// step after it, the row's bobs, and whether that pass found the row's redraw still running.
#[derive(Debug)]
struct Row {
    pass: u32,
    bobs: u32,
    cut: bool,
}

/// Every row redraw the original ran on the case's steps.
fn rows_on_the_original(case: &Case) -> Vec<Row> {
    let inputs = Inputs::load(case).unwrap();
    let mut am = reach_case(case, 0).unwrap_or_else(|e| panic!("{e}"));
    let irq = layout_var("irq_count").unwrap();
    am.watch_pcs(&[HUNK_BASES[0] + TILE, HUNK_BASES[0] + WAITED]);
    am.set_profile(true);
    let mut rows = Vec::new();
    for _ in 0..case.frames - 2 {
        let f = am.frame();
        am.step(inputs.at(f)).unwrap();
        let hits: BTreeMap<u32, u32> = am.pc_hits().iter().copied().collect();
        let (tiles, waited) = (
            hits[&(HUNK_BASES[0] + TILE)],
            hits[&(HUNK_BASES[0] + WAITED)],
        );
        // A column alone is 16 tiles, the rest of a cut row none: a row starts at 23. Its blits all
        // come in the window, which one step holds whole.
        if tiles < 23 {
            continue;
        }
        let pc = interrupted_pc(&am).expect("the pass started in the step");
        let waiting = matches!(pc, 0x3ac2..0x3ada) as u32;
        // Each tile and bob waits once; a column's bobs come after its 16 tiles, past the window.
        let bobs = waited + waiting - tiles;
        // The pass came inside the down or up redraw, or between their latches' tests: the row's.
        let cut = matches!(edges_cut(&am), Some(EdgesCut { next: 1 | 2, .. }));
        rows.push(Row {
            pass: Live(&am).value(irq),
            bobs,
            cut: tiles == 23 && cut,
        });
    }
    rows
}

/// The passes before which the engine's redraw half stopped after a row.
fn cuts_on_the_engine(case: &Case) -> Vec<u32> {
    let inputs = Inputs::load(case).unwrap();
    let mut g = Game::new(
        game_data().unwrap_or_else(|e| panic!("{e}")),
        fd_core::Start::Level1,
    );
    let mut cuts = Vec::new();
    for f in case.sync_frame..case.sync_frame + case.frames - 8 {
        g.step(inputs.at(f));
        g.take_events();
        if g.state().edges_resume.is_some() {
            cuts.push(g.state().irq_count as u32);
        }
    }
    cuts
}

/// Measured 2026-11-10, creatures live: 166 row redraws, none of more than 5 bobs, 4 cut by the
/// next pass, each a row of no bobs (passes 344, 348, 352 and 633), which the longer pass decides and
/// not the row. The engine alone, cutting at `ROW_BOBS_CUT`, cuts none of them; a case hands it the
/// measured cut instead. Frozen (T73, T74) the same steps held 17 rows of 9 bobs and 2 of 8, creature
/// cells the frozen `h0_681c` left in the map: no live row of W1-1 measured reaches the rule.
#[test]
fn live_no_row_reaches_the_bob_rule_and_the_cut_rows_are_the_long_passes() {
    let case = route();
    let (rows, engine): (Vec<Row>, Vec<u32>) = std::thread::scope(|s| {
        let rows = s.spawn(|| rows_on_the_original(&case));
        let engine = s.spawn(|| cuts_on_the_engine(&case));
        (rows.join().unwrap(), engine.join().unwrap())
    });
    assert!(rows.len() >= 150, "{} rows", rows.len());
    assert!(rows.iter().any(|r| r.bobs >= 3), "{rows:?}");
    assert!(
        rows.iter().all(|r| r.bobs < ROW_BOBS_CUT as u32),
        "{rows:?}"
    );
    let cut: Vec<(u32, u32)> = rows
        .iter()
        .filter(|r| r.cut)
        .map(|r| (r.pass, r.bobs))
        .collect();
    assert_eq!(cut, [(344, 0), (348, 0), (352, 0), (633, 0)]);
    // The engine alone stops after exactly the rows of `ROW_BOBS_CUT` bobs or more: none.
    assert_eq!(engine, [] as [u32; 0], "{rows:?}");
}

/// The route's first 1 300 steps on every compared variable, `edge_latch_right` included: it
/// differed at pass 1211 before the engine modelled the cut (T73), and on 2 more passes once the
/// animated cells ran (T47), until the runner handed the engine each measured cut (T74).
#[test]
fn the_routes_first_1300_steps_agree_on_the_right_edge_latch() {
    let case = Case::load(Path::new(CASE)).unwrap();
    assert!(case.exclude.is_empty(), "{:?}", case.exclude);
    assert!(case.vars.iter().any(|v| v == "edge_latch_right"));
    let v = run(&case).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.goal_met.is_some(), "{v}");
    assert!(v.passes >= 1200, "{v}");
    assert!(v.edge_cuts >= 20, "{v}");
}

/// The level engine with the runner's measured cut changed: dropped (the engine's own rule then
/// decides), or handed without where the cut redraw's scan stood.
struct Handed {
    game: Game,
    how: Hand,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hand {
    Dropped,
    WholeScan,
}

impl Transplant for Handed {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem)
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Handed {
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input)
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        Engine::cut_redraw(&self.game, cut)
    }
    fn creatures_uncut(&self, rows: &[u16]) -> Option<Picture> {
        self.game.creatures_uncut(rows)
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        match self.how {
            Hand::Dropped => {}
            Hand::WholeScan => self.game.cut_foreground(at.map(|c| EdgesCut {
                defer_from: None,
                ..c
            })),
        }
    }
}

fn handed(name: &str, how: Hand) -> Verdict {
    let mut case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    // The creature slots left out: a creature started at another pass differs before the latch or
    // the picture does (`mechanic-vanish` pass 304 with no cut, `mechanic-spring` pass 93 with the
    // scan whole), which `enemy_cases.rs` holds. This test is the latches' and the animated cells'.
    case.vars.retain(|n| !n.starts_with("creature_"));
    // `mechanic-spring` compares the play area: among what remains the scan's place shows in the
    // picture alone.
    if how == Hand::Dropped {
        // The same creature drawn: started at another pass it shows in the play area before the
        // latch differs (picture, pass 313), since `mechanic-vanish` excludes nothing (T81). The
        // play area is left out as the case left it out until then; the status bar stays compared.
        case.exclude.push(Exclusion {
            rows: 0..194,
            cols: None,
            frames: None,
            var: None,
            what: "rows 0..194".into(),
            task: "T83".into(),
        });
    }
    let game = Game::new(
        game_data().unwrap_or_else(|e| panic!("{e}")),
        fd_core::Start::Level1,
    );
    // The scan run whole is judged to the pixel: as the toolkit counts frames in a row (T82) its
    // 18 glitch frames, never three together, are inside the tolerance and the case passes.
    // Measured 2026-11-10: strict, it differs at frame 6397; the hand-over whole not before 6776.
    let opts = Options {
        mode: match how {
            Hand::Dropped => Mode::Faithful,
            Hand::WholeScan => Mode::Strict,
        },
        ..Options::default()
    };
    run_with(&case, &mut Handed { game, how }, &opts).unwrap_or_else(|e| panic!("{e}"))
}

/// What the runner hands over is what the original showed, and each half of it is needed (T74):
/// without the measured cut `mechanic-vanish` differs on `edge_latch_right` at pass 340, where a
/// pass came inside a redraw and the latch stayed set; with the cut but the redraw's scan run whole
/// `mechanic-spring`'s coins join other animated-cell lists and its picture differs at frame 6397,
/// to the pixel (`handed`).
#[test]
fn the_engine_fails_without_the_measured_cut_or_the_scan_it_stopped() {
    let (dropped, whole) = std::thread::scope(|s| {
        let d = s.spawn(|| handed("mechanic-vanish", Hand::Dropped));
        let w = s.spawn(|| handed("mechanic-spring", Hand::WholeScan));
        (d.join().unwrap(), w.join().unwrap())
    });
    assert!(
        matches!(&dropped.outcome, Outcome::Differs { var, pass, .. }
            if var.starts_with("edge_latch_") && *pass < 400),
        "{dropped}"
    );
    assert!(
        matches!(&whole.outcome, Outcome::PictureDiffers { frame, .. } if *frame < 6700),
        "{whole}"
    );
}

/// Resuming after a cut row runs every latch after it in the order and none before it, `frame_flag`
/// clear or not (T73 review: resuming one latch late passed). No live row of the route reaches the
/// engine's own rule (above), so the cut is set here: after an up row, as the frozen route made it,
/// and after a down row.
#[test]
fn a_resumed_redraw_half_clears_every_later_latch_and_no_earlier_one() {
    use fd_core::level::{Cut, camera::Dir, main_loop_redraws, redraw::LevelImage};
    let case = route();
    let inputs = Inputs::load(&case).unwrap();
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut g = Game::new(data.clone(), fd_core::Start::Level1);
    for f in case.sync_frame..case.sync_frame + 400 {
        g.step(inputs.at(f));
        g.take_events();
    }
    let image = LevelImage::new(&data.level, &data.program);
    let order = [Dir::Down, Dir::Up, Dir::Left, Dir::Right];
    // The cut after an up row, then as a cut down row would leave it.
    for from in [2, 1] {
        let mut s = g.state().clone();
        s.edges_resume = Some(from);
        s.frame_flag = 0;
        for dir in order {
            s.camera.edges.latch[dir as usize] = 0xffff;
            s.camera.edges.at[dir as usize] = (s.camera.x, s.camera.y);
        }
        // A row resumed into can be cut again; each call is the foreground after one more pass.
        let mut plane = g.plane().clone();
        for _ in 0..order.len() {
            main_loop_redraws(&mut s, &mut plane, &image, &mut Vec::new(), Cut::Rows);
            if s.edges_resume.is_none() {
                break;
            }
        }
        for (k, dir) in order.into_iter().enumerate() {
            let left = s.camera.edges.latch[dir as usize];
            let want = if k < from as usize { 0xffff } else { 0 };
            assert_eq!(left, want, "resumed at {from}: {dir:?}");
        }
        assert_eq!((s.frame_flag, s.edges_resume), (0, None));
    }
}

/// The original reached for `case`, twice: as it runs alone, and as [`run`] runs it, counting
/// `camera_update` and watching pass ends (both proven to change nothing, T31) and sampling the
/// steps [`Sampler`] picks, or every step when `every`. Steps both a frame at a time, up to `steps`,
/// and counting the status redraw's calls (T82),
/// and returns the first step whose chip and slow RAM, picture or CPU registers differ, and how
/// many steps were sampled. The sampled side's creature cuts are read every step as the runner
/// reads them (T80), from what the oracle keeps of each step whether read or not.
fn sampled_against_alone(case: &Case, steps: u32, every: bool) -> (Option<u32>, u32) {
    sampled_against_alone_with(case, steps, every, |_, _| {})
}

/// [`sampled_against_alone`], handing the sampled side to `after_sampled` after each step it
/// sampled, with the step, before the two sides are compared.
fn sampled_against_alone_with(
    case: &Case,
    steps: u32,
    every: bool,
    mut after_sampled: impl FnMut(&mut Amiga, u32),
) -> (Option<u32>, u32) {
    let inputs = Inputs::load(case).unwrap_or_else(|e| panic!("{e}"));
    let mut alone = reach_case(case, 0).unwrap_or_else(|e| panic!("{e}"));
    let mut seen = reach_case(case, 0).unwrap_or_else(|e| panic!("{e}"));
    // The runner also counts `main_loop`'s status redraw calls (T82): on the sampled side only,
    // so the count is held to change nothing as well.
    let status_redraw = HUNK_BASES[STATUS_REDRAW.0 as usize] + STATUS_REDRAW.1;
    alone.watch_pcs(&[Sampler::CAMERA_UPDATE]);
    seen.watch_pcs(&[Sampler::CAMERA_UPDATE, status_redraw]);
    for am in [&mut alone, &mut seen] {
        am.watch_write(Some(PASS_END));
    }
    let mut sampler = Sampler::default();
    let (mut profiling, mut sampled) = (false, 0);
    for step in 0..steps.min(case.frames) {
        let sample = every || sampler.samples(&seen, case);
        if sample != profiling {
            seen.set_profile(sample);
            profiling = sample;
        }
        sampled += sample as u32;
        let input = inputs.at(case.sync_frame + step);
        let a = alone.step(input).unwrap_or_else(|e| panic!("{e}"));
        let s = seen.step(input).unwrap_or_else(|e| panic!("{e}"));
        std::hint::black_box(fd_conform::creature_cuts(&seen));
        if sample {
            after_sampled(&mut seen, step);
        }
        if a.picture != s.picture || alone.cpu() != seen.cpu() || alone.memory() != seen.memory() {
            return (Some(step), sampled);
        }
        sampler.stepped(&seen);
    }
    (None, sampled)
}

/// A measurement that changes the original is not one (owner, 2026-11-09): in every case where the
/// runner samples lines to read the edge cut (`records_edge_cuts`: every level case since T78), the
/// original sampled as the runner samples it is the original alone, every
/// frame, in chip and slow RAM, picture and CPU registers. The same run reads each step's creature
/// cuts (`creature_cuts`, T80), which switches nothing on.
#[test]
fn sampling_for_the_edge_cut_leaves_the_original_unchanged_every_frame() {
    let dir = fd_conform::conform_dir().join("cases");
    let mut cases: Vec<Case> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| Case::load(&p).unwrap_or_else(|e| panic!("{e}")))
        .filter(|c| c.is_level() && records_edge_cuts(c))
        .collect();
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    let names: Vec<&str> = cases.iter().map(|c| c.name.as_str()).collect();
    for n in [
        "mechanic-spring",
        "mechanic-vanish",
        "edge-latch-route",
        "death-water",
        "enemy-route",
        "enemy-contact",
        "death-countdown",
    ] {
        assert!(names.contains(&n), "{names:?}");
    }
    let runs: Vec<(String, (Option<u32>, u32))> = std::thread::scope(|s| {
        let hs: Vec<_> = cases
            .iter()
            .map(|c| s.spawn(move || (c.name.clone(), sampled_against_alone(c, u32::MAX, false))))
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for (name, (differs, _)) in &runs {
        assert_eq!(*differs, None, "{name}: sampling changed the original");
    }
    let sampled: Vec<&str> = runs
        .iter()
        .filter(|r| r.1.1 > 0)
        .map(|r| r.0.as_str())
        .collect();
    assert!(sampled.len() > cases.len() / 2, "sampled: {sampled:?}");
    for n in ["enemy-route", "enemy-contact", "edge-latch-route"] {
        assert!(sampled.contains(&n), "{n} not sampled: {sampled:?}");
    }
}

/// The check above can fail (T85): one byte of the sampled side changed after a sampled step is
/// that step's difference, and the same steps undisturbed have
/// none. The check's own failures were never the sampling's: the oracle's core wiped a table every
/// machine shares whenever a machine was set up, under machines running on other threads (core
/// patch 0007, `amiga-oracle`'s `tests/machines_apart.rs`).
#[test]
fn a_sampled_step_made_to_differ_is_caught_at_that_step() {
    let case = Case::load(Path::new("cases/stretch-pit.toml")).unwrap();
    // The third sampled step: later than the first, so the steps before it are held equal too.
    let mut nth = 0;
    let mut changed = None;
    let (differs, _) = sampled_against_alone_with(&case, u32::MAX, false, |seen, step| {
        nth += 1;
        if nth == 3 {
            let byte = seen.peek(CHANGED_BYTE, 1)[0];
            seen.poke(CHANGED_BYTE, &[byte ^ 1]);
            changed = Some(step);
        }
    });
    assert!(changed.is_some(), "fewer than three sampled steps");
    assert_eq!(differs, changed);
    let (differs, sampled) = sampled_against_alone(&case, u32::MAX, false);
    assert_eq!(differs, None, "undisturbed");
    assert!(sampled >= 3);
}

/// With creatures live, sampling every step leaves the original as it runs alone, every frame: before
/// T78 a sampled step ended early where a blit held the CPU across the frame's end (`enemy-contact`:
/// a pass ended a step later from step 377; `enemy-route` from 336), so the runner handed no
/// measured edge cut there.
#[test]
fn sampling_every_step_leaves_the_original_unchanged_where_creatures_are_live() {
    std::thread::scope(|s| {
        let hs: Vec<_> = ["enemy-contact", "enemy-route"]
            .map(|name| {
                s.spawn(move || {
                    let case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
                    let (differs, sampled) = sampled_against_alone(&case, u32::MAX, true);
                    assert_eq!(differs, None, "{name}: sampling changed the original");
                    assert_eq!(sampled, case.frames, "{name}");
                })
            })
            .into_iter()
            .collect();
        for h in hs {
            h.join().unwrap();
        }
    });
}
