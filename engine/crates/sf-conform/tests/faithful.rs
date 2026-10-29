//! The faithful checks (DESIGN §2.1 checks 1–3, T180) on Superfrog: the state check's variables,
//! and on the oracle that T120's guards still hold and that the loosening did not blind the checks.
//! The oracle tests need what `runner.rs` needs. The tolerances on their own, on synthetic
//! pictures and writes, are `toolkit/crates/amiga-conform/tests/faithful.rs` (toolkit-extract T06).

use sf_conform::compare::{gameplay_vars, vars};
use sf_conform::{Case, Mode, Tolerance};
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarId, def};

/// Superfrog's bar is the toolkit's, value for value as it was set (rust-engine DESIGN §2.1).
#[test]
fn the_owners_tolerance_is_the_one_superfrog_judges_by() {
    assert_eq!(
        Tolerance::OWNER,
        Tolerance {
            colour_step: 1,
            few_pixels: 64,
            glitch_frames: 2,
            glitch_budget_pct: 1,
            sound_frames: 2,
        }
    );
    assert_eq!(sf_conform::channels::OWNER, amiga_conform::channels::OWNER);
}

/// Memory covering every `LAYOUT` variable, all zero.
fn memory() -> AmigaMemory {
    AmigaMemory {
        chip: vec![0; 512 * 1024],
        slow: vec![0; 512 * 1024],
    }
}

fn probe(m: &AmigaMemory) -> Vec<(VarId, u32)> {
    LAYOUT
        .iter()
        .flat_map(|d| d.pieces())
        .map(|p| (p.id, p.read(m as &dyn Ram)))
        .collect()
}

#[test]
fn a_presentation_variable_differing_is_not_a_state_difference() {
    let case = Case::parse("state", "sync = \"power-on\"\nframes = 1").unwrap();
    let theirs = memory();
    let mut engine = memory();
    engine.chip[def(VarId::LevelCopper).addr as usize + 6] = 0x55;
    engine.chip[def(VarId::PlayfieldBuffer1).addr as usize + 100] = 0x0f;
    let p = probe(&engine);
    assert!(
        vars(LAYOUT, &p, &theirs, &[]).is_some(),
        "frame-exact sees it"
    );
    assert_eq!(gameplay_vars(&case, LAYOUT, &p, &theirs, &[]), None);
    // The frog's x is gameplay.
    let x = def(VarId::FrogX).addr - sf_core::layout::SLOW_BASE;
    engine.slow[x as usize + 1] = 1;
    let (d, t, o) = gameplay_vars(&case, LAYOUT, &probe(&engine), &theirs, &[]).unwrap();
    assert_eq!((d.name, t, o), ("frog_x", 0, 1));
}

/// T120's guard holds in faithful mode: an original held mid-pass past the limit fails the case
/// rather than leaving it green comparing nothing (`items-run-w1-1`'s longest run, 20 from 301,
/// its objects all live since T21).
#[test]
fn a_faithful_case_fails_once_the_original_stays_mid_pass_past_its_limit() {
    let full = Case::load(&sf_conform::cases_dir().join("items-run-w1-1.toml")).unwrap();
    assert_eq!(full.mode, Mode::Faithful);
    let pinned = |frames: u32| Case {
        frames: 330,
        lag_limit: Some(sf_conform::LagLimit {
            frames,
            note: "test".into(),
        }),
        ..full.clone()
    };
    let o = sf_conform::run(&pinned(20)).unwrap_or_else(|d| panic!("{d}"));
    assert_eq!((o.mode, o.longest_mid_pass), (Mode::Faithful, (301, 20)));
    let d = sf_conform::run(&pinned(19)).expect_err("a run past the limit was green");
    assert_eq!((d.frame, d.lag), (301 + 19, Some((301, 19))), "{d}");
}

/// The loosening did not blind the checks: the frog's jump one unit stronger than the original's
/// fails a frog case in faithful mode, on the frog's state at a pass's end. The original's objects
/// are frozen out here, as the case had them until T21: with them live its first jump falls in
/// the lag of 155–259, where no pass is compared.
#[test]
fn a_jump_off_by_one_fails_a_frog_case_faithfully() {
    let mut case = Case {
        frames: 260,
        ..Case::load(&sf_conform::cases_dir().join("frog-run-w1-1.toml")).unwrap()
    };
    case.exclude.push(sf_conform::case::Exclusion {
        what: sf_conform::case::Excluded::Objects,
        note: "the judge's test: an original with no objects, and no lag".into(),
        task: "T21".into(),
    });
    assert_eq!(case.mode, Mode::Faithful);
    sf_conform::run(&case).unwrap_or_else(|d| panic!("unbroken: {d}"));
    let vy = def(VarId::FrogVy).addr;
    let mut broken = None;
    let d = sf_conform::run_with(&case, |f, game| {
        let v = i16::from_be_bytes(game.peek(vy, 2).try_into().unwrap());
        if broken.is_none() && v < 0 {
            broken = Some(f);
            game.poke(vy, &(v - 1).to_be_bytes());
        }
    })
    .expect_err("a stronger jump was faithful");
    let from = broken.expect("the frog never jumped");
    assert!((from..from + 3).contains(&d.frame), "broken at {from}: {d}");
    let (v, _, _) = d.first_var.unwrap_or_else(|| panic!("{d}"));
    assert!(
        v.name.starts_with("frog_") || v.name.starts_with("camera_"),
        "{d}"
    );
}

/// The title music stopped (`music_on` planted 0) fails the title faithfully, on its sound: the
/// events the original's music makes, the engine does not.
#[test]
fn silence_fails_the_title_on_its_sound() {
    let case = Case {
        compare: sf_conform::Compare {
            paula: true,
            ..sf_conform::Compare::default()
        },
        name: "sound-test".into(),
        ..Case::load(&sf_conform::cases_dir().join("title-rng.toml")).unwrap()
    };
    sf_conform::run(&case).unwrap_or_else(|d| panic!("the music alone: {d}"));
    let d = sf_conform::run_with(&case, |f, game| {
        if f == 3 {
            game.poke(sf_core::layout::h0(0x13420), &[0, 0]);
        }
    })
    .expect_err("silence matched the title music");
    let s = d.sound_diff.clone().unwrap_or_else(|| panic!("{d}"));
    assert!(s.contains("channel") && s.contains("the engine"), "{s}");
    assert!((3..=20).contains(&d.frame), "{d}");
}

