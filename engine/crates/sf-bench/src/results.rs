//! `results.jsonl` (DESIGN §3.4): one flat JSON object per level run, appended and flushed as
//! each level ends, so a killed run loses at most the level in flight and a rerun skips what is
//! recorded for the same (planner, guided, set, level, commit) (§2.9). `serde_json` is not in the
//! workspace (§5), so lines are written and read here: numbers, booleans, `null` and short
//! strings only.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct LevelResult {
    pub planner: String,
    pub set: String,
    pub level: String,
    pub commit: String,
    pub completed: bool,
    pub wall_s: f64,
    pub engine_frames: u64,
    pub oracle_frames: u64,
    pub confirm_share: f64,
    pub route_frames: Option<u32>,
    pub ref_frames: Option<u32>,
    pub hearts_lost: u32,
    /// Why it did not complete (`timeout`, `hinted`, `over-budget`, `no exit`, `not faithful: …`);
    /// empty when it did.
    pub reason: String,
    /// For a `best-old` line, the old tool whose result it is.
    pub tool: Option<String>,
    /// The planner was given the level's committed route as its guide (DESIGN §4.2, D21). Lines
    /// written before the field existed read as `false`: the old tools' and every early run's.
    pub guided: bool,
}

impl LevelResult {
    /// A guided and an unguided run of one planner on one level are two results, both kept.
    pub fn key(&self) -> (String, bool, String, String, String) {
        (self.planner.clone(), self.guided, self.set.clone(), self.level.clone(), self.commit.clone())
    }

    pub fn to_json(&self) -> String {
        let mut s = String::from("{");
        let str_field = |s: &mut String, k: &str, v: &str| {
            let _ = write!(s, "\"{k}\":\"{}\",", escape(v));
        };
        let opt = |v: Option<u32>| v.map_or("null".to_string(), |v| v.to_string());
        str_field(&mut s, "planner", &self.planner);
        str_field(&mut s, "set", &self.set);
        str_field(&mut s, "level", &self.level);
        str_field(&mut s, "commit", &self.commit);
        let _ = write!(
            s,
            "\"completed\":{},\"wall_s\":{:.1},\"engine_frames\":{},\"oracle_frames\":{},\
             \"confirm_share\":{:.4},\"route_frames\":{},\"ref_frames\":{},\"hearts_lost\":{},",
            self.completed,
            self.wall_s,
            self.engine_frames,
            self.oracle_frames,
            self.confirm_share,
            opt(self.route_frames),
            opt(self.ref_frames),
            self.hearts_lost
        );
        if let Some(t) = &self.tool {
            str_field(&mut s, "tool", t);
        }
        let _ = write!(s, "\"guided\":{},", self.guided);
        let _ = write!(s, "\"reason\":\"{}\"}}", escape(&self.reason));
        s
    }

    /// A line back; `None` for anything not a whole object of the expected fields (a line cut
    /// off by a killed run).
    pub fn from_json(line: &str) -> Option<LevelResult> {
        let f = parse_object(line.trim())?;
        let s = |k: &str| match f.get(k)? {
            Value::Str(s) => Some(s.clone()),
            _ => None,
        };
        let n = |k: &str| match f.get(k)? {
            Value::Num(n) => Some(*n),
            _ => None,
        };
        let opt = |k: &str| match f.get(k)? {
            Value::Num(n) => Some(Some(*n as u32)),
            Value::Null => Some(None),
            _ => None,
        };
        Some(LevelResult {
            planner: s("planner")?,
            set: s("set")?,
            level: s("level")?,
            commit: s("commit")?,
            completed: match f.get("completed")? {
                Value::Bool(b) => *b,
                _ => return None,
            },
            wall_s: n("wall_s")?,
            engine_frames: n("engine_frames")? as u64,
            oracle_frames: n("oracle_frames")? as u64,
            confirm_share: n("confirm_share")?,
            route_frames: opt("route_frames")?,
            ref_frames: opt("ref_frames")?,
            hearts_lost: n("hearts_lost")? as u32,
            reason: s("reason")?,
            tool: s("tool"),
            guided: match f.get("guided") {
                None => false,
                Some(Value::Bool(b)) => *b,
                Some(_) => return None,
            },
        })
    }
}

/// Every whole line of the file, in order; a missing file is empty.
pub fn load(path: &Path) -> Vec<LevelResult> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(LevelResult::from_json)
        .collect()
}

/// Appends `r` as a line and flushes it. A file whose last line was cut off (no newline) gets one
/// first, so the cut line stays alone and is skipped on reading.
pub fn append(path: &Path, r: &LevelResult) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let cut = std::fs::read(path).is_ok_and(|b| b.last().is_some_and(|&c| c != b'\n'));
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    let mut line = String::new();
    if cut {
        line.push('\n');
    }
    line += &r.to_json();
    line.push('\n');
    f.write_all(line.as_bytes())?;
    f.sync_data()
}

/// Whether `results` already holds a line for this (planner, guided, set, level, commit).
pub fn recorded(
    results: &[LevelResult],
    planner: &str,
    guided: bool,
    set: &str,
    level: &str,
    commit: &str,
) -> Option<LevelResult> {
    results
        .iter()
        .rev()
        .find(|r| r.planner == planner && r.guided == guided && r.set == set && r.level == level && r.commit == commit)
        .cloned()
}

/// A failed run's reason with the planner's own (DESIGN §2.9: `sf-plan: no route: …`, its last
/// such stderr line) after it, so a run cut off at its budget still says how far it got (T14:
/// the tuning table read only `over-budget`).
pub fn with_planner_reason(reason: &str, stderr: &str) -> String {
    match stderr
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("sf-plan: no route: "))
    {
        Some(why) if !reason.contains(why) => format!("{reason}; {why}"),
        _ => reason.to_string(),
    }
}

/// The `PLAN_STATS` line a planner prints to stderr at its exit: `key=value` words after the tag.
/// The last such line counts.
pub fn plan_stats(stderr: &str) -> Option<BTreeMap<String, String>> {
    let line = stderr.lines().rev().find(|l| l.starts_with("PLAN_STATS"))?;
    Some(
        line.split_whitespace()
            .skip(1)
            .filter_map(|w| w.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

fn escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '\n' => o += "\\n",
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
}

/// A flat object: `{"k":v,...}` with string, number, boolean or null values.
fn parse_object(s: &str) -> Option<BTreeMap<String, Value>> {
    let mut c = s.strip_prefix('{')?.strip_suffix('}')?.chars().peekable();
    let mut out = BTreeMap::new();
    let skip_ws = |c: &mut std::iter::Peekable<std::str::Chars>| {
        while c.peek().is_some_and(|x| x.is_whitespace()) {
            c.next();
        }
    };
    let string = |c: &mut std::iter::Peekable<std::str::Chars>| -> Option<String> {
        if c.next()? != '"' {
            return None;
        }
        let mut o = String::new();
        loop {
            match c.next()? {
                '"' => return Some(o),
                '\\' => match c.next()? {
                    'n' => o.push('\n'),
                    'u' => {
                        let h: String = (0..4).filter_map(|_| c.next()).collect();
                        o.push(char::from_u32(u32::from_str_radix(&h, 16).ok()?)?);
                    }
                    x => o.push(x),
                },
                x => o.push(x),
            }
        }
    };
    loop {
        skip_ws(&mut c);
        if c.peek().is_none() {
            return Some(out);
        }
        let k = string(&mut c)?;
        skip_ws(&mut c);
        if c.next()? != ':' {
            return None;
        }
        skip_ws(&mut c);
        let v = if c.peek() == Some(&'"') {
            Value::Str(string(&mut c)?)
        } else {
            let mut w = String::new();
            while c.peek().is_some_and(|&x| x != ',') {
                w.push(c.next()?);
            }
            match w.trim() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                "null" => Value::Null,
                n => Value::Num(n.parse().ok()?),
            }
        };
        out.insert(k, v);
        skip_ws(&mut c);
        match c.next() {
            None => return Some(out),
            Some(',') => {}
            Some(_) => return None,
        }
    }
}
