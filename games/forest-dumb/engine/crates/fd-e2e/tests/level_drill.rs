//! T29, the level drill (DESIGN §1, §2): the finished W1-1 used in the real `fd-app` as a person
//! would, through the script layer's real input path, and judged against the original.
//!
//! - Each enemy, mechanic and death stretch the cases hold (`engine/conform/cases/`), played as
//!   key presses on the default keys with the recorder on from the first step: the app's state equals
//!   the headless game's every step, the zip it saves replays to the same states, and sampled frames
//!   are judged against the original's picture within the owner's tolerance. `death-spikes` loses
//!   every life to the game over screen; fire restarts. Each sampled frame, the app's and the
//!   original's, is saved to the scratch folder (`$TMPDIR/fd-level-drill`) for a look.
//! - The one route (T84), recorded from launch through "Level 1 complete", fire's restart and a
//!   second run: the second run's states are the first's, step for step.
//! - What a person tries that no task listed: every key held at once, opposite keys, focus lost
//!   mid-jump, the window resized mid-play, a pad unplugged with a button held.
//!
//! Script layer, not the OS layer the task doc names: the cases and the route need each key on its
//! exact step, which real key events cannot give; `drill_os.rs` (T68) holds the real windows and
//! real keys, focus lost to another app included. All but the last are full (`--ignored`).

mod common;

use std::path::{Path, PathBuf};

use amiga_conform::compare::{PictureClass, Tolerance, classify};
use amiga_input::{FrameInput, Joy};
use common::{DOWN, FIRE, LEFT, RIGHT, UP};
use fd_conform::sync::{LEVEL_1_SYNC, reach_level_1};
use fd_conform::{Case, Inputs, PLAY_WINDOW};
use fd_core::{Event, Game, Scene};
use fd_e2e::{App, Captured, LaunchOpts};

/// Where the drill's pictures go: the disks' graphics, so never the repo.
fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join("fd-level-drill");
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The app in a temporary home, bugs into `<home>/test-bugs`.
fn launch(max_frames: u64, timeout_s: u64, extra: &[&str]) -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s,
        extra_args: extra.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    (app, bugs)
}

/// The one zip the run saved, as the app wrote it.
fn the_recording(bugs: &Path) -> fd_core::recording::Recording {
    let mut zips: Vec<PathBuf> = std::fs::read_dir(bugs)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    zips.retain(|p| p.extension().is_some_and(|x| x == "zip"));
    assert_eq!(zips.len(), 1, "one recording saved: {zips:?}");
    let bytes = std::fs::read(&zips[0]).unwrap();
    fd_conform::bug::open_zip(&bytes, fd_tools::ENGINE_BUILD)
        .unwrap_or_else(|e| panic!("{}: {e}", zips[0].display()))
        .0
}

/// The headless game stepped on what the app was fed, its hash checked against the app's every
/// step; `each` sees the step and the game after it.
fn same_as_headless(c: &Captured, steps: u64, mut each: impl FnMut(u64, &mut Game)) {
    let hashes = common::hashes(c);
    assert_eq!(
        hashes.len() as u64,
        steps + 1,
        "a hash for the start and every step"
    );
    let fed = common::inputs(c);
    let mut g = common::headless();
    assert_eq!(hashes[0], common::hash(&g), "the start");
    for n in 1..=steps {
        g.step(fed.get(&n).copied().unwrap_or(FrameInput::NONE));
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "the state after step {n}"
        );
        each(n, &mut g);
    }
}

/// The recording replayed headless passes through the app's states: its frame `k` is the app's
/// step `from + k`.
fn replays_as_played(c: &Captured, bugs: &Path, from: u64, steps: u64) {
    let rec = the_recording(bugs);
    assert_eq!(
        rec.inputs.len() as u64,
        steps,
        "steps {from} to {}",
        from + steps - 1
    );
    let hashes = common::hashes(c);
    let mut seen = 0u64;
    fd_conform::bug::replay(common::data(), &rec, |k, g, _| {
        let n = from + k as u64;
        assert_eq!(
            hashes[n as usize],
            common::hash(g),
            "the replay's frame {k}, the app's step {n}"
        );
        seen += 1;
    });
    assert_eq!(seen, steps);
}

/// A case's inputs from the app's start (the level sync point) to its last frame: entry `k` is
/// what the app's step `k + 1` sees, as the case runner feeds oracle frame `LEVEL_1_SYNC + k`.
fn case_inputs(name: &str) -> (Case, Vec<FrameInput>) {
    let case =
        Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"));
    let inputs = Inputs::load(&case).unwrap_or_else(|e| panic!("{e}"));
    let n = case.sync_frame - LEVEL_1_SYNC + case.frames;
    (case, (0..n).map(|k| inputs.at(LEVEL_1_SYNC + k)).collect())
}

/// `key_lines` moved `by` steps later.
fn shifted(lines: &str, by: u64) -> String {
    lines
        .lines()
        .map(|l| {
            let (n, rest) = l.split_once(": ").unwrap();
            format!("{}: {rest}\n", n.parse::<u64>().unwrap() + by)
        })
        .collect()
}

/// The steps sampled for the picture: eight spread over the run and its last.
fn samples(steps: u64) -> Vec<u64> {
    let mut v: Vec<u64> = (1..=8).map(|k| steps * k / 9).collect();
    v.push(steps);
    v
}

/// One sample judged: the original's frame `step` shifted by `shift` gave `class`.
#[derive(Debug)]
struct Judged {
    step: u64,
    shift: i64,
    class: PictureClass,
}

/// The recording judged against the original as a bug zip is (`fd-conform bug compare`, T15):
/// its inputs retimed by pass onto the original, every variable and the picture compared by the
/// level case runner within the owner's tolerance, diff pictures into the scratch folder. A script
/// written for the original cannot be fed to it frame for frame instead: the app feeds a step's
/// input to its pass, the original reads it when its pass gets there, and a run drifts apart.
///
/// Then each sampled step's picture beside the original's, on the retimed inputs, judged on its
/// own: the best of the original's frames two either side, since a frame can show the pass before
/// or after (`engine/conform/README.md`, "The picture"). Both go to the scratch folder.
fn judge_against_original(
    name: &str,
    rec: &fd_core::recording::Recording,
    pictures: &[(u64, Game)],
) -> (fd_conform::bug::Compared, Vec<Judged>) {
    let dir = scratch().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let compared = fd_conform::bug::compare(common::data(), rec, Some(dir.clone()))
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    std::fs::write(dir.join("compare.txt"), compared.report()).unwrap();
    let script = &compared.retimed.script;
    let last = pictures.iter().map(|(n, _)| *n).max().unwrap_or(0) + 4;
    let mut am = reach_level_1(last as u32 + 4).unwrap_or_else(|e| panic!("{e}"));
    let mut theirs = std::collections::BTreeMap::new();
    for k in 0..last {
        am.step(amiga_conform::script::input_at(&script.lines, k as u32))
            .unwrap_or_else(|e| panic!("{e}"));
        theirs.insert(k + 1, am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1));
    }
    let tol = Tolerance::OWNER;
    let rank = |c: &PictureClass| match c {
        PictureClass::Same => 0,
        PictureClass::ColourStep(_) => 1,
        PictureClass::FewPixels(_) => 2,
        PictureClass::Glitch(n) => 3 + *n as u64,
    };
    let mut out = Vec::new();
    for (n, g) in pictures {
        let ours = g.picture();
        let (shift, class) = (-2i64..=2)
            .filter_map(|d| {
                let t = theirs.get(&u64::try_from(*n as i64 + d).ok()?)?;
                Some((d, classify(&ours, t, &[], &tol)))
            })
            .min_by_key(|(d, c)| (rank(c), d.abs()))
            .unwrap();
        let t = &theirs[&((*n as i64 + shift) as u64)];
        std::fs::write(
            dir.join(format!("{n:05}-app.png")),
            fd_conform::bug::picture_png(&ours),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{n:05}-original.png")),
            fd_conform::bug::frame_png(t),
        )
        .unwrap();
        out.push(Judged {
            step: *n,
            shift,
            class,
        });
    }
    (compared, out)
}

/// What a case's run in the app gave.
struct Played {
    c: Captured,
    /// The headless game after each sampled step.
    pictures: Vec<(u64, Game)>,
    /// The headless game's events, with their step.
    events: Vec<(u64, Event)>,
    /// The headless game after the last step.
    last: Game,
    /// The zip's recording.
    rec: fd_core::recording::Recording,
}

/// A case's stretch played in the app as keys, the recorder on from step 1, then `after`'s lines
/// (steps counted from the app's start) for `extra` steps more, then Stop and Save. Unpaced to 30
/// steps before the end. The app's state equals the headless game's every step, the sampled frames
/// are the headless game's pictures, and the zip replays through the same states.
fn play_case(name: &str, after: &str, extra: u64) -> Played {
    let (_, inputs) = case_inputs(name);
    let steps = inputs.len() as u64;
    let stop = steps + extra + 1;
    let total = stop + 5;
    // Start Level at step 1 restarts the level after step 0, the start itself: the run is the
    // case's from the sync point, recorded from its first step.
    let script = format!(
        "fast-until {}\n1: menu start-level\nform start\n{}{after}{stop}: menu stop-recording\nnote {name}\n",
        stop.saturating_sub(30),
        common::key_lines(&inputs),
    );
    let (mut app, bugs) = launch(total, 600, &[]);
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{name}: {}", c.stderr);
    assert!(
        c.stderr
            .contains("fd-app: started W1-1 after step 0, recording"),
        "{}",
        c.stderr
    );
    let want = samples(steps);
    let (mut pictures, mut events) = (Vec::new(), Vec::new());
    let mut last = None;
    same_as_headless(&c, total, |n, g| {
        events.extend(g.take_events().into_iter().map(|e| (n, e)));
        if want.contains(&n) {
            assert_eq!(
                c.read_frame(n),
                common::picture(g),
                "{name}: the app's frame {n}"
            );
            pictures.push((n, g.clone()));
        }
        if n == total {
            last = Some(g.clone());
        }
    });
    replays_as_played(&c, &bugs, 1, stop - 1);
    let rec = the_recording(&bugs);
    drop(app);
    Played {
        c,
        pictures,
        events,
        last: last.unwrap(),
        rec,
    }
}

/// The cases each enemy, mechanic and death of W1-1 is met in, and the coins.
const STRETCHES: [&str; 9] = [
    "collect-ledge",
    "enemy-contact",
    "enemy-hop",
    "enemy-route",
    "enemy-kind4",
    "mechanic-spring",
    "mechanic-vanish",
    "death-water",
    "death-spikes",
];

#[test]
#[ignore = "full: nine stretches in the app and on the original, a few minutes; cargo test -q -p fd-e2e --test level_drill -- --ignored"]
fn each_enemy_mechanic_and_death_played_recorded_and_judged_against_the_original() {
    let mut differs = Vec::new();
    for name in STRETCHES {
        // death-spikes' own fire ends the game over screen (its step 4470), where the original
        // enters its high scores and the app restarts the level (DESIGN §2.4); fire again after it
        // is a press in the new level, and play goes on.
        let (after, extra) = if name == "death-spikes" {
            let s = case_inputs(name).1.len() as u64;
            (
                format!("{}: down AltLeft\n{}: up AltLeft\n", s + 30, s + 33),
                100,
            )
        } else {
            (String::new(), 0)
        };
        let p = play_case(name, &after, extra);
        assert!(
            !p.events
                .iter()
                .any(|(_, e)| matches!(e, Event::Unported(..))),
            "{name}: {:?}",
            p.events
        );
        if name == "death-spikes" {
            assert!(p.c.stderr.contains("fd-app: game over\n"), "{}", p.c.stderr);
            assert!(
                p.events.iter().any(|(_, e)| *e == Event::Restart),
                "{:?}",
                p.events
            );
            assert_eq!(*p.last.scene(), Scene::Level);
            assert_eq!(
                (p.last.lives(), p.last.state().score),
                (32, 0),
                "lives full, score zero"
            );
        }
        let (compared, judged) = judge_against_original(name, &p.rec, &p.pictures);
        eprintln!("{name}: {}", compared.report().lines().next().unwrap_or(""));
        if let Err(e) = held_to_the_original(name, &compared) {
            differs.push(e);
        }
        // The samples are for the eye (the scratch folder): the bar allows a lone frame past the
        // tolerance, and the compare above judged every frame.
        for j in judged {
            eprintln!("{name}: step {} shift {} {:?}", j.step, j.shift, j.class);
        }
    }
    assert!(
        differs.is_empty(),
        "the app's runs against the original: {differs:#?}"
    );
}

/// A drill recording's compare passes: same; or differing only from the first pass that read in the
/// frame of the pass before, where the original read other input than the app fed (`Lagged`, T15:
/// "a difference from there on may be the lag's"); or, for `death-spikes`, judged until the
/// original left play for its game over screen, which a compare does not judge (T15).
fn held_to_the_original(name: &str, c: &fd_conform::bug::Compared) -> Result<(), String> {
    let lag = c.retimed.lagged.first().map(|l| l.step);
    if let Some((_, step, _)) = c.first()
        && !lag.is_some_and(|l| step >= l)
    {
        return Err(format!("{name}: {}", c.report()));
    }
    if c.first().is_none()
        && let Some(left) = c.left()
        && !(name == "death-spikes" && left.what.contains("left play") && left.step >= 4400)
    {
        return Err(format!("{name}: {}", c.report()));
    }
    Ok(())
}

/// The route's steps (T84); `main_loop` finds the level's end in the last.
const END: u64 = 22464;
/// Start Level, recorder on, at this step: the route runs from the level start after it.
const START: u64 = 50;
/// Steps of the route played again after fire's restart.
const AGAIN: u64 = 1500;

#[test]
#[ignore = "full: the whole route recorded and a second run, about 1 min (debug); cargo test -q -p fd-e2e --test level_drill -- --ignored"]
fn the_route_recorded_through_the_level_s_end_and_a_second_run_after_the_restart() {
    let route = common::route();
    assert_eq!(route.len() as u64, END);
    // A little play before Start Level, so its restart is a real one.
    // The route's step 1 is the app's step START: Start Level lands before it.
    let end = START + END - 1;
    let fire = end + 40;
    let again = fire + 1;
    let stop = again + AGAIN;
    let total = stop + 5;
    let script = format!(
        "fast-until {}\n5: down ArrowRight\n40: up ArrowRight\n{START}: menu start-level\nform start\n\
         {}{fire}: press AltLeft\n{}{stop}: menu stop-recording\nnote drill route\n",
        stop - 30,
        shifted(&common::key_lines(&route), START - 1),
        shifted(&common::key_lines(&route[..AGAIN as usize]), again - 1),
    );
    let (mut app, bugs) = launch(total, 900, &["--capture-hashes"]);
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains(&format!(
            "fd-app: started W1-1 after step {}, recording",
            START - 1
        )),
        "{}",
        c.stderr
    );
    let said: Vec<&str> = c
        .stderr
        .lines()
        .filter(|l| {
            l.contains("level 1 complete") || l.contains("restarts") || l.contains("game over")
        })
        .collect();
    assert_eq!(
        said,
        [
            "fd-app: level 1 complete",
            "fd-app: fire on the message; W1-1 restarts, lives full, score zero"
        ],
        "{}",
        c.stderr
    );

    // The headless game: the app's first START - 1 steps, then a fresh level as Start Level makes.
    let hashes = common::hashes(&c);
    assert_eq!(hashes.len() as u64, total + 1);
    let fed = common::inputs(&c);
    let at = |n: u64| fed.get(&n).copied().unwrap_or(FrameInput::NONE);
    let mut g = common::headless();
    for n in 1..START {
        g.step(at(n));
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "before Start Level, step {n}"
        );
    }
    let mut g = common::headless();
    let mut first_run = Vec::new();
    let mut events = Vec::new();
    for n in START..=total {
        let i = at(n);
        let want = match n {
            _ if n <= end => route[(n - START) as usize],
            _ if n == fire => FrameInput::joy(Joy::FIRE),
            _ if (again..stop).contains(&n) => route[(n - again) as usize],
            _ => FrameInput::NONE,
        };
        assert_eq!(i, want, "the input step {n} saw");
        g.step(i);
        events.extend(g.take_events().into_iter().map(|e| (n, e)));
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "the state after step {n}"
        );
        if n < START + AGAIN {
            first_run.push(hashes[n as usize].clone());
        }
        if (again..stop).contains(&n) {
            assert_eq!(
                hashes[n as usize],
                first_run[(n - again) as usize],
                "the second run's step {} is the first's",
                n - again + 1
            );
        }
        if n == end - 1 {
            assert_eq!(g.lives(), 32, "no life lost on the way");
        }
    }
    assert_eq!(events, [(end, Event::LevelEnd), (fire, Event::Restart)]);
    replays_as_played(&c, &bugs, START, stop - START);
    // On the original: judged to the level's end, where it loads world 2 and leaves play (T15).
    let rec = the_recording(&bugs);
    drop(app);
    let dir = scratch().join("route");
    std::fs::create_dir_all(&dir).unwrap();
    let compared = fd_conform::bug::compare(common::data(), &rec, Some(dir.clone()))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(dir.join("compare.txt"), compared.report()).unwrap();
    eprintln!("{}", compared.report());
    let lag = compared.retimed.lagged.first().map(|l| l.step);
    if let Some((_, step, _)) = compared.first() {
        assert!(lag.is_some_and(|l| step >= l), "{}", compared.report());
    }
}

#[test]
fn keys_held_together_opposite_keys_focus_lost_mid_jump_a_resize_and_a_pad_unplugged() {
    let frames = 260;
    let script = "\
        10: down ArrowLeft\n10: down ArrowRight\n10: down Space\n10: down ArrowDown\n10: down AltLeft\n\
        40: up AltLeft\n40: up ArrowDown\n\
        60: up ArrowLeft\n60: up ArrowRight\n\
        70: down ArrowDown\n100: blur\n\
        110: down ArrowRight\n120: resize 900 700\n130: shot\n140: resize 320 256\n150: shot\n\
        160: up ArrowRight\n\
        170: pad connect 1\n175: pad down 1 DPadLeft\n175: pad down 1 South\n190: pad disconnect 1\n\
        200: pad connect 1\n210: pad press 1 South\n220: shot\n";
    let mut app = App::launch(LaunchOpts {
        max_frames: frames,
        scale: Some(1),
        ..Default::default()
    });
    let c = app.run_script_unread(script);
    assert!(c.exit.success(), "{}", c.stderr);
    // Every key the game knows held at once reaches it whole, opposite directions included, as
    // Superfrog's app does (DESIGN §2.7; owner, T29). A real stick, and vAmiga's, holds one way an
    // axis, so the original never sees both: a known gap (FINDINGS, T29).
    assert_eq!(
        common::joy(&c, 10, 39),
        [UP | DOWN | LEFT | RIGHT | FIRE; 30]
    );
    assert_eq!(common::joy(&c, 40, 59), [UP | LEFT | RIGHT; 20]);
    assert_eq!(common::joy(&c, 60, 69), [UP; 10]);
    // Focus lost mid-jump with Up and Down held: both let go at once, play goes on.
    assert_eq!(common::joy(&c, 70, 99), [UP | DOWN; 30]);
    assert_eq!(common::joy(&c, 100, 109), [0; 10]);
    // A resize mid-play changes nothing the game sees.
    assert_eq!(common::joy(&c, 110, 159), [RIGHT; 50]);
    // A pad unplugged with a direction and fire held lets go of both; plugged back it holds nothing.
    assert_eq!(common::joy(&c, 175, 189), [LEFT | FIRE; 15]);
    assert_eq!(common::joy(&c, 190, 209), [0; 20]);
    assert_eq!(common::joy(&c, 210, 210), [FIRE]);
    let mut pictures = Vec::new();
    same_as_headless(&c, frames, |n, g| {
        g.take_events();
        if [130, 150, 220].contains(&n) {
            pictures.push((n, common::picture(g)));
        }
    });
    // Each shot shows the whole picture at a whole scale, at the size the window was then.
    for (n, want) in pictures {
        let (frame, shown) = common::shown(&c, n, want.width, want.height);
        assert_eq!(frame, n, "the shot at step {n} shows its own frame");
        assert_eq!(shown, want, "the window at step {n}");
        let window = c.dir.join(format!("window-{n:05}.png"));
        std::fs::copy(
            &window,
            scratch().join(format!("unlisted-window-{n:05}.png")),
        )
        .unwrap();
    }
}
