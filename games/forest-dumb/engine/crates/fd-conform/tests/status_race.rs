//! Where pass `32k + 1` takes `main_loop`'s status bar redraw, on the original (plans/forest-dumb-level-1
//! T71; docs/09-play-measured.md, "The status bar's redraw and the next pass").
//!
//! After pass `32k`, `main_loop` calls `h0_6a96` over and over until `irq_count` moves on, so the
//! level-3 interrupt starting pass `32k + 1` (line 233) lands somewhere in that loop. Read off line
//! samples ([`fd_conform::interrupted_pc`]), the place varies from redraw to redraw with the cycles the
//! passes took: no fixed split exists for the engine to copy, which is why a case accepts the
//! engine's cut redraw at the cut it measures ([`fd_conform::redraw_cut`]).

use std::collections::BTreeMap;
use std::path::Path;

use amiga_oracle::{Amiga, FrameInput};
use fd_conform::sync::reach_level_1;
use fd_conform::{Case, Inputs, Live, interrupted_pc, reach_case, redraw_cut};
use fd_core::{Mem, layout_var};

/// For every pass `32k + 1` started in `frames` steps: the pass and where it took the foreground (a
/// hunk 0 offset).
fn cuts(am: &mut Amiga, frames: u32, input: impl Fn(u32) -> FrameInput) -> Vec<(u32, u32)> {
    let irq = layout_var("irq_count").unwrap();
    am.set_profile(true);
    let mut out = Vec::new();
    for _ in 0..frames {
        let f = am.frame();
        am.step(input(f)).unwrap();
        let n = Live(am).value(irq);
        if n & 31 == 1
            && let Some(pc) = interrupted_pc(am)
        {
            out.push((n, pc));
        }
    }
    out
}

/// How many redraws were cut at each count of `h0_6a96`'s reads made before the pass (`None`: not
/// cut).
fn classes(seen: &[(u32, u32)]) -> BTreeMap<Option<(u8, u8)>, u32> {
    let mut m = BTreeMap::new();
    for &(_, pc) in seen {
        *m.entry(redraw_cut(pc).map(|r| (r.start, r.end)))
            .or_insert(0) += 1;
    }
    m
}

/// Idle at the level's start and on `death-spikes` (measured 2026-11-06): idle, the interrupt lands
/// mostly between the score's and the items' reads or after all four; on the spikes mostly in
/// `main_loop` before any. Each run has several classes, and pass 1441, which takes a life, comes
/// before the lives read.
#[test]
fn the_next_pass_cuts_the_redraw_at_no_fixed_read() {
    let (idle, spikes) = std::thread::scope(|s| {
        let idle = s.spawn(|| {
            let mut am = reach_level_1(1700).unwrap();
            cuts(&mut am, 1600, |_| FrameInput::NONE)
        });
        let spikes = s.spawn(|| {
            let case = Case::load(Path::new("cases/death-spikes.toml")).unwrap();
            let inputs = Inputs::load(&case).unwrap();
            let mut am = reach_case(&case, 0).unwrap();
            cuts(&mut am, 1600, |f| inputs.at(f))
        });
        (idle.join().unwrap(), spikes.join().unwrap())
    });
    for (name, seen) in [("idle", &idle), ("death-spikes", &spikes)] {
        let c = classes(seen);
        println!("{name}: {} redraws, {c:?}", seen.len());
        assert!(seen.len() >= 45, "{name}: {seen:x?}");
        assert!(c.len() >= 3, "{name}: one split would do: {c:?}");
    }
    let pc = spikes
        .iter()
        .find(|s| s.0 == 1441)
        .map(|s| s.1)
        .expect("pass 1441 sampled");
    assert_eq!(redraw_cut(pc), Some(0..1), "h0+${pc:x}");
}
