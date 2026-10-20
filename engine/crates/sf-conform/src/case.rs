//! Case files: `engine/conform/cases/<name>.toml` (format in `engine/conform/README.md`).

use std::path::{Path, PathBuf};

use crate::script::InputScript;
use crate::sync::SyncPoint;

/// What a case compares each frame (DESIGN §2.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Compare {
    /// Every `LAYOUT` variable: pass by pass and but the `presentation` ones (faithful), or
    /// every one each frame (`--strict`).
    pub vars: bool,
    /// The picture inside the display window, as 12-bit colour.
    pub picture: bool,
    /// The sound: as events (faithful), or the ordered Paula register writes (`--strict`).
    pub paula: bool,
    /// The picture the renderer draws from the original's memory transplanted afresh each frame
    /// (no engine stepping): what proves the renderer while the mechanics are not ported yet.
    pub render: bool,
}

impl Compare {
    pub const ALL: Compare = Compare {
        vars: true,
        picture: true,
        paula: true,
        render: false,
    };
    pub const VARS: Compare = Compare {
        vars: true,
        picture: false,
        paula: false,
        render: false,
    };
}

/// What a case leaves out because the engine does not model it yet (`[[exclude]]` in the case
/// file, each with the task that will model it): see [`crate::freeze`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Excluded {
    /// Every object of the original is frozen out from power-on and the engine runs with none
    /// (T14's frog cases, [`crate::freeze`]).
    Objects,
    /// The original's objects the engine has no kind for (enemies, each world's scenery) are
    /// frozen out from power-on ([`crate::freeze`]).
    Enemies,
    /// The status panel's bitmap and the picture's panel lines are not compared.
    Panel,
    /// Listed only: what the case's input never makes the original do (the spud thrown, the
    /// level finished…), so nothing needs freezing.
    Unreached,
    /// The picture is not compared on frames `from..=to` (`frames = [from, to]`): what the beam
    /// shows there depends on timing the engine does not model yet.
    Picture { from: u32, to: u32 },
    /// The channels' output (`sf-conform channels`) is known to differ on frames of play
    /// `from..=to` (`frames = [from, to]`): a run inside it is reported, not failed (T215).
    Channels { from: u32, to: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exclusion {
    pub what: Excluded,
    /// What it is, in the case file's words.
    pub note: String,
    /// The task that will model it.
    pub task: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub name: String,
    pub sync: SyncPoint,
    /// Frames count from the sync point.
    pub script: InputScript,
    pub frames: u32,
    pub compare: Compare,
    pub exclude: Vec<Exclusion>,
    /// When set, only these `LAYOUT` variables (by name) are compared: for a scene that models
    /// only some of what the original changes there (the title before T20, say).
    pub only_vars: Option<Vec<String>>,
    /// Whether the runner may re-sync the engine from the original's memory when a variable
    /// still differs on the frame after a lag (`resync = false` makes that a divergence).
    pub resync: bool,
    /// The case's own limit on frames in a row the original ends mid-pass, in place of
    /// [`crate::LAG_LIMIT`] (`lag-limit = { frames = N, note = "…" }`), with why it needs one.
    pub lag_limit: Option<LagLimit>,
    /// How the case is judged: faithful unless the run asks for `--strict` (not a case file's
    /// key: a case cannot choose its own bar).
    pub mode: crate::compare::Mode,
    /// Check 4, plays alone (DESIGN §2.1, T182): on for every case that steps the engine (owner,
    /// 2026-10-15), off for a `render`-only case; not a case file's key, as the bar is not a
    /// case's to choose. `run --alone` judges by it only.
    pub alone: Alone,
    /// A bug zip's case (bug-recorder T14): where the original is put back on the recording it
    /// left (`put-back = { steps = "…", at = [[F, S], …] }`, a `Setup` case's only).
    pub put_back: Option<PutBacks>,
}

/// Where a bug zip's case puts the original back on the owner's recording (bug-recorder T14):
/// under lag the original stops playing what the owner played, so at the pass end it is seen to
/// have left, the recording's engine there is written into its memory (`bug::Follow`) and the run
/// goes on from the owner's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutBacks {
    /// The recording's inputs, one per engine step from the setup's start (`scripts/<steps>.txt`).
    pub steps: InputScript,
    /// (the original's frames run, the recording's step): once the original has run that many
    /// frames, the recording's engine after that step is written into it. In order.
    pub at: Vec<(u32, u32)>,
    /// What a test writes into the recording's engine at its start (`bug::Pokes`); never in a
    /// case file.
    pub pokes: Vec<(u32, Vec<u8>)>,
}

/// Whether a run judges check 4, plays alone ([`crate::milestones`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Alone {
    /// A `render`-only case: nothing steps, nothing is judged alone.
    #[default]
    Off,
    /// After checks 1–3, which end the run at their first failure.
    With,
    /// Check 4 alone (`run --alone`): checks 1–3 still run, as the runner needs them to skip
    /// the original's loads, but none of their failures ends the run.
    Only,
}

impl Alone {
    /// What a case comparing `compare` gets by default.
    pub fn of(compare: Compare) -> Alone {
        if compare.vars || compare.picture || compare.paula {
            Alone::With
        } else {
            Alone::Off
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LagLimit {
    pub frames: u32,
    /// Why the case needs it, in the case file's words.
    pub note: String,
}

impl Case {
    pub fn excludes(&self, what: Excluded) -> bool {
        self.exclude.iter().any(|e| e.what == what)
    }

    /// Whether the picture of frame `frame` is left out ([`Excluded::Picture`]).
    pub fn picture_excluded(&self, frame: u32) -> bool {
        self.exclude.iter().any(|e| match e.what {
            Excluded::Picture { from, to } => (from..=to).contains(&frame),
            _ => false,
        })
    }

    /// The task owning a known stretch of the channels' output holding frames of play
    /// `first..=last` ([`Excluded::Channels`]), if one does.
    pub fn channels_known(&self, first: u32, last: u32) -> Option<&str> {
        self.exclude.iter().find_map(|e| match e.what {
            Excluded::Channels { from, to } if from <= first && last <= to => Some(e.task.as_str()),
            _ => None,
        })
    }

    /// What the case freezes out of the original.
    pub fn freeze(&self) -> crate::freeze::Freeze {
        use crate::freeze::Freeze;
        if self.excludes(Excluded::Objects) {
            Freeze::Objects
        } else if self.excludes(Excluded::Enemies) {
            Freeze::Enemies
        } else {
            Freeze::None
        }
    }
}

/// `engine/conform/scripts/`.
pub fn scripts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conform/scripts")
}

/// `engine/conform/cases/`.
pub fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conform/cases")
}

/// A script by name (`boot-to-title` is `engine/conform/scripts/boot-to-title.txt`), or by path
/// when the name holds a `/`.
pub fn script_path(name: &str) -> PathBuf {
    if name.contains('/') {
        PathBuf::from(name)
    } else {
        scripts_dir().join(format!("{name}.txt"))
    }
}

/// The keys only [`SyncPoint::Setup`]'s form has: any of them beside `level` makes `lives` the
/// Start Level dialog's 3/5/7, not `frog_lives` ([`SyncPoint::Lives`]).
const SETUP_KEYS: [&str; 6] = ["points", "energy", "difficulty", "spud", "wings", "seed"];

/// `sync = { level = "W1-1", lives = 3, points = 0, energy = 4, difficulty = "normal",
/// spud = false, wings = false, seed = "lo hi" }`: every key given, the values in the dialog's
/// ranges (`LevelSetup::validate`), the seed two hex longs as a recording's `seed` line has them.
fn setup_sync(s: &toml::Table) -> Result<SyncPoint, String> {
    use sf_core::recording::{Difficulty, LevelSetup, Lives};
    const KEYS: [&str; 8] = [
        "level", "lives", "points", "energy", "difficulty", "spud", "wings", "seed",
    ];
    if let Some(k) = s.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!(
            "sync: unknown key `{k}` in a level setup ({})",
            KEYS.join(", ")
        ));
    }
    if let Some(k) = KEYS.iter().find(|k| !s.contains_key(**k)) {
        return Err(format!(
            "sync: a level setup needs `{k}` ({})",
            KEYS.join(", ")
        ));
    }
    let int = |k: &str| s[k].as_integer();
    let flag = |k: &str| s[k].as_bool().ok_or(format!("sync.{k} is true or false"));
    let hex = |h: &str| {
        ((1..=8).contains(&h.len()) && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| u32::from_str_radix(h, 16).ok())
            .flatten()
    };
    let setup = LevelSetup {
        level: s["level"]
            .as_str()
            .ok_or("sync.level is a string like \"W1-1\"")?
            .parse()?,
        lives: match int("lives") {
            Some(3) => Lives::Three,
            Some(5) => Lives::Five,
            Some(7) => Lives::Seven,
            _ => return Err("sync.lives in a level setup is 3, 5 or 7".into()),
        },
        points: int("points")
            .and_then(|v| u32::try_from(v).ok())
            .ok_or("sync.points is a score, 0 or more")?,
        energy: int("energy")
            .and_then(|v| u8::try_from(v).ok())
            .ok_or("sync.energy is 1 to 4 hearts")?,
        difficulty: match s["difficulty"].as_str() {
            Some("easy") => Difficulty::Easy,
            Some("normal") => Difficulty::Normal,
            _ => return Err("sync.difficulty is \"easy\" or \"normal\"".into()),
        },
        spud: flag("spud")?,
        wings: flag("wings")?,
    };
    setup.validate().map_err(|e| format!("sync: {e}"))?;
    let seed = s["seed"]
        .as_str()
        .and_then(|v| v.split_once(' '))
        .and_then(|(lo, hi)| {
            Some(sf_core::Rng {
                lo: hex(lo)?,
                hi: hex(hi)?,
            })
        })
        .ok_or("sync.seed is two hex longs, \"lo hi\"")?;
    Ok(SyncPoint::Setup { setup, seed })
}

impl Case {
    pub fn load(path: &Path) -> Result<Case, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Case::parse(&name, &text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// A case from its TOML text.
    pub fn parse(name: &str, text: &str) -> Result<Case, String> {
        let t: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
        for k in t.keys() {
            if ![
                "sync",
                "script",
                "script-from",
                "frames",
                "compare",
                "exclude",
                "vars",
                "resync",
                "lag-limit",
                "put-back",
            ]
            .contains(&k.as_str())
            {
                return Err(format!(
                    "unknown key `{k}` (sync, script, script-from, frames, compare, exclude, vars, resync, lag-limit, put-back)"
                ));
            }
        }
        let sync = match t.get("sync") {
            Some(toml::Value::String(s)) if s == "power-on" => SyncPoint::PowerOn,
            Some(toml::Value::String(s)) if s == "power-on-disk3" => SyncPoint::PowerOnDisk3,
            Some(toml::Value::String(s)) if s == "power-on-df1" => SyncPoint::PowerOnDf1,
            Some(toml::Value::String(s)) if s == "end-sequence" => SyncPoint::EndSequence,
            Some(toml::Value::Table(s)) if s.contains_key("slot") => {
                let name = |k: &str| match s.get(k) {
                    None => Ok(None),
                    Some(v) => v
                        .as_str()
                        .map(|v| Some(v.to_string()))
                        .ok_or(format!("sync.{k} is a script name")),
                };
                if let Some(k) = s.keys().find(|k| !["slot", "start"].contains(&k.as_str())) {
                    return Err(format!(
                        "sync: unknown key `{k}` beside `slot` (slot, start)"
                    ));
                }
                SyncPoint::Slot {
                    script: name("slot")?.unwrap_or_default(),
                    start: name("start")?,
                }
            }
            Some(toml::Value::Table(s)) if s.contains_key("end-sequence") => {
                if let Some(k) = s
                    .keys()
                    .find(|k| !["end-sequence", "score"].contains(&k.as_str()))
                {
                    return Err(format!("sync: unknown key `{k}` beside `end-sequence` (score)"));
                }
                if s["end-sequence"].as_bool() != Some(true) {
                    return Err("sync.end-sequence is true".into());
                }
                SyncPoint::EndSequenceScore {
                    score: s
                        .get("score")
                        .and_then(|v| v.as_integer())
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or("sync.score beside `end-sequence` is a score, 0 or more")?,
                }
            }
            Some(toml::Value::Table(s)) if s.contains_key("game-over") => {
                if let Some(k) = s.keys().find(|k| !["game-over", "score"].contains(&k.as_str())) {
                    return Err(format!("sync: unknown key `{k}` beside `game-over` (score)"));
                }
                SyncPoint::GameOver {
                    script: s["game-over"]
                        .as_str()
                        .ok_or("sync.game-over is a script name")?
                        .to_string(),
                    score: match s.get("score") {
                        None => None,
                        Some(v) => Some(
                            v.as_integer()
                                .and_then(|v| u32::try_from(v).ok())
                                .ok_or("sync.score is a score, 0 or more")?,
                        ),
                    },
                }
            }
            Some(toml::Value::Table(s))
                if s.contains_key("level") && SETUP_KEYS.iter().any(|k| s.contains_key(*k)) =>
            {
                setup_sync(s)?
            }
            Some(toml::Value::Table(s)) if s.contains_key("level") => {
                if let Some(k) = s
                    .keys()
                    .find(|k| !["level", "clock", "exit", "lives"].contains(&k.as_str()))
                {
                    return Err(format!(
                        "sync: unknown key `{k}` beside `level` (level, clock, exit, lives)"
                    ));
                }
                if [s.contains_key("clock"), s.contains_key("exit"), s.contains_key("lives")]
                    .iter()
                    .filter(|&&b| b)
                    .count()
                    > 1
                {
                    return Err("sync: one of `clock`, `exit` and `lives`".into());
                }
                let level = s["level"]
                    .as_str()
                    .ok_or("sync.level is a string like \"W1-1\"")?
                    .parse()?;
                let exit = match s.get("exit") {
                    None => false,
                    Some(v) => v.as_bool().ok_or("sync.exit is true or false")?,
                };
                match s.get("clock") {
                    None if exit => SyncPoint::Exit(level),
                    Some(_) if exit => return Err("sync: `clock` and `exit` together".into()),
                    None if s.contains_key("lives") => SyncPoint::Lives {
                        level,
                        lives: s["lives"]
                            .as_integer()
                            .and_then(|v| u16::try_from(v).ok())
                            .filter(|&v| v <= 99)
                            .ok_or("sync.lives is a number of lives, 0 to 99")?,
                    },
                    None => SyncPoint::AtLevel(level),
                    Some(v) => {
                        let bad = || "sync.clock is \"M:SS\", under 10 minutes".to_string();
                        let (m, sec) = v.as_str().and_then(|c| c.split_once(':')).ok_or_else(bad)?;
                        if m.len() != 1 || sec.len() != 2 {
                            return Err(bad());
                        }
                        let (m, sec): (u16, u16) = (
                            m.parse().map_err(|_| bad())?,
                            sec.parse().map_err(|_| bad())?,
                        );
                        if sec > 59 {
                            return Err(bad());
                        }
                        SyncPoint::Clock {
                            level,
                            seconds: m * 60 + sec,
                        }
                    }
                }
            }
            Some(toml::Value::Table(s)) => SyncPoint::AtFrame {
                script: s
                    .get("script")
                    .and_then(|v| v.as_str())
                    .ok_or("sync needs `script` and `frame`")?
                    .to_string(),
                frame: s
                    .get("frame")
                    .and_then(|v| v.as_integer())
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or("sync needs `script` and `frame` (a frame number)")?,
                df1: match s.get("df1") {
                    None => false,
                    Some(v) => v.as_bool().ok_or("sync.df1 is true or false")?,
                },
            },
            _ => {
                return Err(
                    "sync is \"power-on\", \"power-on-disk3\", \"power-on-df1\", \"end-sequence\", { script = \"…\", frame = N }, { level = \"W1-1\" }, { level = \"W1-1\", lives = 3, points = 0, energy = 4, difficulty = \"normal\", spud = false, wings = false, seed = \"lo hi\" }, { slot = \"…\", start = \"…\" }, { game-over = \"…\" } or { end-sequence = true, score = N }"
                        .into(),
                );
            }
        };
        let script = match t.get("script") {
            None => InputScript::idle(),
            Some(v) => {
                InputScript::load(&script_path(v.as_str().ok_or("script is a script name")?))?
            }
        };
        // `script-from`: the case's first frame plays the script's frame N (T208: a stretch of a
        // long script, its sync point that script's frame N).
        let script = match t.get("script-from") {
            None => script,
            Some(v) => script.from_frame(
                v.as_integer()
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or("script-from is a frame number")?,
            ),
        };
        let frames = t
            .get("frames")
            .and_then(|v| v.as_integer())
            .and_then(|v| u32::try_from(v).ok())
            .filter(|&v| v > 0)
            .ok_or("frames is a positive number")?;
        let compare = match t.get("compare") {
            None => Compare::ALL,
            Some(toml::Value::Array(a)) => {
                let mut c = Compare::default();
                for v in a {
                    match v.as_str() {
                        Some("vars") => c.vars = true,
                        Some("picture") => c.picture = true,
                        Some("paula") => c.paula = true,
                        Some("render") => c.render = true,
                        _ => {
                            return Err(format!(
                                "compare: {v} is not vars, picture, paula or render"
                            ));
                        }
                    }
                }
                c
            }
            Some(_) => {
                return Err(
                    "compare is a list of \"vars\", \"picture\", \"paula\", \"render\"".into(),
                );
            }
        };
        let mut exclude = Vec::new();
        if let Some(v) = t.get("exclude") {
            let items = v
                .as_array()
                .ok_or("exclude is a list of [[exclude]] tables")?;
            for e in items {
                let e = e.as_table().ok_or("each exclude is a table")?;
                let s = |k: &str| {
                    e.get(k)
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .ok_or(format!("exclude needs `{k}`"))
                };
                let what = match s("what")?.as_str() {
                    "objects" => Excluded::Objects,
                    "enemies" => Excluded::Enemies,
                    "panel" => Excluded::Panel,
                    "unreached" => Excluded::Unreached,
                    w @ ("picture" | "channels") => {
                        let r = e
                            .get("frames")
                            .and_then(|v| v.as_array())
                            .map(|a| a.iter().filter_map(|v| v.as_integer()).collect::<Vec<_>>())
                            .filter(|a| a.len() == 2 && 0 <= a[0] && a[0] <= a[1])
                            .ok_or(format!("exclude: a {w} one needs `frames = [from, to]`"))?;
                        let (from, to) = (r[0] as u32, r[1] as u32);
                        if w == "picture" {
                            Excluded::Picture { from, to }
                        } else {
                            Excluded::Channels { from, to }
                        }
                    }
                    w => {
                        return Err(format!(
                            "exclude: `{w}` is not objects, enemies, panel, unreached, picture or channels"
                        ));
                    }
                };
                exclude.push(Exclusion {
                    what,
                    note: s("note")?,
                    task: s("task")?,
                });
            }
        }
        let only_vars = match t.get("vars") {
            None => None,
            Some(v) => {
                let names: Vec<String> = v
                    .as_array()
                    .ok_or("vars is a list of LAYOUT names")?
                    .iter()
                    .map(|n| n.as_str().map(str::to_string).ok_or("vars holds names"))
                    .collect::<Result<_, _>>()?;
                for n in &names {
                    if !sf_core::LAYOUT.iter().any(|d| d.name == n) {
                        return Err(format!("vars: `{n}` is not a LAYOUT variable"));
                    }
                }
                Some(names)
            }
        };
        let resync = match t.get("resync") {
            None => true,
            Some(v) => v.as_bool().ok_or("resync is true or false")?,
        };
        let lag_limit = match t.get("lag-limit") {
            None => None,
            Some(v) => {
                let bad = "lag-limit is { frames = N, note = \"why\" }";
                let l = v.as_table().ok_or(bad)?;
                if let Some(k) = l.keys().find(|k| !["frames", "note"].contains(&k.as_str())) {
                    return Err(format!("lag-limit: unknown key `{k}` (frames, note)"));
                }
                Some(LagLimit {
                    frames: l
                        .get("frames")
                        .and_then(|v| v.as_integer())
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or(bad)?,
                    note: l
                        .get("note")
                        .and_then(|v| v.as_str())
                        .filter(|n| !n.trim().is_empty())
                        .ok_or(bad)?
                        .to_string(),
                })
            }
        };
        let put_back = match t.get("put-back") {
            None => None,
            Some(v) => {
                let bad = "put-back is { steps = \"script\", at = [[frame, step], …] }";
                if !matches!(sync, SyncPoint::Setup { .. }) {
                    return Err("put-back needs a level-setup sync (a bug zip's case)".into());
                }
                let p = v.as_table().ok_or(bad)?;
                if let Some(k) = p.keys().find(|k| !["steps", "at"].contains(&k.as_str())) {
                    return Err(format!("put-back: unknown key `{k}` (steps, at)"));
                }
                let steps = p.get("steps").and_then(|v| v.as_str()).ok_or(bad)?;
                let mut at = Vec::new();
                for pair in p.get("at").and_then(|v| v.as_array()).ok_or(bad)? {
                    let n: Vec<u32> = pair
                        .as_array()
                        .ok_or(bad)?
                        .iter()
                        .map(|v| v.as_integer().and_then(|v| u32::try_from(v).ok()))
                        .collect::<Option<_>>()
                        .ok_or(bad)?;
                    let [frame, step] = n[..] else { return Err(bad.into()) };
                    if at.last().is_some_and(|&(f, _)| f >= frame) {
                        return Err("put-back: the frames in `at` are in increasing order".into());
                    }
                    at.push((frame, step));
                }
                Some(PutBacks {
                    steps: InputScript::load(&script_path(steps))?,
                    at,
                    pokes: Vec::new(),
                })
            }
        };
        Ok(Case {
            name: name.to_string(),
            sync,
            script,
            frames,
            compare,
            exclude,
            only_vars,
            resync,
            lag_limit,
            mode: crate::compare::Mode::Faithful,
            alone: Alone::of(compare),
            put_back,
        })
    }
}
