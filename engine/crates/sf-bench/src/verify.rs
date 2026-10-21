//! What counts as a completed route (DESIGN §4.1, D9): a case built from the route passes
//! `sf_conform::run`, the faithful check the completion cases pass, and the original's milestones
//! include the level's exit. Its length and the hearts it lost are read off the original replaying
//! it.

use sf_conform::{Case, InputScript, Milestone, SyncPoint, sync};
use sf_core::LevelId;
use sf_core::layout::h0;

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

/// Whether `script` from `s` completes the level (§4.1).
pub fn verify(set: Set, s: &Start, script: &InputScript) -> Verdict {
    if set == Set::C {
        // Set C is checked on the engine from `start_level` and in the real app (§4.1), which
        // needs `Game::start_level` (D3): T12.
        return Verdict::failed("set C verification is not built yet (T12)");
    }
    let run = std::panic::catch_unwind(|| verify_inner(set, s, script));
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
