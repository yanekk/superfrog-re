//! The level engine's entry (T07): fd-core's `Game` starts where the original is at the level sync
//! point, steps idle in step with it, and an object nobody ported fails a case. Needs the disks and
//! the Kickstart: absent, it fails naming where it looked.

use std::path::Path;
use std::process::Command;

use amiga_oracle::FrameInput;
use fd_conform::sync::{LEVEL_1_SYNC, reach_level_1};
use fd_conform::{
    Case, Goal, Options, OracleMem, Outcome, PASS_END, game_data, game_data_in, run, run_with,
};
use fd_core::{Game, LAYOUT, Mem, Object, Start, Transplant, layout_var};

fn idle() -> Case {
    Case::load(Path::new("cases/w1-idle.toml")).unwrap_or_else(|e| panic!("{e}"))
}

fn game() -> Game {
    Game::new(game_data().unwrap_or_else(|e| panic!("{e}")), Start::Level1)
}

/// `Game::new` is the original at pass 1's end, the first pass end after the sync point and where a
/// case transplants (T69): the sync point's frame ends at `char_update`'s start, which `Game::new`
/// runs. `mt_counter` is the title's, so it is the boot script's here and not compared.
#[test]
fn every_layout_variable_equals_the_original_at_the_first_pass_end() {
    let mut am = reach_level_1(4).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(am.frame(), LEVEL_1_SYNC);
    am.watch_write(Some(PASS_END));
    let mem = (0..4)
        .find_map(|_| {
            am.step(FrameInput::NONE).unwrap();
            am.watched_memory()
        })
        .expect("a pass ends within 4 frames of the sync point");
    let theirs = OracleMem(&mem);
    let g = game();
    let irq = layout_var("irq_count").unwrap();
    assert_eq!(theirs.value(irq), 1, "the first pass end is pass 1's");
    let wrong: Vec<String> = LAYOUT
        .iter()
        .filter(|v| v.name != "mt_counter" && g.probe(v) != theirs.value(v))
        .map(|v| {
            format!(
                "{}: engine {:#x}, original {:#x}",
                v.name,
                g.probe(v),
                theirs.value(v)
            )
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
    // Not equal by accident: pass 1 starts the character's fall.
    assert_eq!(theirs.value(layout_var("char_falling").unwrap()), 1);
}

#[test]
fn the_idle_case_passes_on_the_level_engine() {
    let case = idle();
    assert_eq!(case.goal, Some(Goal::Idle));
    let v = run(&case).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // Every pass after the transplant's own, one a frame, none unseen.
    assert_eq!((v.passes, v.unseen), (case.frames - 1, 0), "{v}");
    // countdown loses one at passes 441 and 504 of play, both inside the stretch, compared since
    // T23 ported `countdown_step`.
    assert!(case.vars.iter().any(|v| v == "countdown"), "{v}");
    assert!(v.excluded.is_empty(), "{v}");
    assert!(v.events.is_empty(), "{v}");

    // The command line runs it too, and says so.
    let out = Command::new(env!("CARGO_BIN_EXE_fd-conform"))
        .args(["run", "cases/w1-idle.toml"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("same on all 199 passes"), "{text}");
}

#[test]
fn an_unported_object_fails_a_case_whatever_its_goal() {
    // 20 frames from 6400: `countdown` next changes at frame 6426, so it is compared, not excluded.
    let base = idle();
    let short = Case {
        frames: 20,
        exclude: Vec::new(),
        vars: [base.vars.clone(), vec!["countdown".into()]].concat(),
        ..base
    };
    let all = Case {
        goal: Some(Goal::All {
            all: vec![("lives".into(), 0x32)],
            by_frame: short.sync_frame + 20,
        }),
        ..short.clone()
    };
    for case in [short, all] {
        // The same case passes without the object.
        let v = run_with(&case, &mut game(), &Options::default()).unwrap();
        assert_eq!(v.outcome, Outcome::Same, "{v}");
        let mut g = game();
        g.spawn(Object::at_cell(0x5a, 10, 20));
        let v = run_with(&case, &mut g, &Options::default()).unwrap();
        assert!(
            matches!(
                v.outcome,
                Outcome::Unported {
                    pass: 1,
                    value: 0x5a,
                    ..
                }
            ),
            "{v}"
        );
        assert!(v.to_string().contains("unported object value 0x5a"), "{v}");
    }
}

#[test]
fn missing_data_reports_and_does_not_panic() {
    let dir = std::env::temp_dir().join(format!("fd-level-entry-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let err = game_data_in(&dir).unwrap_err();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(err.contains(&dir.display().to_string()), "{err}");
}
