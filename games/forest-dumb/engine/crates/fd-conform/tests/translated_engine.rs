//! The translated program as a case's engine (translator T28), from level 1's sync point. Full: the
//! owner's disks and Kickstart, and the tree built in, which `fd-translated` takes from
//! `games/forest-dumb/build/translator/tree/` when this test is compiled. Without it these tests
//! fail naming the command that makes it, never skip (DESIGN §2.14).
//!
//! Each starts the original from power-on to take its start (`case_start`, about 8 s), since
//! the sync cache keeps no custom registers.

use fd_conform::translated_engine::{TranslatedEngine, case_start};
use fd_conform::{Case, Engine, Options, run_with};
use fd_core::{Transplant, layout_var};
use fd_translated::TranslatedGame;

fn generated() {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
}

fn w1_idle() -> Case {
    Case::load(std::path::Path::new("cases/w1-idle.toml")).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; the original from power-on"]
fn fifty_frames_from_level_1s_sync_point_run_with_no_stop() {
    generated();
    let start = case_start(&w1_idle()).unwrap_or_else(|e| panic!("{e}"));
    let first = start.pass;
    let mut e = TranslatedEngine::new(start);
    let irq_count = layout_var("irq_count").unwrap();
    for n in 1..=50 {
        e.pass(amiga_oracle::FrameInput::NONE);
        assert_eq!(e.stop(), None, "stopped in pass {n}");
        // One level 3 interrupt, so one pass of `game_frame`, a frame.
        assert_eq!(e.probe(irq_count), first + n, "irq_count after frame {n}");
    }
}

/// The case runs to its end on the translated engine; whether it passes is T30's.
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; the original from power-on twice"]
fn w1_idle_runs_on_the_translated_engine() {
    generated();
    let case = w1_idle();
    let mut e = TranslatedEngine::new(case_start(&case).unwrap_or_else(|e| panic!("{e}")));
    let verdict = run_with(&case, &mut e, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    eprintln!(
        "w1-idle on the translated engine, {} passes: {verdict}",
        e.passes()
    );
    assert_eq!(e.stop(), None, "the translated program stopped");
    assert!(e.passes() >= 190, "{} passes run", e.passes());
}
