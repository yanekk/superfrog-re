//! When the next pass cuts `main_loop`'s row redraw, on the original (plans/forest-dumb-level-1 T73;
//! docs/09-play-measured.md, "The edge redraws and the next pass").
//!
//! The redraws' blits wait for the raster to be on lines 103 to 230 (`h0_3ac2`), and the next pass
//! comes at line 233. A row's 23 tiles always fit; its bobs, about 5 lines each, decide whether the
//! row's redraw returns to `main_loop` before that pass. When it does not, the latches after it in
//! `main_loop`'s order (left, right) are still set during the pass. The engine stops its redraw half
//! after a row of `ROW_BOBS_CUT` bobs or more (`fd_core::level::main_loop_redraws`). Measured on the
//! first 1 300 steps of T24's planner route, frozen as `fd_plan::confirm` freezes it.

use std::collections::BTreeMap;
use std::path::Path;

use fd_conform::{
    Case, Inputs, Live, Outcome, engine_for, game_data, interrupted_pc, reach_case, run,
};
use fd_core::level::ROW_BOBS_CUT;
use fd_core::{HUNK_BASES, Mem, layout_var};

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
        // Spinning in `main_loop` (`$252..$35c`) or `h0_6a96`'s compares: the redraw half ended.
        let ended = matches!(pc, 0x252..0x35c | 0x6a96..0x6ae8);
        rows.push(Row {
            pass: Live(&am).value(irq),
            bobs,
            cut: tiles == 23 && !ended,
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

/// Measured 2026-11-09: rows of 9 bobs or more were cut, rows of 7 or fewer were not, and of the
/// two rows of 8 one was cut (pass 893, still in its scan at `h0+$ca6`) and one was not (pass 899):
/// where the 8th bob's blit ends against line 233 decides. The engine cuts at 8, which changes no
/// state unless a later latch is waiting; neither of those had one. Measured with the animated cells
/// frozen; since T47 they run, the original's passes are longer and it cuts other rows (one of no
/// bobs at pass 328), so the rule waits for T74 to restate it (owner, 2026-11-09).
#[test]
#[ignore = "T74: T73's nine-bob rule was measured with the animated cells frozen; restate it with them running"]
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
    assert!(few.iter().all(|r| !r.cut), "{few:?}");
    assert!(
        eight.iter().any(|r| r.cut) && eight.iter().any(|r| !r.cut),
        "{eight:?}"
    );
    // The engine stops after exactly the rows of `ROW_BOBS_CUT` bobs or more.
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
/// differed at pass 1211 before the engine modelled the cut. Since T47 runs the animated cells it is
/// excluded, naming T74, on the 2 passes the original's longer passes leave it set (owner, 2026-11-09).
#[test]
fn the_routes_first_1300_steps_agree_on_the_right_edge_latch() {
    let case = Case::load(Path::new(CASE)).unwrap();
    let only_t74 =
        |e: &fd_conform::Exclusion| e.var.as_deref() == Some("edge_latch_right") && e.task == "T74";
    assert!(
        case.exclude.len() == 1 && only_t74(&case.exclude[0]),
        "{:?}",
        case.exclude
    );
    let v = run(&case).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.goal_met.is_some(), "{v}");
    assert!(v.passes >= 1200, "{v}");
    assert!(v.excluded.iter().all(|(_, n)| *n > 0), "{v}");
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
