//! The level's cost model on the beam (`sf_core::level::beam`, T36): what normal play has, with
//! no anchors at the original's Paula writes or blit starts. Over `items-run-w1-1`, each Paula
//! write the engine makes lands near the line the original made it on: the median error within a
//! line, 70 % of them within 3 lines and 98 % within 10 (T36 measured 72 % and 99 %).
//!
//! Frames the original ends mid-pass and the frame after are left out (the engine runs the pass
//! across them its own way), and so are frames whose writes are not the original's, register for
//! register: there is no pairing to measure. When the variables differ the engine is re-synced
//! from the original's memory, as the runner does after a lag.

use sf_conform::{Case, compare, sync};
use sf_core::{Game, Start};

/// Colour clocks in a PAL line (a write's `hpos` counts them).
const LINE: f64 = 227.0;

#[test]
fn the_cost_model_alone_puts_paula_writes_on_the_originals_lines() {
    let case = Case::load(&sf_conform::cases_dir().join("items-run-w1-1.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, case.frames + 10, frozen)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut amiga = synced.amiga;
    let display = sf_conform::display_data();
    let mut game = Game::with_display(display.clone(), Start::Transplant(synced.memory));
    let mut errors: Vec<f64> = Vec::new();
    let (mut measured, mut resynced) = (0, 0);
    let mut after_mid = false;
    for f in 0..case.frames {
        frozen.apply(&mut amiga, &display);
        let input = case.script.input_at(f);
        let theirs = sync::step_inserting(&mut amiga, input).unwrap_or_else(|e| panic!("{e}"));
        let mem = sync::core_memory(amiga.memory());
        let pacing = sync::pacing(&amiga.cpu(), &mem, &theirs.paula);
        let two = amiga.frames_run() == 2;
        if two {
            game.step_paced(input, pacing);
        }
        let ours = game.step_paced(input, pacing);
        let mid = sync::mid_pass(&amiga.cpu(), &mem);
        let same = theirs.paula.len() == ours.paula.len()
            && theirs
                .paula
                .iter()
                .zip(&ours.paula)
                .all(|(t, o)| t.reg_value() == o.reg_value());
        if same && !mid && !after_mid && !two && !theirs.paula.is_empty() {
            measured += 1;
            errors.extend(theirs.paula.iter().zip(&ours.paula).map(|(t, o)| {
                let at = |w: &sf_paula::PaulaWrite| w.vpos as f64 * LINE + w.hpos as f64;
                (at(o) - at(t)) / LINE
            }));
        }
        after_mid = mid;
        if !mid && compare::vars(game.layout(), &game.probe(), &mem, &[]).is_some() {
            game = Game::with_display(display.clone(), Start::Transplant(mem.clone()));
            resynced += 1;
        }
    }
    // The median as `fit` reports it: of the signed error, the model's bias (the median of its
    // size is 1.9 lines).
    errors.sort_by(|a, b| a.total_cmp(b));
    let n = errors.len();
    assert!(measured >= 100 && n > 0, "{measured} frames measured");
    let within = |k: f64| errors.iter().filter(|e| e.abs() <= k).count() as f64 / n as f64;
    let median = errors[n / 2];
    let report = format!(
        "{n} writes on {measured} frames ({resynced} re-syncs): median {median:.2} lines, {:.1} % \
         within 3, {:.1} % within 10, from {:.1} to {:.1}",
        100.0 * within(3.0),
        100.0 * within(10.0),
        errors[0],
        errors[n - 1]
    );
    assert!(median.abs() <= 1.0, "{report}");
    assert!(within(3.0) >= 0.70, "{report}");
    assert!(within(10.0) >= 0.98, "{report}");
}

/// Each `sfx_play` in a frame's writes: the clocks from its DMACON off to its DMACON on, and the
/// lines from its INTENA `$4000` to its `$c000` (the raster wait). An effect stealing its channel
/// makes neither DMACON write and is left out.
fn sfx_plays(ws: &[sf_paula::PaulaWrite]) -> Vec<(i64, u16)> {
    let at = |w: &sf_paula::PaulaWrite| w.vpos as i64 * LINE as i64 + w.hpos as i64;
    let mut out = Vec::new();
    for (i, w) in ws.iter().enumerate() {
        // DMACON off of one channel, then its period's 1, then INTENA `$4000` and `$c000`.
        let [off, per, masked, back] = [i, i + 1, i + 2, i + 3].map(|k| ws.get(k));
        let (Some(off), Some(per), Some(masked), Some(back)) = (off, per, masked, back) else {
            break;
        };
        let bit = off.value & 0xf;
        if !(w.reg == 0x96 && off.value & 0x8000 == 0 && bit.count_ones() == 1) {
            continue;
        }
        let chan = bit.trailing_zeros() as u16;
        if per.reg != 0xa6 + chan * 0x10
            || per.value != 1
            || (masked.reg, masked.value) != (0x9a, 0x4000)
            || (back.reg, back.value) != (0x9a, 0xc000)
        {
            continue;
        }
        let Some(on) = ws[i + 4..].iter().find(|x| x.reg == 0x96 && x.value == 0x8000 | bit) else {
            continue;
        };
        out.push((at(on) - at(off), back.vpos - masked.vpos));
    }
    out
}

/// T216: `sfx_play` (h0+$12a72) turns its channel's DMA off, waits for the line whose VPOS low
/// byte is three on, sets the channel's record up and turns the DMA on: whether Paula restarts the
/// channel with the new effect depends on that time (it notices the off only when the old period
/// runs out). Over `bonus-enter-wb-1` (WB-1's shots on channel 2), the engine on its cost model
/// alone ends every wait three lines on (four from a line's end), and its off-to-on times are the original's within 150
/// colour clocks at the median (before T216: ~780 against the original's 969-1208, the wait one
/// line).
#[test]
fn sfx_play_waits_three_lines_and_turns_the_dma_on_when_the_original_does() {
    let case = Case::load(&sf_conform::cases_dir().join("bonus-enter-wb-1.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let synced = sync::reach(&case.sync, &case.script, case.frames + 10).unwrap_or_else(|e| panic!("{e}"));
    let mut amiga = synced.amiga;
    let display = sf_conform::display_data();
    let mut game = Game::with_display(display.clone(), Start::Transplant(synced.memory));
    let (mut theirs_all, mut ours_all) = (Vec::new(), Vec::new());
    let mut gaps: Vec<i64> = Vec::new();
    for f in 0..case.frames {
        let input = case.script.input_at(f);
        let theirs = sync::step_inserting(&mut amiga, input).unwrap_or_else(|e| panic!("{e}"));
        let mem = sync::core_memory(amiga.memory());
        let pacing = sync::pacing(&amiga.cpu(), &mem, &theirs.paula);
        if amiga.frames_run() == 2 {
            game.step_paced(input, pacing);
        }
        let ours = game.step_paced(input, pacing);
        let (t, o) = (sfx_plays(&theirs.paula), sfx_plays(&ours.paula));
        if t.len() == o.len() {
            gaps.extend(t.iter().zip(&o).map(|(t, o)| (o.0 - t.0).abs()));
        }
        theirs_all.extend(t);
        ours_all.extend(o);
        if !sync::mid_pass(&amiga.cpu(), &mem)
            && compare::vars(game.layout(), &game.probe(), &mem, &[]).is_some()
        {
            game = Game::with_display(display.clone(), Start::Transplant(mem.clone()));
        }
    }
    assert!(gaps.len() >= 10, "{} sfx_plays paired", gaps.len());
    // Three lines, four when INTENA `$4000` falls at a line's end and VPOS is read on the next.
    assert!(theirs_all.iter().all(|p| (3..=4).contains(&p.1)), "the original's waits: {theirs_all:?}");
    assert!(ours_all.iter().all(|p| (3..=4).contains(&p.1)), "the engine's waits: {ours_all:?}");
    gaps.sort();
    let median = gaps[gaps.len() / 2];
    assert!(
        median <= 150,
        "median {median} clocks off; original {:?}, engine {:?}",
        theirs_all.iter().map(|p| p.0).collect::<Vec<_>>(),
        ours_all.iter().map(|p| p.0).collect::<Vec<_>>()
    );
}
