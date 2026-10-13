//! Case files: `engine/conform/cases/<name>.toml` (format in `engine/conform/README.md`).

use std::path::{Path, PathBuf};

use crate::script::InputScript;
use crate::sync::SyncPoint;

/// What a case compares each frame (DESIGN §2.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Compare {
    /// Every `LAYOUT` variable.
    pub vars: bool,
    /// The picture inside the display window, as 12-bit colour.
    pub picture: bool,
    /// The ordered Paula register writes.
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
                "sync", "script", "frames", "compare", "exclude", "vars", "resync",
            ]
            .contains(&k.as_str())
            {
                return Err(format!(
                    "unknown key `{k}` (sync, script, frames, compare, exclude, vars, resync)"
                ));
            }
        }
        let sync = match t.get("sync") {
            Some(toml::Value::String(s)) if s == "power-on" => SyncPoint::PowerOn,
            Some(toml::Value::String(s)) if s == "power-on-disk3" => SyncPoint::PowerOnDisk3,
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
            Some(toml::Value::Table(s)) if s.contains_key("level") => {
                if let Some(k) = s
                    .keys()
                    .find(|k| !["level", "clock", "lives"].contains(&k.as_str()))
                {
                    return Err(format!(
                        "sync: unknown key `{k}` beside `level` (level, clock, lives)"
                    ));
                }
                if s.contains_key("clock") && s.contains_key("lives") {
                    return Err("sync: `clock` or `lives`, not both".into());
                }
                let level = s["level"]
                    .as_str()
                    .ok_or("sync.level is a string like \"W1-1\"")?
                    .parse()?;
                match s.get("clock") {
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
            },
            _ => {
                return Err(
                    "sync is \"power-on\", \"power-on-disk3\", \"end-sequence\", { script = \"…\", frame = N }, { level = \"W1-1\" } or { slot = \"…\", start = \"…\" }"
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
                    "picture" => {
                        let r = e
                            .get("frames")
                            .and_then(|v| v.as_array())
                            .map(|a| a.iter().filter_map(|v| v.as_integer()).collect::<Vec<_>>())
                            .filter(|a| a.len() == 2 && 0 <= a[0] && a[0] <= a[1])
                            .ok_or("exclude: a picture's needs `frames = [from, to]`")?;
                        Excluded::Picture {
                            from: r[0] as u32,
                            to: r[1] as u32,
                        }
                    }
                    w => {
                        return Err(format!(
                            "exclude: `{w}` is not objects, enemies, panel, unreached or picture"
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
        Ok(Case {
            name: name.to_string(),
            sync,
            script,
            frames,
            compare,
            exclude,
            only_vars,
            resync,
        })
    }
}
