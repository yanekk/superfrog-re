//! What counts as a completed route (DESIGN §4.1, D9): a case built from the route passes
//! `sf_conform::run`, the faithful check the completion cases pass, and the original's milestones
//! include the level's exit. Its length and the hearts it lost are read off the original replaying
//! it. Set C has no original (T12): the route replays on the engine from the app's Start Level
//! start to the exit with no life lost, and the real app, started the same way by its script layer
//! and fed the route as key presses, keeps the engine's state on every frame to the exit.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use sf_conform::{Case, InputScript, Milestone, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::Joy;
use sf_core::layout::h0;
use sf_core::recording::LevelSetup;
use sf_core::{Game, LevelId, Rng};

use crate::sets::{Set, Start, StartKind};

/// `level_state` (h0+$a06): −2 while the level runs.
const LEVEL_STATE: u32 = h0(0xa06);
/// `frog+$5a` (h0+$830): the hearts.
const HEARTS: u32 = h0(0x830);
/// Nothing held after a route's last line before the run ends: the exit's milestone is seen even
/// for a route that stops on its exit frame.
pub const TAIL: u32 = 25;
/// The level's `level_state` is not yet −2 on its first frames (`plan_exit` waits 10 too).
const SETTLE: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub completed: bool,
    /// Frames from the start to the end of the frame `level_state` left −2 on, on the original.
    pub route_frames: Option<u32>,
    pub hearts_lost: u32,
    /// Empty when completed.
    pub reason: String,
}

impl Verdict {
    pub fn failed(reason: impl Into<String>) -> Verdict {
        Verdict {
            completed: false,
            route_frames: None,
            hearts_lost: 0,
            reason: reason.into(),
        }
    }
}

/// The committed route a level's result is compared with: its standalone completion for sets A
/// and C, its `chain-*` leg for set B (W1-1 and W5-2 play their standalone routes in the chain,
/// as `examples/whole_game` has them).
pub fn committed_script(set: Set, level: &LevelId) -> String {
    let l = level.to_string().to_lowercase();
    let world = l.as_bytes()[1];
    if set == Set::B && !matches!(l.as_str(), "w1-1" | "w5-2") {
        return format!("chain-{l}");
    }
    match world {
        b'1' | b'4' => format!("complete-{l}"),
        b'2' => format!("{l}-complete"),
        _ => format!("enemies-{l}-complete"),
    }
}

/// The sync point a set's start is verified from (§4.1).
pub fn sync_point(s: &Start) -> SyncPoint {
    match &s.kind {
        StartKind::AtLevel | StartKind::App => SyncPoint::AtLevel(s.level),
        StartKind::Snap(_, frame) => SyncPoint::AtFrame {
            script: "game-whole".into(),
            frame: *frame,
            df1: true,
        },
    }
}

/// The case the route is judged by: the start's sync point, the route, `TAIL` frames past its
/// last line, re-sync allowed (§4.1). The lag limit is the one the committed route's own case
/// carries (its level's completion case for A, the whole game's for B), as the completion cases
/// are verified with theirs.
pub fn case(set: Set, s: &Start, script: &InputScript) -> Result<Case, String> {
    let end = script.lines.last().map_or(0, |l| l.0) + TAIL;
    // The case file §4.1 describes, but for the script, which is the route in memory.
    let sync = match sync_point(s) {
        SyncPoint::AtFrame { script, frame, df1 } => {
            format!("{{ script = \"{script}\", frame = {frame}, df1 = {df1} }}")
        }
        _ => format!("{{ level = \"{}\" }}", s.level),
    };
    let text = format!("sync = {sync}\nframes = {end}\nresync = true\n");
    let mut case = Case::parse(&format!("bench-{set}-{}", s.level), &text)?;
    debug_assert_eq!(case.sync, sync_point(s));
    case.script = script.clone();
    let committed = match set {
        Set::B => "game-whole".to_string(),
        _ => committed_script(Set::A, &s.level),
    };
    let path = sf_conform::cases_dir().join(format!("{committed}.toml"));
    case.lag_limit = Case::load(&path)?.lag_limit;
    Ok(case)
}

/// Whether `script` from `s` completes the level (§4.1). `seed` is the run's: set C's start is the
/// app's under power-on seed `seed seed`, as `sf-plan --app --seed` planned it.
pub fn verify(set: Set, s: &Start, script: &InputScript, seed: u32) -> Verdict {
    let run = std::panic::catch_unwind(|| match set {
        Set::C => verify_app(s.level, seed, script, AppCheck::RealApp),
        _ => verify_inner(set, s, script),
    });
    run.unwrap_or_else(|p| {
        let why = p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        Verdict::failed(format!("verify panicked: {why}"))
    })
}

fn verify_inner(set: Set, s: &Start, script: &InputScript) -> Verdict {
    let case = match case(set, s, script) {
        Ok(c) => c,
        Err(e) => return Verdict::failed(format!("case: {e}")),
    };
    let (route_frames, hearts_lost) = match replay(&case.sync, script, case.frames) {
        Ok(r) => r,
        Err(e) => return Verdict::failed(format!("replay: {e}")),
    };
    let mut v = Verdict {
        completed: false,
        route_frames,
        hearts_lost,
        reason: String::new(),
    };
    match sf_conform::run(&case) {
        Err(d) => {
            let first = d.to_string().lines().next().unwrap_or("").to_string();
            v.reason = format!("not faithful: {first}");
        }
        Ok(o) => {
            let (world, level) = numbers(&s.level);
            let exited = o.milestones.as_ref().is_some_and(|m| {
                m.iter().any(|(_, x)| *x == Milestone::LevelExit { world, level })
            });
            if exited {
                v.completed = true;
            } else {
                v.reason = "no exit".into();
            }
        }
    }
    v
}

/// The original's `world` and `level` numbers of a normal level.
fn numbers(l: &LevelId) -> (u16, u16) {
    let t = l.to_string();
    let b = t.as_bytes();
    ((b[1] - b'0') as u16, (b[3] - b'0') as u16)
}

/// The original replaying `script` from `sync` for `frames` frames: the frames to the end of the
/// one `level_state` first left −2 on (`None` if it never did), and the hearts lost before it.
pub fn replay(sync: &SyncPoint, script: &InputScript, frames: u32) -> Result<(Option<u32>, u32), String> {
    let mut a = sync::reach(sync, &InputScript::idle(), frames + 100)?.amiga;
    let word = |a: &sf_oracle::Amiga, at: u32| {
        let b = a.peek(at, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    let mut hearts = word(&a, HEARTS);
    let mut lost = 0;
    for f in 0..frames {
        sync::step_inserting(&mut a, script.input_at(f))?;
        let h = word(&a, HEARTS);
        if h < hearts {
            lost += (hearts - h) as u32;
        }
        hearts = h;
        let b = a.peek(LEVEL_STATE, 4);
        if f >= SETTLE && u32::from_be_bytes([b[0], b[1], b[2], b[3]]) != 0xffff_fffe {
            return Ok((Some(f + 1), lost));
        }
    }
    Ok((None, lost))
}

/// The committed route's exit frame (`ref_frames`), replayed from the same start.
pub fn ref_frames(set: Set, s: &Start) -> Option<u32> {
    let name = committed_script(set, &s.level);
    let script = InputScript::load(&sf_conform::case::script_path(&name)).ok()?;
    let end = script.lines.last().map_or(0, |l| l.0) + TAIL;
    std::panic::catch_unwind(|| replay(&sync_point(s), &script, end))
        .ok()?
        .ok()?
        .0
}

/// Set C's start (DESIGN §2.1 `--app`): the level's first frame as the app's `start-level <level>`
/// script line makes it, the Start Level dialog's defaults under power-on seed `seed seed` (sf-app
/// `menu.rs`). Built here and not taken from `sf-plan`, so the benchmark never links what it
/// measures.
pub fn app_start(level: LevelId, seed: u32) -> Game {
    let setup = LevelSetup {
        level,
        ..Default::default()
    };
    Game::start_level(sf_conform::display_data(), &setup, Rng::seeded(seed, seed))
}

/// A route played on the engine from [`app_start`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppReplay {
    /// Frames to the end of the one `level_state` first left −2 on; `None` if it never did.
    pub exit: Option<u32>,
    pub lives_lost: u32,
    pub hearts_lost: u32,
    /// The state hash after each step to the exit (`sf_core::feed::frame_hashes`' second), what
    /// the app's `--capture-hashes` writes for its frames 1 on.
    pub states: Vec<u64>,
}

/// `script` from `level`'s app start, `frames` frames at most, stepped as the app steps it.
pub fn engine_app(level: LevelId, seed: u32, script: &InputScript, frames: u32) -> AppReplay {
    let mut g = app_start(level, seed);
    let word = |g: &Game, at: u32| g.state().w(at);
    let (mut lives, mut hearts) = (word(&g, FROG + f::LIVES), word(&g, HEARTS));
    let mut r = AppReplay {
        exit: None,
        lives_lost: 0,
        hearts_lost: 0,
        states: Vec::new(),
    };
    for k in 0..frames {
        g.step(script.input_at(k));
        r.states.push(sf_core::feed::frame_hashes(&g, &g.render(g.view())).1);
        let (l, h) = (word(&g, FROG + f::LIVES), word(&g, HEARTS));
        r.lives_lost += lives.saturating_sub(l) as u32;
        r.hearts_lost += hearts.saturating_sub(h) as u32;
        (lives, hearts) = (l, h);
        if k >= SETTLE && g.state().l(LEVEL_STATE) != 0xffff_fffe {
            r.exit = Some(k + 1);
            break;
        }
    }
    r
}

/// The host keys that hold each joystick bit (sf-app `input.rs`'s defaults).
const KEYS: [(Joy, &str); 5] = [
    (Joy::UP, "ArrowUp"),
    (Joy::DOWN, "ArrowDown"),
    (Joy::LEFT, "ArrowLeft"),
    (Joy::RIGHT, "ArrowRight"),
    (Joy::FIRE, "Space"),
];

/// The app's script (sf-app `script.rs`) that plays `frames` frames of `script` as a person
/// would: power-on seed `seed seed`, Game ▸ Start Level… with the dialog's defaults at step 1, and
/// each joystick change a key going down or up. The route's frame `i` is the app's step `i + 1`
/// (a line at `N:` lands before step N), so the app's frame `i + 1` is the engine's after `i + 1`
/// steps (bug-recorder T11's `key_lines`).
pub fn app_script(level: LevelId, seed: u32, script: &InputScript, frames: u32) -> String {
    let mut s = format!("seed {seed} {seed}\n1: start-level {level}\n");
    let mut held = Joy::NONE;
    for i in 0..frames {
        let joy = script.input_at(i).joy;
        for (bit, key) in KEYS {
            let (was, now) = (held.0 & bit.0 != 0, joy.0 & bit.0 != 0);
            if was != now {
                s += &format!("{}: {} {key}\n", i + 1, if now { "down" } else { "up" });
            }
        }
        held = joy;
    }
    s
}

/// Whether the real app's frames 1..=`states.len()` have the engine's state hashes: `None`, or the
/// first frame they part at and why.
pub fn app_parts(hashes_txt: &str, states: &[u64]) -> Option<String> {
    let app = match sf_core::feed::parse_hashes(hashes_txt) {
        Ok(h) => h,
        Err(e) => return Some(format!("the app's hashes: {e}")),
    };
    for (k, want) in states.iter().enumerate() {
        match app.get(k + 1) {
            None => return Some(format!("the app stopped at frame {} of {}", app.len().saturating_sub(1), states.len())),
            Some((_, got)) if got != want => return Some(format!("the app parts from the engine at frame {}", k + 1)),
            _ => {}
        }
    }
    None
}

/// How far set C's check goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCheck {
    /// The engine replay alone (the test block's half; no window).
    Engine,
    /// The engine replay, then the real app (§4.1).
    RealApp,
}

/// Set C (§4.1): the engine from the app start to the exit with no life lost, then (with
/// `RealApp`) the real app played the route by its script layer, its state the engine's on every
/// frame to the exit.
pub fn verify_app(level: LevelId, seed: u32, script: &InputScript, check: AppCheck) -> Verdict {
    let frames = script.lines.last().map_or(0, |l| l.0) + TAIL;
    let r = engine_app(level, seed, script, frames);
    let mut v = Verdict {
        completed: false,
        route_frames: r.exit,
        hearts_lost: r.hearts_lost,
        reason: String::new(),
    };
    if r.exit.is_none() {
        v.reason = "no exit on the engine from the app start".into();
    } else if r.lives_lost > 0 {
        v.reason = format!("{} lives lost on the engine from the app start", r.lives_lost);
    } else if check == AppCheck::RealApp
        && let Err(why) = real_app(level, seed, script, &r.states)
    {
        v.reason = why;
    } else {
        v.completed = true;
    }
    v
}

/// The real app (`superfrog`, release) played `script` in a fresh home for as many frames as the
/// engine's `states`, its frame hashes compared with them: `Err` with why it parts.
fn real_app(level: LevelId, seed: u32, script: &InputScript, states: &[u64]) -> Result<(), String> {
    let frames = states.len() as u32;
    let bin = crate::adapters::build(&[("sf-app", "bin", "superfrog")])?;
    let home = scratch_home();
    let out = run_app(&bin[0], &home, &app_script(level, seed, script, frames), frames as u64 + 1);
    let parted = out.and_then(|hashes| app_parts(&hashes, states).map_or(Ok(()), Err));
    let _ = std::fs::remove_dir_all(&home);
    parted
}

/// A fresh home for one app run, under the temp folder.
fn scratch_home() -> PathBuf {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let home = std::env::temp_dir().join(format!("sf-bench-app-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    home
}

/// `superfrog` in `home` importing the owner's disks, then running `script` for `frames` frames
/// at 1× with each frame's hashes captured: `hashes.txt`. In its own process group, bounded by
/// its `--timeout` and the benchmark's guard (DESIGN §5.2).
fn run_app(bin: &Path, home: &Path, script: &str, frames: u64) -> Result<String, String> {
    let disks = sf_conform::disks::disk_dir()?;
    std::fs::create_dir_all(home).map_err(|e| format!("{}: {e}", home.display()))?;
    let (script_path, capture) = (home.join("route.script"), home.join("capture"));
    std::fs::write(&script_path, script).map_err(|e| format!("{}: {e}", script_path.display()))?;
    // The app runs at the PAL rate (50 frames a second), after an import of a few seconds.
    let timeout = 120 + frames / 25;
    let mut cmd = Command::new(bin);
    cmd.arg("--home")
        .arg(home)
        .arg("--cache-from-disks")
        .arg(&disks)
        .arg("--script")
        .arg(&script_path)
        .arg("--capture")
        .arg(&capture)
        .arg("--capture-hashes")
        .args(["--max-frames", &frames.to_string(), "--timeout", &timeout.to_string(), "--scale", "1"])
        .env_remove("SUPERFROG_HOME")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(home.join("app.stderr")).map_err(|e| e.to_string())?);
    let run = crate::child::run(cmd, Duration::from_secs(timeout + 30), None)?;
    let stderr = std::fs::read_to_string(home.join("app.stderr")).unwrap_or_default();
    if run.timed_out || !run.status.is_some_and(|s| s.success()) {
        let last = stderr.lines().last().unwrap_or("");
        return Err(format!("the app did not finish ({:?}): {last}", run.status));
    }
    std::fs::read_to_string(capture.join("hashes.txt")).map_err(|e| format!("the app's hashes: {e}"))
}
