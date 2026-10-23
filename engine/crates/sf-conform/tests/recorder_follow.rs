//! The original kept on a recording it leaves (bug-recorder T14): under lag the original stops
//! playing what the owner played; `bug compare` puts it back on the recording (the recording's
//! engine written into its memory at the pass end it is seen to have left) and judges on from the
//! owner's state, and says plainly what it could not judge. Needs the disks and the oracle, as
//! every sf-conform test does.

use sf_conform::bug::{self, Pokes};
use sf_conform::{InputScript, Mode, case::script_path};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{VarId, def, h0};
use sf_core::recording::{LevelSetup, Recording};
use sf_core::{Game, Rng, Scene};

const LEVEL_STATE: u32 = h0(0xa06);

/// The engine's unpaced steps from the Start Level dialog's defaults and seed 0 0, each input
/// chosen by `input(step, game, exit)` (`exit` the step W1-1's exit was set on), until `done`: the
/// app's recording.
fn played(
    mut input: impl FnMut(u32, &Game, Option<u32>) -> FrameInput,
    mut done: impl FnMut(u32, &Game) -> bool,
) -> Recording {
    let setup = LevelSetup::default();
    let seed = Rng::seeded(0, 0);
    let mut g = Game::start_level(sf_conform::display_data(), &setup, seed);
    let mut exit = None;
    let mut inputs = Vec::new();
    let mut n = 0u32;
    while !done(n, &g) {
        // The level runs at −2; the exit sets 1 (the first frame or two are −1, T132).
        if exit.is_none() && n > 10 && (g.state().l(LEVEL_STATE) as i32) >= 0 {
            exit = Some(n);
        }
        let i = input(n, &g, exit);
        g.step(i);
        inputs.push(i);
        n += 1;
        assert!(n < 12_000, "never done");
    }
    Recording {
        build: "test".into(),
        setup,
        seed,
        stopped_at: Some(g.location()),
        note: "built by recorder_follow.rs".into(),
        inputs,
    }
}

/// W1-1 played to its exit on the engine (T11's route, no exit forced).
fn route() -> InputScript {
    InputScript::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../sf-e2e/tests/data/w1-1-exit-engine.txt"),
    )
    .unwrap()
}

/// Steps from W1-2's first level frame, once `then` holds.
fn w1_2_for(frames: u32) -> impl FnMut(u32, &Game) -> bool {
    let mut w1_2 = None;
    move |n, g| {
        if w1_2.is_none() && g.progress().level == 2 && g.scene() == Scene::Level {
            w1_2 = Some(n);
        }
        w1_2.is_some_and(|s| n >= s + frames)
    }
}

/// T12's dry run 1, as T11's drill records it in the app (`sf-e2e/tests/recorder_drill.rs`
/// `chain`): W1-1 to its exit, the tally choosing the slot machine (`tally-slot`), fire every 80
/// frames from 172 after the exit till its coins are spent, then 300 frames of W1-2. 7 487 steps.
pub fn through_the_slot_machine() -> Recording {
    let (route, tally) = (route(), InputScript::load(&script_path("tally-slot")).unwrap());
    let mut slot = false;
    let mut w1_2 = w1_2_for(300);
    played(
        |n, _, exit| match exit {
            None => route.input_at(n),
            Some(e) if n >= e + 172 && (n - e - 172) % 80 == 0 => FrameInput::joy(Joy::FIRE),
            Some(e) if n >= e + 172 => FrameInput::NONE,
            Some(e) => tally.input_at(n - e),
        },
        |n, g| {
            slot |= matches!(g.scene(), Scene::Slot { .. });
            slot && w1_2(n, g)
        },
    )
}

/// T12's dry run 2's recording: W1-1 to its exit, `tally-points` from the exit (the points), then
/// 300 frames of W1-2.
pub fn through_the_points() -> Recording {
    let (route, tally) = (route(), InputScript::load(&script_path("tally-points")).unwrap());
    played(
        |n, _, exit| match exit {
            None => route.input_at(n),
            Some(e) => tally.input_at(n - e),
        },
        w1_2_for(300),
    )
}

/// A recording the original leaves early, short enough for the test block: `complete-w4-3`'s
/// first 700 steps of W4-3 (T05: the original leaves at frame 399 under its lag). It is put back,
/// judged on to the end with no panic, and every leg is same.
#[test]
fn a_recording_the_original_leaves_is_put_back_on_it() {
    let t0 = std::time::Instant::now();
    let setup = LevelSetup {
        level: "W4-3".parse().unwrap(),
        ..LevelSetup::default()
    };
    let s = InputScript::load(&script_path("complete-w4-3")).unwrap();
    let mut g = Game::start_level(sf_conform::display_data(), &setup, Rng::default());
    let inputs: Vec<FrameInput> = (0..700)
        .map(|n| {
            let i = s.input_at(n);
            g.step(i);
            i
        })
        .collect();
    let rec = Recording {
        build: "test".into(),
        setup,
        seed: Rng::default(),
        stopped_at: Some(g.location()),
        note: "complete-w4-3, 700 steps".into(),
        inputs,
    };
    // The judging run's engine on each frame, before it steps: after a put-back it starts from
    // the original written with the recording's engine, so it holds the owner's game there.
    let watched = [VarId::FrogX, VarId::FrogY, VarId::Score, VarId::FrogLives, VarId::FrogHearts];
    let peek = |g: &Game| -> Vec<u32> {
        watched
            .iter()
            .map(|&id| {
                let d = def(id);
                g.peek(d.addr, d.width.len()).iter().fold(0, |v, &b| v << 8 | b as u32)
            })
            .collect()
    };
    let mut judged = Vec::new();
    let c = bug::compare_with(&rec, Mode::Faithful, &[], |_, g| judged.push(peek(g))).unwrap();
    let r = c.report();
    eprintln!("{r}\n{:.0?}", t0.elapsed());
    let p = c.retimed.put_back.first().expect("the original is put back");
    assert!(p.frame < 700, "{r}");
    // No loads in these 700 steps: the judging run's frame after the put-back is the original's.
    let mut owner = None;
    bug::replay(&rec, |k, g, _| {
        if k == p.step {
            owner = Some(peek(g));
        }
    });
    assert_eq!(
        judged[p.frame as usize + 1],
        owner.unwrap(),
        "the judging run is put back on the owner's game at frame {}: {r}",
        p.frame + 1
    );
    assert_eq!(c.retimed.left, None, "{r}");
    assert!(r.contains("was put back on it"), "{r}");
    assert!(r.contains("the original was on the recording to its end"), "{r}");
    assert!(c.same(), "{r}");
    assert!(r.starts_with("same\n"), "{r}");
}

/// A recording the original cannot follow (W1-1's exit forced in the recording's engine alone, a
/// recording the app could not make: the engine goes to the tally and loads, and the original,
/// playing on in W1-1, never begins that load): `bug compare` names the frame it judged up to and
/// why, says nothing is judged after it, is not `same`, and does not panic (T12's dry run 2
/// panicked there).
#[test]
fn a_recording_the_original_leaves_for_good_is_judged_only_up_to_there() {
    // W1-1's exit forced in the recording's engine (the played side) and nowhere else: the
    // original plays on in W1-1, and the engine's load of the tally is one it never ends.
    let rec = {
        let setup = LevelSetup::default();
        let mut g = Game::start_level(sf_conform::display_data(), &setup, Rng::default());
        g.poke(LEVEL_STATE, &1u32.to_be_bytes());
        let s = InputScript::load(&script_path("tally-points")).unwrap();
        let inputs: Vec<FrameInput> = (0..400)
            .map(|n| {
                let i = s.input_at(n);
                g.step(i);
                i
            })
            .collect();
        Recording {
            build: "test".into(),
            setup,
            seed: Rng::default(),
            stopped_at: Some(g.location()),
            note: "W1-1's exit forced in the recording's engine only".into(),
            inputs,
        }
    };
    let exit: &Pokes = &[(LEVEL_STATE, 1u32.to_be_bytes().to_vec())];
    let c = bug::compare_played(&rec, Mode::Faithful, exit).unwrap();
    let r = c.report();
    eprintln!("{r}");
    let left = c.retimed.left.as_ref().expect("the original leaves for good");
    assert!(!c.same(), "{r}");
    assert_eq!(c.first().map(|d| d.frame), None, "{r}");
    assert!(
        r.starts_with(&format!(
            "same up to frame {} (engine step {}), not judged after it\n",
            left.frame, left.step
        )),
        "{r}"
    );
    assert!(r.contains("could not be put back: the engine loads here"), "{r}");
    assert!(c.retimed.frames <= left.frame, "{r}");
}

/// T12's dry run 1 (T14 Tests): the app's recording of W1-1 played to its exit, the tally, the
/// slot machine and W1-2. The original leaves it at frame 1815 under W1-1's lag (`score`); it is
/// put back, and is in W1-2 with the recording's score at STOP, every leg same.
#[test]
#[ignore = "slow: about 4 min (release; 7 487 steps, the slot machine's loads on the original)"]
fn w1_1_played_through_the_slot_machine_is_judged_on_the_recording_to_w1_2() {
    let t0 = std::time::Instant::now();
    let rec = through_the_slot_machine();
    assert_eq!(rec.inputs.len(), 7487);
    let c = bug::compare(&rec, Mode::Faithful).unwrap();
    let r = c.report();
    eprintln!("{r}\n{:.0?}", t0.elapsed());
    assert!(!c.retimed.put_back.is_empty(), "{r}");
    assert_eq!(c.retimed.left, None, "{r}");
    assert!(c.same() && r.starts_with("same\n"), "{r}");
    // At STOP the original is where the recording is: W1-2, the score and lives the same.
    let ids = [VarId::World, VarId::Level, VarId::Score, VarId::FrogLives];
    let theirs: Vec<u32> = ids
        .iter()
        .map(|id| c.retimed.stop_watched.iter().find(|(w, _)| w == id).unwrap().1)
        .collect();
    let ours = bug::replay(&rec, |_, _, _| {}).game;
    let ours: Vec<u32> = ids
        .iter()
        .map(|&id| {
            let d = def(id);
            ours.peek(d.addr, d.width.len())
                .iter()
                .fold(0, |v, &b| v << 8 | b as u32)
        })
        .collect();
    assert_eq!(theirs, ours, "world, level, score, lives at STOP: {r}");
    assert_eq!(ours[1], 2, "W1-2");
}

/// T12's dry run 2 on today's engine: W1-1 to its exit, the tally's points, W1-2. Put back where
/// the original leaves, same; and `bug case` writes a case (with its `put-back`) that runs green.
#[test]
#[ignore = "slow: about 2 min (release; 5 352 steps compared, then the case run)"]
fn w1_1_played_through_the_points_compares_same_and_its_case_runs_green() {
    let t0 = std::time::Instant::now();
    let rec = through_the_points();
    let c = bug::compare(&rec, Mode::Faithful).unwrap();
    let r = c.report();
    eprintln!("{r}\n{:.0?}", t0.elapsed());
    assert!(!c.retimed.put_back.is_empty(), "{r}");
    assert!(c.same(), "{r}");
    let dir = std::env::temp_dir().join(format!("sf-recorder-follow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (cases, scripts) = (dir.join("cases"), dir.join("scripts"));
    std::fs::create_dir_all(&cases).unwrap();
    std::fs::create_dir_all(&scripts).unwrap();
    let written = bug::write_case(&rec, "bug-points", "points.zip", &cases, &scripts).unwrap();
    let [case, script, steps] = &written[..] else {
        panic!("a case, its script and its steps: {written:?}")
    };
    let toml = std::fs::read_to_string(case).unwrap();
    assert!(toml.contains("put-back = { steps = \"bug-points-steps\", at = [["), "{toml}");
    let swapped = toml
        .replace(
            "script = \"bug-points\"",
            &format!("script = {:?}", script.display().to_string()),
        )
        .replace(
            "steps = \"bug-points-steps\"",
            &format!("steps = {:?}", steps.display().to_string()),
        );
    let case = sf_conform::Case::parse("bug-points", &swapped).unwrap();
    let p = case.put_back.as_ref().unwrap();
    assert_eq!(p.at.len(), c.retimed.put_back.len());
    assert_eq!(p.steps.input_at(rec.inputs.len() as u32 - 1), *rec.inputs.last().unwrap());
    sf_conform::run(&case).unwrap_or_else(|d| panic!("the case: {d}"));
    eprintln!("{:.0?}", t0.elapsed());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `SF_T14_ZIPS=<dir> cargo test --release -p sf-conform --test recorder_follow -- --ignored
/// zips`: both dry runs' recordings as bug zips in `<dir>`, for `sf-conform bug compare` and `bug
/// case` by hand (the screenshot is the headless replay's last picture, as the app saves it).
#[test]
#[ignore = "a tool: writes the dry runs' zips into $SF_T14_ZIPS"]
fn zips() {
    use sf_core::recording::{ZIP_RECORDING, ZIP_SCREENSHOT};
    let dir = std::path::PathBuf::from(std::env::var_os("SF_T14_ZIPS").expect("SF_T14_ZIPS"));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, mut rec) in [
        ("bug-W1-2-slot.zip", through_the_slot_machine()),
        ("bug-W1-2-points.zip", through_the_points()),
    ] {
        rec.note = format!("T12 dry run: {name}");
        let png = bug::picture_png(&bug::replay(&rec, |_, _, _| {}).last);
        let zip = sf_disk::zip::write_stored(&[
            (ZIP_RECORDING, rec.to_text().as_bytes()),
            (ZIP_SCREENSHOT, &png),
        ]);
        std::fs::write(dir.join(name), zip).unwrap();
        eprintln!("{} ({} steps)", dir.join(name).display(), rec.inputs.len());
    }
}

/// A case file's `put-back`: read with its steps script; refused beside any sync but a level
/// setup's, with frames out of order, or with a key it does not know.
#[test]
fn a_case_files_put_back_parses_and_refuses_what_it_does_not_know() {
    let setup = "sync = { level = \"W1-1\", lives = 3, points = 0, energy = 4, difficulty = \"normal\", spud = false, wings = false, seed = \"0 0\" }\nframes = 10\n";
    let c = sf_conform::Case::parse(
        "p",
        &format!("{setup}put-back = {{ steps = \"frog-run-right\", at = [[5, 4], [9, 7]] }}\n"),
    )
    .unwrap();
    let p = c.put_back.unwrap();
    assert_eq!(p.at, [(5, 4), (9, 7)]);
    assert_eq!(p.steps, InputScript::load(&script_path("frog-run-right")).unwrap());
    for (text, why) in [
        (
            "sync = \"power-on\"\nframes = 10\nput-back = { steps = \"frog-run-right\", at = [] }\n".to_string(),
            "level-setup sync",
        ),
        (
            format!("{setup}put-back = {{ steps = \"frog-run-right\", at = [[9, 7], [5, 4]] }}\n"),
            "increasing order",
        ),
        (
            format!("{setup}put-back = {{ steps = \"frog-run-right\", at = [[5]] }}\n"),
            "put-back is",
        ),
        (
            format!("{setup}put-back = {{ steps = \"frog-run-right\", at = [], frames = 2 }}\n"),
            "unknown key `frames`",
        ),
    ] {
        let e = sf_conform::Case::parse("p", &text).unwrap_err();
        assert!(e.contains(why), "{text}: {e}");
    }
}
