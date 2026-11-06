//! A bug recording compared with the original (T15, DESIGN §2.8): `fd_conform::bug::compare` on
//! recordings made in code as the app makes them (one input a step from `Game::new`), played on the
//! original from the level sync point with their inputs retimed by pass. Needs the disks and
//! Kickstart, as every case does.

use amiga_conform::script;
use amiga_oracle::{FrameInput, Joy};
use fd_conform::bug::{self, Compared};
use fd_core::recording::{LevelId, Recording, recorder};
use fd_core::{Game, Start};

/// A recording of `inputs`, stopped where the engine is after them, as the app saves one.
fn recording(inputs: &[FrameInput], note: &str) -> Recording {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut r = recorder(LevelId::W1_1, fd_tools::ENGINE_BUILD);
    let mut game = Game::new(data, Start::Level1);
    for &i in inputs {
        r.step(i);
        game.step(i);
    }
    r.finish(game.location(), note.to_string())
        .expect("recording")
}

/// The first `steps` steps of `conform/scripts/<name>.txt`, read as one input a step.
fn script_steps(name: &str, steps: u32) -> Vec<FrameInput> {
    let path = fd_conform::conform_dir()
        .join("scripts")
        .join(format!("{name}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{e}"));
    let s = script::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    (0..steps).map(|k| s.input_at(k)).collect()
}

fn compare_with(rec: &Recording, before_step: impl FnMut(u32, &mut Game)) -> Compared {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let c = bug::compare_with(data, rec, None, before_step).unwrap_or_else(|e| panic!("{e}"));
    println!("{}", c.report());
    c
}

fn compare(rec: &Recording) -> Compared {
    compare_with(rec, |_, _| {})
}

/// The jumps of `character-run-jumps` (a tap, then up held), 200 steps: no creature or bonus comes
/// into view, so every play variable and picture is judged, the engine from `Game::new` (T69).
#[test]
fn a_scripted_recording_stays_in_step_on_the_original() {
    let rec = recording(&script_steps("character-run-jumps", 200), "jumps");
    let c = compare(&rec);
    assert!(c.same(), "{}", c.report());
    assert!(c.report().starts_with("same\n"));
    let v = c.verdict.as_ref().expect("judged");
    assert!(v.passes > 190 && v.pictures > 190, "{}", c.report());
    assert_eq!(c.retimed.passes, 200);
    assert!(c.retimed.lagged.is_empty());
    // The engine runs from its own start, not the original's (T69), and it is the original's.
    assert!(c.start_differs.is_empty(), "{:?}", c.start_differs);
    assert!(
        c.report()
            .contains("the engine's own start equals the original's at the first pass end")
    );
}

/// Running left into the wall and back is lag-free: one frame a pass, every pass reading the
/// recording's own input, and the comparison same.
#[test]
fn a_lag_free_stretch_stays_in_step() {
    let rec = recording(&script_steps("character-run-left", 200), "left");
    let c = compare(&rec);
    assert!(c.same(), "{}", c.report());
    assert_eq!((c.retimed.frames, c.retimed.passes), (200, 200));
    assert!(c.retimed.lagged.is_empty());
    assert!(
        c.report()
            .contains("read the recording's input on each of its 200 passes")
    );
}

/// 4 000 steps of running, jumping and dying both ways: retimed by pass, the original runs a pass
/// a frame over the whole of it and two passes read the joystick in the frame of the pass before
/// (measured 2026-11-06, T15 FINDINGS). The pairing also holds across a read cut by the frame's end
/// (frame 7715).
#[test]
#[ignore = "slow: about 12 s"]
fn lagged_input_is_measured() {
    let moves = [
        Joy::RIGHT,
        Joy::RIGHT | Joy::UP,
        Joy::LEFT,
        Joy::NONE,
        Joy::LEFT | Joy::UP,
        Joy::UP,
        Joy::RIGHT | Joy::FIRE,
        Joy::DOWN,
    ];
    let mut inputs = Vec::new();
    let mut k = 0usize;
    while inputs.len() < 4000 {
        let hold = 7 + (k * 13) % 34;
        let i = FrameInput::joy(moves[(k * 5) % moves.len()]);
        inputs.extend(std::iter::repeat_n(i, hold));
        k += 1;
    }
    let rec = recording(&inputs, "varied");
    let r = bug::retime(&rec).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.left, None);
    assert_eq!(r.passes, inputs.len() as u32);
    assert_eq!(r.frames, r.passes);
    let at: Vec<u32> = r.lagged.iter().map(|l| l.step).collect();
    assert_eq!(at, [1658, 1695], "{:?}", r.lagged);
    for l in &r.lagged {
        assert_ne!(l.recorded, l.read);
    }
}

/// Esc (key `$45`, press and release) gives up on the original: `main_loop` goes to game over and
/// then the high scores, where no pass reads the joystick. The engine does not run `play_keys` yet
/// (T23), so it plays on. The comparison ends where the original left play and says so; it is not
/// a difference.
#[test]
fn a_recording_past_game_over_reports_where_judging_ended() {
    let mut inputs = vec![FrameInput::NONE; 300];
    inputs[40] = FrameInput::key(0x45);
    inputs[41] = FrameInput::key(0x45);
    let c = compare(&recording(&inputs, "gave up"));
    assert!(!c.same());
    assert_eq!(c.first(), None, "{}", c.report());
    let l = c.left().expect("judging ended");
    // `main_loop` acts at the 32-pass check after the press (docs/09).
    assert!((41..=75).contains(&l.step), "{l:?}");
    let line = format!(
        "same up to frame {} (engine step {}), not judged after it\n",
        l.frame, l.step
    );
    assert!(c.report().starts_with(&line), "{}", c.report());
    assert!(c.report().contains("left play"));
}

/// An engine that runs one pass too many at step 120 differs there, at the pass end of that step.
#[test]
fn an_engine_mutation_is_reported_at_its_step() {
    let rec = recording(&script_steps("character-run-jumps", 200), "mutated");
    let c = compare_with(&rec, |step, game| {
        if step == 120 {
            game.step(FrameInput::NONE);
        }
    });
    let (frame, step, what) = c.first().expect("a difference");
    assert_eq!(step, 120, "{what}");
    assert!(what.contains("in the engine"), "{what}");
    let line = format!("differs at frame {frame} (engine step 120): ");
    assert!(c.report().starts_with(&line), "{}", c.report());
}

/// The command line: a zip saved as the app saves one, `fd-conform bug compare <zip> --out DIR`
/// prints the report, writes both pictures at Stop and exits 0 on `same`.
#[test]
fn the_command_compares_a_zip_and_writes_both_pictures() {
    let rec = recording(
        &script_steps("character-run-jumps", 120),
        "jumps by command",
    );
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let last = bug::replay(data, &rec, |_, _, _| {}).last;
    let zip = amiga_record::zip_bytes(&rec, &bug::picture_png(&last)).unwrap();
    let dir = std::env::temp_dir().join(format!("fd-bug-compare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("jumps.zip");
    std::fs::write(&path, zip).unwrap();
    let out = dir.join("out");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_fd-conform"))
        .args(["bug", "compare"])
        .arg(&path)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(stdout.starts_with("same\n"), "{stdout}");
    for png in ["original-stop.png", "engine-stop.png"] {
        let bytes = std::fs::read(out.join(png)).unwrap_or_else(|e| panic!("{png}: {e}"));
        let (w, h, _) = amiga_conform::screenshot::decode_png(&bytes).unwrap();
        assert_eq!((w, h), (last.width as u32, last.height as u32), "{png}");
    }
    let text = std::fs::read_to_string(out.join("original-inputs.txt")).unwrap();
    assert!(text.starts_with("# jumps by command: "), "{text}");
    let script = script::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(script.input_at(0), FrameInput::NONE);
    assert!(script.lines.len() > 2, "{text}");
    // The case it writes, run on the script it writes, is the same: a fix is checked by it.
    let toml = std::fs::read_to_string(out.join("bug-case.toml")).unwrap();
    let case = fd_conform::Case::parse("bug-NAME", &toml).unwrap_or_else(|e| panic!("{e}\n{toml}"));
    assert_eq!(case.input.as_deref(), Some("bug-NAME"));
    // Run on the script in memory: the way to the sync point reads no `input` file.
    let mut case = case;
    case.input = None;
    let inputs = fd_conform::Inputs::with_after(&case, script).unwrap();
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut game = Game::new(data, Start::Level1);
    let v = fd_conform::run_inputs(&case, &inputs, &mut game, &Default::default())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(v.passed(), "{v}\n{toml}");
    std::fs::remove_dir_all(&dir).unwrap();
}
