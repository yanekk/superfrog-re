//! Shell, test tooling: the conformance runner (DESIGN §2.1, §4).
//!
//! A [`Case`] names a sync point, an input script, a frame count and what to compare. [`run`]
//! brings the original (on the oracle) and the engine to the sync point, feeds both the same input
//! every frame and compares them; it returns the first frame that differs as a [`Divergence`]
//! naming the first variable, the first pixel (with a diff PNG) and the first Paula write.
//!
//! The CLI (`cargo run -q --release -p sf-conform -- run <case.toml> | all | record <case.toml>`)
//! is in `main.rs`; how to write scripts and cases is in `engine/conform/README.md`.

pub mod case;
pub mod compare;
pub mod disks;
pub mod report;
pub mod script;
pub mod sync;

pub use case::{Case, Compare, cases_dir, scripts_dir};
pub use report::{Divergence, PictureDiff};
pub use script::InputScript;
pub use sync::SyncPoint;

use sf_assets::GameData;
use sf_core::{Game, Start};

/// Run `case`: `Ok` when every compared frame matched. Panics when the oracle cannot be set up
/// (no Kickstart, no disks, a sync point that cannot be reached), with the reason.
pub fn run(case: &Case) -> Result<(), Divergence> {
    run_with(case, |_, _| {})
}

/// [`run`], calling `before_step(frame, game)` before the engine steps each frame: the hook a test
/// uses to plant a divergence.
pub fn run_with(
    case: &Case,
    mut before_step: impl FnMut(u32, &mut Game),
) -> Result<(), Divergence> {
    let synced = sync::reach(&case.sync, &case.script, case.frames)
        .unwrap_or_else(|e| panic!("{}: {e}", case.name));
    let mut amiga = synced.amiga;
    let start = match &case.sync {
        SyncPoint::PowerOn => Start::PowerOn {
            seed: synced.seed.unwrap_or_default(),
        },
        _ => Start::Transplant(synced.memory),
    };
    let mut game = Game::new(&GameData::default(), start);
    for f in 0..case.frames {
        let input = case.script.input_at(f);
        let theirs = amiga
            .step(input)
            .unwrap_or_else(|e| panic!("{}: oracle, frame {f}: {e}", case.name));
        let mem = sync::core_memory(amiga.memory());
        let pacing = sync::pacing(&amiga.cpu(), &mem);
        before_step(f, &mut game);
        let ours = game.step_paced(input, pacing);
        let d = compare::frame(case, f, &game.probe(), &mem, &ours, &theirs);
        if let Some(d) = d {
            return Err(d);
        }
    }
    Ok(())
}
