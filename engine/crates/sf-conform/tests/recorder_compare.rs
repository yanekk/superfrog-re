//! `sf-conform bug compare` and `bug case` (bug-recorder T05, DESIGN §2.8, §2.9 as T00 amended
//! it): existing routes played on the engine unpaced from `Game::start_level`, as the app records
//! them, retimed onto the original's frames and judged in legs. Needs the disks and the oracle, as
//! every sf-conform test does.

use sf_conform::bug::{self, LEG_PASSES};
use sf_conform::{InputScript, Mode, case::script_path};
use sf_core::input::FrameInput;
use sf_core::layout::{VarId, def};
use sf_core::recording::{LevelSetup, Recording};
use sf_core::{Game, Rng, Scene};

/// `script`'s first `frames` frames from `setup`'s start, as a recording.
fn recording(setup: LevelSetup, script: &str, frames: u32) -> Recording {
    let s = InputScript::load(&script_path(script)).unwrap();
    Recording {
        build: "test".into(),
        setup,
        seed: Rng::default(),
        stopped_at: None,
        note: format!("{script}, built by recorder_compare.rs"),
        inputs: (0..frames).map(|f| s.input_at(f)).collect(),
    }
}

fn w1_1() -> LevelSetup {
    LevelSetup::default()
}

/// W1-1's exit forced at frame 0 on every side, as `chain-w1-1-to-w1-2`'s sync point forces it.
fn exit_forced() -> Vec<(u32, Vec<u8>)> {
    vec![(sf_core::layout::h0(0xa06), 1u32.to_be_bytes().to_vec())]
}

fn same(c: &bug::Compared) {
    assert!(c.same(), "{}", c.report());
    assert!(c.report().starts_with("same\n"), "{}", c.report());
}

/// Lag-free: W1-1's first 500 frames of `frog-run-right` stay on the recording every frame and
/// compare same, every leg. The test block's short one.
#[test]
fn a_short_run_compares_same() {
    let t0 = std::time::Instant::now();
    let rec = recording(w1_1(), "frog-run-right", 500);
    let c = bug::compare(&rec, Mode::Faithful).unwrap();
    eprintln!("{}\n{:.0?}", c.report(), t0.elapsed());
    same(&c);
    assert_eq!(c.retimed.left, None, "{}", c.report());
    assert!(c.legs.len() > 10, "{} legs", c.legs.len());
    assert_eq!(c.retimed.frames, 500);
    let stop = c
        .retimed
        .stop
        .as_ref()
        .expect("the original's picture at STOP");
    assert!(stop.width > 0);
}

/// The runner's engine perturbed mid-run (100 points added to `score` at the first frame from
/// 400 not in the middle of a pass, in a stretch the original does not lag: after a lag the
/// runner re-syncs the engine, RE §2.1; a poked `frog_x` is put back by the frog's own step):
/// differs, at the pass end after it, naming `score`; that leg ends at the next pass end, and the leg
/// after is re-synced from the original and same again.
#[test]
fn a_perturbed_engine_differs_at_that_frame_and_variable() {
    let rec = recording(w1_1(), "frog-run-right", 500);
    let score = def(VarId::Score).addr;
    let mut planted = None;
    let c = bug::compare_with(&rec, Mode::Faithful, &[], |f, g| {
        if f >= 400 && planted.is_none() && !g.mid_pass() {
            planted = Some(f);
            let at = g.peek(score, 4);
            g.poke(
                score,
                &(u32::from_be_bytes([at[0], at[1], at[2], at[3]]) + 100).to_be_bytes(),
            );
        }
    })
    .unwrap();
    let r = c.report();
    eprintln!("{r}");
    let f = planted.expect("planted");
    assert!(!c.same(), "{r}");
    let d = c.first().unwrap();
    // Seen at the pass's end that follows.
    assert!((f..f + 3).contains(&d.frame), "planted at {f}: {r}");
    let g = d.frame;
    assert_eq!(c.at(d), (g, g), "{r}");
    assert!(
        r.starts_with(&format!(
            "differs at frame {g} (engine step {g}): variable score"
        )),
        "{r}"
    );
    let bad: Vec<_> = c.legs.iter().filter(|l| l.first.is_some()).collect();
    assert_eq!(bad.len(), 1, "{r}");
    // The leg that differs ends at the next pass end (a pass is 2 frames here), not after its 8.
    assert!(bad[0].to <= g + 2, "the leg differing at {g} ran to {}: {r}", bad[0].to);
    assert!(c.legs.last().unwrap().first.is_none(), "{r}");
}

/// `bug case` writes a case that parses and that `run` passes (its script swapped for the scratch
/// folder's path, so the test leaves nothing in the repo), and refuses its name a second time.
#[test]
fn a_case_from_a_recording_runs_green_and_its_name_is_taken_once() {
    let mut rec = recording(w1_1(), "frog-run-right", 300);
    rec.note = "the frog\nruns".into();
    let dir = std::env::temp_dir().join(format!("sf-recorder-compare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (cases, scripts) = (dir.join("cases"), dir.join("scripts"));
    std::fs::create_dir_all(&cases).unwrap();
    std::fs::create_dir_all(&scripts).unwrap();
    let (case, script) = bug::write_case(&rec, "bug-run", "run.zip", &cases, &scripts).unwrap();
    let toml = std::fs::read_to_string(&case).unwrap();
    assert!(
        toml.starts_with("# the frog runs (stopped at -)\n"),
        "{toml}"
    );
    assert!(toml.contains("\nscript = \"bug-run\"\n"), "{toml}");
    let swapped = toml.replace(
        "script = \"bug-run\"",
        &format!("script = {:?}", script.display().to_string()),
    );
    let c = sf_conform::Case::parse("bug-run", &swapped).unwrap();
    assert_eq!(c.frames, 300);
    assert!(matches!(c.sync, sf_conform::SyncPoint::Setup { .. }));
    assert_eq!(c.script.input_at(0), rec.inputs[0]);
    sf_conform::run(&c).unwrap_or_else(|d| panic!("the case: {d}"));
    let e = bug::write_case(&rec, "bug-run", "run.zip", &cases, &scripts).unwrap_err();
    assert!(e.contains("already there"), "{e}");
    assert!(bug::write_case(&rec, "Bug Run", "run.zip", &cases, &scripts).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The engine's unpaced steps, each input chosen by `input(step, game)`, as a recording: the app's.
fn played(
    setup: LevelSetup,
    steps: u32,
    mut input: impl FnMut(u32, &Game) -> FrameInput,
) -> Recording {
    let mut g = Game::start_level(sf_conform::display_data(), &setup, Rng::default());
    let mut inputs = Vec::new();
    for n in 0..steps {
        let i = input(n, &g);
        g.step(i);
        inputs.push(i);
    }
    Recording {
        build: "test".into(),
        setup,
        seed: Rng::default(),
        stopped_at: Some(g.location()),
        note: "built by recorder_compare.rs".into(),
        inputs,
    }
}

/// `chain-w1-1-to-w1-2`'s inputs (`tally-points`, 1150 frames) unpaced from `start_level` with W1-1's
/// exit forced: the tally, W1-2's load and 500 frames of W1-2, same.
#[test]
#[ignore = "slow: about 20 s (release; the original's start cached)"]
fn chain_w1_1_to_w1_2_compares_same() {
    let t0 = std::time::Instant::now();
    let rec = recording(w1_1(), "tally-points", 1150);
    let c = bug::compare_with(&rec, Mode::Faithful, &exit_forced(), |_, _| {}).unwrap();
    eprintln!("{}\n{:.0?}", c.report(), t0.elapsed());
    same(&c);
    assert_eq!(c.retimed.left, None, "{}", c.report());
}

/// The acceptance stretch (the owner's black screen, T13): W1-1's exit (forced), the tally
/// choosing the slot machine (`tally-slot`), `slot-gamble`'s play from the slot machine's entry,
/// W1-2's intro and 500 frames of W1-2, same.
#[test]
#[ignore = "slow: about 75 s (release; the slot machine's loads on the original)"]
fn w1_1_through_the_slot_machine_to_w1_2_compares_same() {
    let t0 = std::time::Instant::now();
    let tally = InputScript::load(&script_path("tally-slot")).unwrap();
    let gamble = InputScript::load(&script_path("slot-gamble")).unwrap();
    // `slot-gamble`'s frames count from its sync point, 496 of its frames before the engine's
    // slot scene begins (T00).
    const GAMBLE_FROM: u32 = 496;
    let (mut slot, mut w1_2) = (None, None);
    let setup = w1_1();
    let mut g = Game::start_level(sf_conform::display_data(), &setup, Rng::default());
    for (a, b) in exit_forced() {
        g.poke(a, &b);
    }
    let mut inputs = Vec::new();
    let mut n = 0u32;
    while w1_2.is_none_or(|s| n < s + 500) {
        if slot.is_none() && matches!(g.scene(), Scene::Slot { .. }) {
            slot = Some(n);
        }
        if w1_2.is_none() && slot.is_some() && g.progress().level == 2 && g.scene() == Scene::Level
        {
            w1_2 = Some(n);
        }
        let i = match slot {
            None => tally.input_at(n),
            Some(s) => gamble.input_at(n - s + GAMBLE_FROM),
        };
        g.step(i);
        inputs.push(i);
        n += 1;
        assert!(n < 6_000, "W1-2 not reached");
    }
    eprintln!("slot at step {slot:?}, W1-2 at {w1_2:?}, {n} steps");
    let rec = Recording {
        build: "test".into(),
        setup,
        seed: Rng::default(),
        stopped_at: Some(g.location()),
        note: "built by recorder_compare.rs".into(),
        inputs,
    };
    let c = bug::compare_with(&rec, Mode::Faithful, &exit_forced(), |_, _| {}).unwrap();
    eprintln!("{}\n{:.0?}", c.report(), t0.elapsed());
    same(&c);
    assert_eq!(c.retimed.left, None, "{}", c.report());
}

/// The long-lag level T00 used: `complete-w4-3` played unpaced as a recording. The original's lag
/// takes it off the recording (T00: by frame 400), which the report names; every leg is same.
#[test]
#[ignore = "slow: about 2 min (release; 9 200 steps of W4-3)"]
fn w4_3_under_lag_leaves_the_recording_and_every_leg_is_same() {
    let t0 = std::time::Instant::now();
    let setup = LevelSetup {
        level: "W4-3".parse().unwrap(),
        ..LevelSetup::default()
    };
    let s = InputScript::load(&script_path("complete-w4-3")).unwrap();
    let rec = played(setup, 9200, |n, _| s.input_at(n));
    let c = bug::compare(&rec, Mode::Faithful).unwrap();
    let r = c.report();
    eprintln!("{r}\n{:.0?}", t0.elapsed());
    let left = c
        .retimed
        .left
        .as_ref()
        .expect("the original leaves the recording");
    assert!(
        r.contains(&format!("left the recording at frame {}", left.frame)),
        "{r}"
    );
    same(&c);
    assert!(c.legs.len() as u64 > 9200 / LEG_PASSES / 4, "{r}");
}

/// The real binary: `bug compare` on a zip prints `same` first, exits 0 and writes both STOP
/// pictures beside the zip; `bug case` refuses a name the repo's cases already have, writing
/// nothing.
#[test]
fn the_cli_compares_a_zip_and_refuses_a_taken_case_name() {
    use sf_core::recording::{ZIP_RECORDING, ZIP_SCREENSHOT};
    let rec = recording(w1_1(), "frog-run-right", 200);
    let dir = std::env::temp_dir().join(format!("sf-recorder-compare-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let png = bug::picture_png(&sf_core::Picture::blank(0, 0, 4, 2));
    let zip = dir.join("run.zip");
    std::fs::write(
        &zip,
        sf_disk::zip::write_stored(&[
            (ZIP_RECORDING, rec.to_text().as_bytes()),
            (ZIP_SCREENSHOT, &png),
        ]),
    )
    .unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_sf-conform"))
            .args(args)
            .output()
            .unwrap()
    };
    let o = run(&["bug", "compare", zip.to_str().unwrap()]);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        o.status.success(),
        "{out}{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(out.starts_with("same\n"), "{out}");
    for name in ["original-stop.png", "engine-stop.png"] {
        let b = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(b.starts_with(b"\x89PNG"), "{name}");
    }
    let o = run(&["bug", "case", zip.to_str().unwrap(), "frog-run-w1-1"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("already there"));
    std::fs::remove_dir_all(&dir).unwrap();
}
