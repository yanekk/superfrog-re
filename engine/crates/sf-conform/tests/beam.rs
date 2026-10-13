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
