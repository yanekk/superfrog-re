//! When the next pass cuts `main_loop`'s edge redraws, on the original (plans/forest-dumb-level-1 T73,
//! T74; docs/09-play-measured.md, "The edge redraws and the next pass").
//!
//! The redraws' blits wait for the raster to be on lines 103 to 230 (`h0_3ac2`), and the next pass
//! comes at line 233. A row's 23 tiles always fit; its bobs, about 5 lines each, decide whether the
//! row's redraw returns to `main_loop` before that pass. When it does not, the latches after it in
//! `main_loop`'s order (left, right) are still set during the pass. The engine alone stops its redraw
//! half after a row of `ROW_BOBS_CUT` bobs or more (`fd_core::level::main_loop_redraws`). With the
//! animated cells running passes are longer and cut columns too, which the cycles decide, so a case's
//! runner hands the engine each measured cut ([`fd_conform::edges_cut`], T74). Measured on the first
//! 1 300 steps of T24's planner route, frozen as `fd_plan::confirm` freezes it.

use std::collections::BTreeMap;
use std::path::Path;

use amiga_chips::Picture;
use amiga_oracle::FrameInput;
use fd_conform::{
    Case, Engine, Inputs, Live, Options, Outcome, Verdict, edges_cut, engine_for, game_data,
    interrupted_pc, reach_case, run, run_with,
};
use fd_core::level::{EdgesCut, Event, ROW_BOBS_CUT};
use fd_core::{Game, HUNK_BASES, Mem, Transplant, Var, layout_var};

const CASE: &str = "cases/edge-latch-route.toml";

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
    let mut g = engine_for(case, game_data().unwrap_or_else(|e| panic!("{e}")));
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

/// Measured 2026-11-09 with the animated cells frozen (T73): rows of 9 bobs or more were cut, rows of
/// 7 or fewer were not, and of the two rows of 8 one was cut (pass 893, still in its scan at
/// `h0+$ca6`) and one was not (pass 899): where the 8th bob's blit ends against line 233 decides.
/// Re-measured the same day with them running (T74): the same, but for one row of no bobs of the 142
/// with 7 or fewer, cut in `h0_6882` (pass 328), which the scan's animated cells and a longer pass
/// decide. The engine alone keeps cutting at 8, which changes no state unless a later latch is
/// waiting; a case hands it the measured cut instead.
#[test]
fn a_row_of_nine_bobs_is_cut_by_the_next_pass_and_the_engine_cuts_the_same_rows() {
    let case = Case::load(Path::new(CASE)).unwrap();
    let (rows, engine) = std::thread::scope(|s| {
        let rows = s.spawn(|| rows_on_the_original(&case));
        let engine = s.spawn(|| cuts_on_the_engine(&case));
        (rows.join().unwrap(), engine.join().unwrap())
    });
    let by_bobs = |pred: &dyn Fn(u32) -> bool| -> Vec<&Row> {
        rows.iter().filter(|r| pred(r.bobs)).collect()
    };
    let many = by_bobs(&|b| b >= 9);
    let few = by_bobs(&|b| b <= 7);
    let eight = by_bobs(&|b| b == 8);
    assert!(many.len() >= 15, "{many:?}");
    assert!(many.iter().all(|r| r.cut), "{many:?}");
    assert!(few.iter().any(|r| r.bobs >= 3), "{few:?}");
    let few_cut: Vec<&&Row> = few.iter().filter(|r| r.cut).collect();
    assert!(
        few_cut.len() <= 1 && few_cut.iter().all(|r| r.bobs == 0),
        "{few_cut:?}"
    );
    assert!(
        eight.iter().any(|r| r.cut) && eight.iter().any(|r| !r.cut),
        "{eight:?}"
    );
    // The engine alone stops after exactly the rows of `ROW_BOBS_CUT` bobs or more.
    let expected: Vec<u32> = rows
        .iter()
        .filter(|r| r.bobs >= ROW_BOBS_CUT as u32)
        .map(|r| r.pass)
        .collect();
    assert_eq!(engine, expected, "{rows:?}");
    assert!(
        engine.contains(&1212),
        "pass 1211's up row and right column: {engine:?}"
    );
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
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn thaw(&mut self, offset: u32) {
        Engine::thaw(&mut self.game, offset);
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
    let case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    let game = Game::new(
        game_data().unwrap_or_else(|e| panic!("{e}")),
        fd_core::Start::Level1,
    );
    run_with(&case, &mut Handed { game, how }, &Options::default())
        .unwrap_or_else(|e| panic!("{e}"))
}

/// What the runner hands over is what the original showed, and each half of it is needed (T74):
/// without the measured cut `mechanic-vanish` differs on `edge_latch_down` where pass 293 came
/// inside the right column's redraw and `main_loop`'s `frame_flag` clear swallowed its down latch;
/// with the cut but the right column's scan run whole before pass 495, `mechanic-spring`'s coins
/// join other animated-cell lists and its picture differs (frame 6845).
#[test]
fn the_engine_fails_without_the_measured_cut_or_the_scan_it_stopped() {
    let (dropped, whole) = std::thread::scope(|s| {
        let d = s.spawn(|| handed("mechanic-vanish", Hand::Dropped));
        let w = s.spawn(|| handed("mechanic-spring", Hand::WholeScan));
        (d.join().unwrap(), w.join().unwrap())
    });
    assert!(
        matches!(&dropped.outcome, Outcome::Differs { var, pass, .. }
            if var == "edge_latch_down" && *pass < 300),
        "{dropped}"
    );
    assert!(
        matches!(&whole.outcome, Outcome::PictureDiffers { frame, .. } if *frame < 6900),
        "{whole}"
    );
}

/// Resuming after a cut row runs every latch after it in the order and none before it, `frame_flag`
/// clear or not: the route's cuts are all an up row before a right column, so they never show a
/// left latch waiting, or a cut down row's up latch (T73 review: resuming one latch late passed).
#[test]
fn a_resumed_redraw_half_clears_every_later_latch_and_no_earlier_one() {
    use fd_core::level::{Cut, camera::Dir, main_loop_redraws, redraw::LevelImage};
    let case = Case::load(Path::new(CASE)).unwrap();
    let inputs = Inputs::load(&case).unwrap();
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut g = engine_for(&case, data.clone());
    let mut f = case.sync_frame;
    while g.state().edges_resume.is_none() {
        g.step(inputs.at(f));
        g.take_events();
        f += 1;
        assert!(f < case.sync_frame + case.frames, "no cut on the route");
    }
    let image = LevelImage::new(&data.level);
    let order = [Dir::Down, Dir::Up, Dir::Left, Dir::Right];
    // The cut as the route makes it, then as a cut down row would leave it.
    for from in [g.state().edges_resume.unwrap(), 1] {
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
