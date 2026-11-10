//! Case files: `engine/conform/cases/<name>.toml`, format in `engine/conform/README.md`. Read by a
//! small parser here, since the plan admits no `toml` dependency (DESIGN §5). A key or value the
//! format does not know is an error: a comparison quietly narrower than its file says would pass for
//! the wrong reason.

use std::ops::Range;
use std::path::Path;

use fd_core::layout_var;

use crate::conform_dir;
use crate::sync::{LEVEL_1_SCRIPT, LEVEL_1_SYNC};

/// A disk change on the way to the sync point: `disk` (1 or 2) into df0, before `frame` runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swap {
    pub frame: u32,
    pub disk: u32,
}

/// What a case compares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Compare {
    /// The case's `vars`, at the end of each pass.
    pub vars: bool,
    /// The play area and the status bar, each frame a pass ended in, within the owner's tolerance.
    pub picture: bool,
}

/// What a level case must reach, besides both sides agreeing: two sides failing alike agree
/// (`toolkit/docs/porting-routines.md` §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Goal {
    /// Every `(var, value)` holds together at the end of one compared pass ending at or before
    /// oracle frame `by_frame`.
    All {
        all: Vec<(String, u32)>,
        by_frame: u32,
    },
    /// A stretch where only state is expected to stay in step: the run reaches its last frame with
    /// no engine event, and every listed variable is compared on every pass.
    Idle,
    /// Both sides go to the game over screen together and leave it in the same step, where the
    /// original enters its high scores and the engine restarts the level (T23, DESIGN §2.4), at or
    /// before oracle frame `by_frame`. The comparison ends there.
    GameOver { by_frame: u32 },
    /// Both sides find the level's end together: the original runs `next_world` in the step the
    /// engine enters [`fd_core::Scene::LevelEnd`], `world_index` 1 on both (T26), at or before
    /// oracle frame `by_frame`. The comparison ends there (`crate::AFTER_NEXT_WORLD`).
    LevelEnd { by_frame: u32 },
}

/// Picture rows, a box of them, or a variable, whose differences a case does not fail on, and the
/// task that will make them compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exclusion {
    /// Rows of the compared picture (row 0 is the play area's first line); empty for a variable.
    pub rows: Range<u16>,
    /// Columns of those rows (`"rows A..B cols C..D"`), so the rest of the rows is still compared;
    /// `None` for whole rows (T44: the countdown's digits share rows with the items').
    pub cols: Option<Range<u16>>,
    /// A `LAYOUT` variable read each pass and counted where it differs (`what = "var NAME"`).
    pub var: Option<String>,
    /// As the case file wrote it.
    pub what: String,
    pub task: String,
}

/// One case file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub name: String,
    pub script: String,
    /// How many oracle frames run before the first comparison, and where the engine starts.
    pub sync_frame: u32,
    pub swap: Option<Swap>,
    /// Frames run and compared after the sync point.
    pub frames: u32,
    /// The input after the sync point, `scripts/<input>.txt` with frames counted from it; `None`
    /// carries on with the sync script.
    pub input: Option<String>,
    pub compare: Compare,
    pub vars: Vec<String>,
    pub goal: Option<Goal>,
    pub exclude: Vec<Exclusion>,
    /// `LAYOUT` variables written into the original at the sync point, before the engine takes its
    /// memory: what no input can reach in play, as `cheat_flag`, which only the title's code sets
    /// (T26). `set = [{ var = "cheat_flag", value = 1 }]`.
    pub set: Vec<(String, u32)>,
}

impl Case {
    /// Read a case file; a relative path that is not there is tried under [`conform_dir`], so the
    /// runner takes `cases/x.toml` from any folder.
    pub fn load(path: &Path) -> Result<Case, String> {
        let found = if path.is_relative() && !path.exists() {
            conform_dir().join(path)
        } else {
            path.to_path_buf()
        };
        let text = std::fs::read_to_string(&found)
            .map_err(|e| format!("case {}: {e}", found.display()))?;
        let name = found.file_stem().unwrap_or_default().to_string_lossy();
        Case::parse(&name, &text).map_err(|e| format!("case {}: {e}", found.display()))
    }

    /// Whether this case starts in play (the level sync point) rather than on the title.
    pub fn is_level(&self) -> bool {
        self.script == LEVEL_1_SCRIPT
    }

    /// The whole format and nothing else.
    pub fn parse(name: &str, text: &str) -> Result<Case, String> {
        let mut seen: Vec<(String, Value)> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let at = |e: String| format!("line {}: {e}", i + 1);
            let body = strip_comment(raw).trim();
            if body.is_empty() {
                continue;
            }
            let (key, value) = body
                .split_once('=')
                .ok_or_else(|| at(format!("`{body}`: expected `key = value`")))?;
            let key = key.trim();
            if !KEYS.contains(&key) {
                return Err(at(format!("`{key}` is not a case key")));
            }
            // A second `frames` would otherwise win silently over the one a reader saw first.
            if seen.iter().any(|(k, _)| k == key) {
                return Err(at(format!("`{key}` is given twice")));
            }
            seen.push((key.to_string(), parse_value(value.trim()).map_err(at)?));
        }
        let get = |k: &str| seen.iter().find(|(key, _)| key == k).map(|(_, v)| v);
        let missing = |k: &str| format!("no `{k}`");

        let (script, sync_frame) = sync_table(get("sync").ok_or_else(|| missing("sync"))?)?;
        let level = script == LEVEL_1_SCRIPT;
        let swap = get("swap").map(swap_table).transpose()?;
        let frames = get("frames")
            .ok_or_else(|| missing("frames"))?
            .int("frames")?;
        if sync_frame.checked_add(frames).is_none() {
            return Err(format!(
                "sync frame {sync_frame} plus frames {frames} is past any frame cap"
            ));
        }
        let input = get("input").map(|v| v.string("input")).transpose()?;

        let compare = compare_list(get("compare").ok_or_else(|| missing("compare"))?)?;
        let vars = match get("vars") {
            Some(v) => v.strings("vars")?,
            None => Vec::new(),
        };
        if compare.vars && vars.is_empty() {
            return Err(
                "`compare` has \"vars\" but `vars` is missing or empty: the case would \
                        compare nothing"
                    .to_string(),
            );
        }
        if !compare.vars && !vars.is_empty() {
            return Err("`vars` is given but `compare` has no \"vars\"".to_string());
        }
        for v in &vars {
            known(v)?;
        }
        let goal = get("goal")
            .map(|g| goal(g, sync_frame, frames))
            .transpose()?;
        let exclude = match get("exclude") {
            Some(v) => v
                .list("exclude")?
                .iter()
                .map(exclusion)
                .collect::<Result<_, _>>()?,
            None => Vec::new(),
        };
        let set = match get("set") {
            Some(v) => var_values(v, "set")?,
            None => Vec::new(),
        };
        if goal == Some(Goal::Idle) && !compare.vars {
            return Err(
                "goal `idle = true` is judged on the case's `vars`, but `compare` has \
                        no \"vars\""
                    .to_string(),
            );
        }
        for e in &exclude {
            match &e.var {
                None if !compare.picture => {
                    return Err(
                        "`exclude` leaves picture rows out, but `compare` has no \"picture\""
                            .to_string(),
                    );
                }
                Some(_) if !compare.vars => {
                    return Err(format!(
                        "`exclude` {}: a variable is excluded from `compare = [\"vars\"]`, \
                         which the case does not have",
                        e.what
                    ));
                }
                Some(v) if vars.contains(v) => {
                    return Err(format!(
                        "`exclude` {}: `{v}` is also in `vars`; a variable is compared or \
                         excluded, not both",
                        e.what
                    ));
                }
                _ => {}
            }
        }
        if level {
            // The level sync point is the cached snapshot, which made its own disk change.
            if swap.is_some() {
                return Err(format!(
                    "`swap`: a {LEVEL_1_SCRIPT} case starts from the level sync point, which \
                     already has disk 2 in df0"
                ));
            }
            if sync_frame < LEVEL_1_SYNC {
                return Err(format!(
                    "sync frame {sync_frame}: play starts at frame {LEVEL_1_SYNC}"
                ));
            }
            if goal.is_none() {
                return Err("a level case states its `goal` (porting-routines §4): \
                            `{ all = [...], by_frame = N }`, `{ idle = true }`, \
                            `{ game_over = true, by_frame = N }` or \
                            `{ level_end = true, by_frame = N }`"
                    .to_string());
            }
        } else {
            // The title runner is forest-dumb-start's: one variable, state only.
            for (taken, key) in [
                (goal.is_some(), "goal"),
                (compare.picture, "compare = [\"picture\"]"),
                (!set.is_empty(), "set"),
            ] {
                if taken {
                    return Err(format!(
                        "`{key}` is for level cases (sync script {LEVEL_1_SCRIPT})"
                    ));
                }
            }
        }
        // `run` swaps only on the way to the sync point, so a later swap would never happen.
        if let Some(Swap { frame, .. }) = swap.filter(|s| s.frame >= sync_frame) {
            return Err(format!(
                "swap at frame {frame}: a swap must come before the sync frame {sync_frame}"
            ));
        }
        Ok(Case {
            name: name.to_string(),
            script,
            sync_frame,
            swap,
            frames,
            input,
            compare,
            vars,
            goal,
            exclude,
            set,
        })
    }
}

// No `freeze` or `thaw`: T79 removed the last frozen routine (docs/10, "Freezing objects"), so a
// case naming either is refused as any unknown key is.
const KEYS: [&str; 9] = [
    "sync", "swap", "frames", "input", "compare", "vars", "goal", "exclude", "set",
];

fn known(var: &str) -> Result<(), String> {
    layout_var(var)
        .map(|_| ())
        .ok_or_else(|| format!("`{var}` is not in fd-core's LAYOUT"))
}

/// The line up to a `#` that is not inside a string.
fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}

/// A value of the format: what one line's right-hand side holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Int(u32),
    Str(String),
    Bool(bool),
    List(Vec<Value>),
    Table(Vec<(String, Value)>),
}

impl Value {
    fn int(&self, what: &str) -> Result<u32, String> {
        match self {
            Value::Int(n) => Ok(*n),
            other => Err(format!("{what} = {}: not a whole number", other.show())),
        }
    }

    fn string(&self, what: &str) -> Result<String, String> {
        match self {
            Value::Str(s) => Ok(s.clone()),
            other => Err(format!("{what} = {}: not a quoted string", other.show())),
        }
    }

    fn list(&self, what: &str) -> Result<&[Value], String> {
        match self {
            Value::List(l) => Ok(l),
            other => Err(format!("{what} = {}: not a list", other.show())),
        }
    }

    fn strings(&self, what: &str) -> Result<Vec<String>, String> {
        self.list(what)?.iter().map(|v| v.string(what)).collect()
    }

    /// The table's pairs, every key among `keys`.
    fn table(&self, what: &str, keys: &[&str]) -> Result<&[(String, Value)], String> {
        let Value::Table(pairs) = self else {
            return Err(format!(
                "{what} = {}: not a `{{ key = value }}` table",
                self.show()
            ));
        };
        for (i, (k, _)) in pairs.iter().enumerate() {
            if !keys.contains(&k.as_str()) {
                return Err(format!("`{k}` is not a {what} key"));
            }
            if pairs[..i].iter().any(|(j, _)| j == k) {
                return Err(format!("{what}: `{k}` is given twice"));
            }
        }
        Ok(pairs)
    }

    fn show(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Str(s) => format!("\"{s}\""),
            Value::Bool(b) => b.to_string(),
            Value::List(_) => "[...]".to_string(),
            Value::Table(_) => "{...}".to_string(),
        }
    }
}

fn field<'a>(pairs: &'a [(String, Value)], key: &str, what: &str) -> Result<&'a Value, String> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
        .ok_or_else(|| format!("{what} has no `{key}`"))
}

/// One value, the whole of `text`.
pub(crate) fn parse_value(text: &str) -> Result<Value, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    let v = value(&chars, &mut at)?;
    skip_space(&chars, &mut at);
    if at != chars.len() {
        return Err(format!(
            "`{text}`: unexpected `{}`",
            &text[byte(text, at)..]
        ));
    }
    Ok(v)
}

fn byte(text: &str, ch: usize) -> usize {
    text.char_indices().nth(ch).map_or(text.len(), |(i, _)| i)
}

fn skip_space(s: &[char], at: &mut usize) {
    while s.get(*at).is_some_and(|c| c.is_whitespace()) {
        *at += 1;
    }
}

fn value(s: &[char], at: &mut usize) -> Result<Value, String> {
    skip_space(s, at);
    match s.get(*at) {
        None => Err("a value is missing".to_string()),
        Some('"') => {
            let end = s[*at + 1..]
                .iter()
                .position(|&c| c == '"')
                .ok_or("a string is not closed")?;
            let v: String = s[*at + 1..*at + 1 + end].iter().collect();
            *at += end + 2;
            Ok(Value::Str(v))
        }
        Some('[') => {
            *at += 1;
            items(s, at, ']').map(Value::List)
        }
        Some('{') => {
            *at += 1;
            let mut pairs = Vec::new();
            skip_space(s, at);
            if s.get(*at) == Some(&'}') {
                *at += 1;
                return Ok(Value::Table(pairs));
            }
            loop {
                skip_space(s, at);
                let start = *at;
                while s
                    .get(*at)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                {
                    *at += 1;
                }
                let key: String = s[start..*at].iter().collect();
                if key.is_empty() {
                    return Err("a table key is missing".to_string());
                }
                skip_space(s, at);
                if s.get(*at) != Some(&'=') {
                    return Err(format!("`{key}`: expected `key = value`"));
                }
                *at += 1;
                pairs.push((key, value(s, at)?));
                skip_space(s, at);
                match s.get(*at) {
                    Some(',') => *at += 1,
                    Some('}') => {
                        *at += 1;
                        return Ok(Value::Table(pairs));
                    }
                    _ => return Err("a table is not closed with `}`".to_string()),
                }
            }
        }
        Some(_) => {
            let start = *at;
            while s
                .get(*at)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
            {
                *at += 1;
            }
            let word: String = s[start..*at].iter().collect();
            match word.as_str() {
                "" => Err(format!("`{}` is not a value", s[start])),
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                w => {
                    let n = match w.strip_prefix("0x") {
                        Some(hex) => u32::from_str_radix(&hex.replace('_', ""), 16),
                        None => w.replace('_', "").parse(),
                    };
                    n.map(Value::Int)
                        .map_err(|_| format!("`{w}` is not a whole number"))
                }
            }
        }
    }
}

/// The items of a list up to `close`, which `at` is just inside.
fn items(s: &[char], at: &mut usize, close: char) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    loop {
        skip_space(s, at);
        if s.get(*at) == Some(&close) {
            *at += 1;
            return Ok(out);
        }
        out.push(value(s, at)?);
        skip_space(s, at);
        match s.get(*at) {
            Some(',') => *at += 1,
            Some(c) if *c == close => {}
            _ => return Err(format!("a list is not closed with `{close}`")),
        }
    }
}

fn sync_table(v: &Value) -> Result<(String, u32), String> {
    let t = v.table("sync", &["script", "frame"])?;
    Ok((
        field(t, "script", "sync")?.string("script")?,
        field(t, "frame", "sync")?.int("frame")?,
    ))
}

fn swap_table(v: &Value) -> Result<Swap, String> {
    let t = v.table("swap", &["frame", "disk"])?;
    let disk = field(t, "disk", "swap")?.int("disk")?;
    if !(1..=2).contains(&disk) {
        return Err(format!("swap disk {disk}: the set has disks 1 and 2"));
    }
    Ok(Swap {
        frame: field(t, "frame", "swap")?.int("frame")?,
        disk,
    })
}

fn compare_list(v: &Value) -> Result<Compare, String> {
    let mut c = Compare::default();
    for what in v.strings("compare")? {
        let slot = match what.as_str() {
            "vars" => &mut c.vars,
            "picture" => &mut c.picture,
            other => {
                return Err(format!(
                    "compare = \"{other}\": this runner compares \"vars\" and \"picture\""
                ));
            }
        };
        if *slot {
            return Err(format!("compare: \"{what}\" is given twice"));
        }
        *slot = true;
    }
    if c == Compare::default() {
        return Err("`compare` is empty: the case would compare nothing".to_string());
    }
    Ok(c)
}

fn goal(v: &Value, sync_frame: u32, frames: u32) -> Result<Goal, String> {
    let t = v.table(
        "goal",
        &["all", "by_frame", "idle", "game_over", "level_end"],
    )?;
    if let Some((_, idle)) = t.iter().find(|(k, _)| k == "idle") {
        if t.len() != 1 || *idle != Value::Bool(true) {
            return Err("goal: `idle = true` stands alone".to_string());
        }
        return Ok(Goal::Idle);
    }
    let by_frame = |t: &[(String, Value)]| {
        let by_frame = field(t, "by_frame", "goal")?.int("by_frame")?;
        let end = sync_frame.saturating_add(frames);
        if by_frame <= sync_frame || by_frame > end {
            return Err(format!(
                "goal by_frame {by_frame}: must be after the sync frame {sync_frame} and at most \
                 its last frame {end}"
            ));
        }
        Ok(by_frame)
    };
    for (key, scene) in [
        ("game_over", Goal::GameOver { by_frame: 0 }),
        ("level_end", Goal::LevelEnd { by_frame: 0 }),
    ] {
        if let Some((_, on)) = t.iter().find(|(k, _)| k == key) {
            if *on != Value::Bool(true) || t.iter().any(|(k, _)| k != key && k != "by_frame") {
                return Err(format!(
                    "goal: `{key} = true` takes a `by_frame` and nothing else"
                ));
            }
            let by_frame = by_frame(t)?;
            return Ok(match scene {
                Goal::GameOver { .. } => Goal::GameOver { by_frame },
                _ => Goal::LevelEnd { by_frame },
            });
        }
    }
    let all = var_values(field(t, "all", "goal")?, "goal.all")?;
    if all.is_empty() {
        return Err("goal.all is empty: a goal with nothing to reach".to_string());
    }
    Ok(Goal::All {
        all,
        by_frame: by_frame(t)?,
    })
}

/// `[{ var = "NAME", value = N }, ...]`, each a `LAYOUT` variable.
fn var_values(v: &Value, what: &str) -> Result<Vec<(String, u32)>, String> {
    v.list(what)?
        .iter()
        .map(|item| {
            let p = item.table(what, &["var", "value"])?;
            let var = field(p, "var", what)?.string("var")?;
            known(&var)?;
            Ok((var, field(p, "value", what)?.int("value")?))
        })
        .collect()
}

fn exclusion(v: &Value) -> Result<Exclusion, String> {
    let t = v.table("exclude", &["what", "task"])?;
    let what = field(t, "what", "exclude")?.string("what")?;
    let task = field(t, "task", "exclude")?.string("task")?;
    let var = match what.strip_prefix("var ") {
        Some(name) => {
            known(name.trim())?;
            Some(name.trim().to_string())
        }
        None => None,
    };
    let range = |r: &str| {
        r.split_once("..")
            .and_then(|(a, b)| Some(a.trim().parse::<u16>().ok()?..b.trim().parse::<u16>().ok()?))
            .filter(|r| r.start < r.end)
    };
    let (rows, cols) = if var.is_some() {
        (0..0, None)
    } else {
        what.strip_prefix("rows ")
            .and_then(|r| match r.split_once(" cols ") {
                Some((r, c)) => Some((range(r)?, Some(range(c)?))),
                None => Some((range(r)?, None)),
            })
            .ok_or_else(|| {
                format!(
                    "exclude what = \"{what}\": expected \"rows A..B\", \"rows A..B cols C..D\", \
                     each A below B, or \"var NAME\""
                )
            })?
    };
    // Every exclusion names the task that removes it (porting-routines §4).
    let is_task = task
        .strip_prefix('T')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if !is_task {
        return Err(format!(
            "exclude task = \"{task}\": name the task that removes it, as `T10`"
        ));
    }
    Ok(Exclusion {
        rows,
        cols,
        var,
        what,
        task,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TITLE: &str = "# c\nsync = { script = \"s\", frame = 7 }\nswap = { frame = 3, disk = 2 }\n\
                         frames = 9 # c\ncompare = [\"vars\"]\nvars = [\"mt_counter\"]\n";

    const LEVEL: &str = "sync = { script = \"boot-to-level1\", frame = 6301 }\nframes = 100\n\
                         input = \"w1\"\ncompare = [\"vars\", \"picture\"]\n\
                         vars = [\"char_x\", \"camera_x\"]\n\
                         goal = { all = [{ var = \"char_x\", value = 0x80 }], by_frame = 6400 }\n\
                         exclude = [{ what = \"rows 180..250\", task = \"T10\" }]\n";

    #[test]
    fn a_whole_title_case_parses() {
        assert_eq!(
            Case::parse("n", TITLE).unwrap(),
            Case {
                name: "n".into(),
                script: "s".into(),
                sync_frame: 7,
                swap: Some(Swap { frame: 3, disk: 2 }),
                frames: 9,
                input: None,
                compare: Compare {
                    vars: true,
                    picture: false
                },
                vars: vec!["mt_counter".into()],
                goal: None,
                exclude: vec![],
                set: vec![],
            }
        );
    }

    #[test]
    fn a_whole_level_case_parses() {
        let c = Case::parse("l", LEVEL).unwrap();
        assert!(c.is_level());
        assert_eq!(c.input.as_deref(), Some("w1"));
        assert_eq!(
            c.compare,
            Compare {
                vars: true,
                picture: true
            }
        );
        assert_eq!(
            c.goal,
            Some(Goal::All {
                all: vec![("char_x".into(), 0x80)],
                by_frame: 6400
            })
        );
        assert_eq!(
            c.exclude,
            vec![Exclusion {
                rows: 180..250,
                cols: None,
                var: None,
                what: "rows 180..250".into(),
                task: "T10".into()
            }]
        );
        let idle = LEVEL.replace(
            "goal = { all = [{ var = \"char_x\", value = 0x80 }], by_frame = 6400 }",
            "goal = { idle = true }",
        );
        assert_eq!(Case::parse("l", &idle).unwrap().goal, Some(Goal::Idle));
        let over = LEVEL.replace(
            "goal = { all = [{ var = \"char_x\", value = 0x80 }], by_frame = 6400 }",
            "goal = { game_over = true, by_frame = 6400 }",
        );
        assert_eq!(
            Case::parse("l", &over).unwrap().goal,
            Some(Goal::GameOver { by_frame: 6400 })
        );
        let end = LEVEL.replace(
            "goal = { all = [{ var = \"char_x\", value = 0x80 }], by_frame = 6400 }",
            "goal = { level_end = true, by_frame = 6400 }\nset = [{ var = \"cheat_flag\", value = 1 }]",
        );
        let end = Case::parse("l", &end).unwrap();
        assert_eq!(end.goal, Some(Goal::LevelEnd { by_frame: 6400 }));
        assert_eq!(end.set, [("cheat_flag".to_string(), 1)]);
        for (goal, says) in [
            (
                "goal = { game_over = false, by_frame = 6400 }",
                "game_over = true",
            ),
            (
                "goal = { level_end = true, game_over = true, by_frame = 6400 }",
                "and nothing else",
            ),
            (
                "goal = { level_end = true, by_frame = 6400 }\nset = [{ var = \"nope\", value = 1 }]",
                "not in fd-core's LAYOUT",
            ),
            ("goal = { game_over = true }", "no `by_frame`"),
            ("goal = { game_over = true, by_frame = 6402 }", "at most"),
        ] {
            let text = LEVEL.replace(
                "goal = { all = [{ var = \"char_x\", value = 0x80 }], by_frame = 6400 }",
                goal,
            );
            let err = Case::parse("l", &text).unwrap_err();
            assert!(err.contains(says), "{goal}: {err}");
        }
    }

    fn refused(base: &str, from: &str, to: &str, says: &str) {
        let text = base.replace(from, to);
        assert_ne!(text, base, "{from}");
        let err = Case::parse("n", &text).unwrap_err();
        assert!(err.contains(says), "{to}: {err}");
    }

    #[test]
    fn what_the_title_format_does_not_say_is_refused() {
        for (from, to, says) in [
            ("frames = 9", "frames = 9\nresync = true", "resync"),
            ("frames = 9", "frames = many", "many"),
            ("frames = 9", "", "frames"),
            ("[\"vars\"]", "[\"vars\", \"sound\"]", "sound"),
            ("[\"vars\"]", "[\"vars\", \"vars\"]", "twice"),
            ("[\"vars\"]", "[\"vars\", \"picture\"]", "level cases"),
            ("[\"mt_counter\"]", "[\"lives_left\"]", "lives_left"),
            ("[\"mt_counter\"]", "[]", "compare nothing"),
            ("compare = [\"vars\"]", "compare = []", "compare nothing"),
            ("disk = 2", "disk = 3", "disks 1 and 2"),
            ("frame = 7", "frame = 7, df1 = true", "df1"),
            ("frames = 9", "frames = 9\nframes = 8", "twice"),
            ("frame = 3,", "frame = 7,", "before the sync frame"),
            ("frames = 9", "frames = 4294967295", "frame cap"),
            ("frames = 9", "frames = 9\nfreeze = true", "not a case key"),
            (
                "frames = 9",
                "frames = 9\ngoal = { idle = true }",
                "level cases",
            ),
            ("frames = 9", "frames = 9 }", "unexpected"),
        ] {
            refused(TITLE, from, to, says);
        }
    }

    #[test]
    fn what_the_level_format_does_not_say_is_refused() {
        let goal = "goal = { all = [{ var = \"char_x\", value = 0x80 }], by_frame = 6400 }";
        for (from, to, says) in [
            (goal, "", "states its `goal`"),
            ("frame = 6301", "frame = 6300", "play starts at frame 6301"),
            (
                "frames = 100",
                "frames = 100\nswap = { frame = 3600, disk = 2 }",
                "disk 2",
            ),
            ("by_frame = 6400", "by_frame = 6402", "at most"),
            ("by_frame = 6400", "by_frame = 6301", "after the sync"),
            (", by_frame = 6400", "", "no `by_frame`"),
            (
                "var = \"char_x\", value",
                "var = \"lives_left\", value",
                "lives_left",
            ),
            (
                "value = 0x80",
                "value = 0x80, at = 1",
                "`at` is not a goal.all key",
            ),
            (
                goal,
                "goal = { all = [], by_frame = 6400 }",
                "nothing to reach",
            ),
            (goal, "goal = { idle = false }", "idle = true"),
            (
                goal,
                "goal = { idle = true, by_frame = 6400 }",
                "stands alone",
            ),
            ("rows 180..250", "rows 250..180", "A below B"),
            ("rows 180..250", "columns 0..8", "rows A..B"),
            (
                "rows 180..250",
                "rows 180..250 cols 8..0",
                "rows A..B cols C..D",
            ),
            ("rows 180..250", "rows 180..250 cols", "rows A..B cols C..D"),
            ("rows 180..250", "var no_such", "not in fd-core's LAYOUT"),
            ("rows 180..250", "var char_x", "also in `vars`"),
            ("task = \"T10\"", "task = \"later\"", "name the task"),
            (", task = \"T10\"", "", "no `task`"),
            ("[\"vars\", \"picture\"]", "[\"vars\"]", "no \"picture\""),
            // T79: nothing is frozen any more, so the keys are gone.
            (
                "frames = 100",
                "frames = 100\nfreeze = true",
                "not a case key",
            ),
            (
                "frames = 100",
                "frames = 100\nthaw = [\"h0_681c\"]",
                "not a case key",
            ),
            (
                "[\"vars\", \"picture\"]\nvars = [\"char_x\", \"camera_x\"]\ngoal = { all = [{ var = \"char_x\", value = 0x80 }], by_frame = 6400 }",
                "[\"picture\"]\ngoal = { idle = true }",
                "judged on the case's `vars`",
            ),
            ("input = \"w1\"", "input = w1", "not a whole number"),
        ] {
            refused(LEVEL, from, to, says);
        }
    }

    #[test]
    fn a_box_of_rows_and_columns_is_excluded_with_its_task() {
        let text = LEVEL.replace("rows 180..250", "rows 223..231 cols 152..176");
        let c = Case::parse("l", &text).unwrap();
        assert_eq!(
            c.exclude,
            vec![Exclusion {
                rows: 223..231,
                cols: Some(152..176),
                var: None,
                what: "rows 223..231 cols 152..176".into(),
                task: "T10".into()
            }]
        );
    }

    #[test]
    fn a_variable_is_excluded_by_name_with_its_task() {
        let text = LEVEL.replace("rows 180..250", "var countdown");
        let c = Case::parse("l", &text).unwrap();
        assert_eq!(
            c.exclude,
            vec![Exclusion {
                rows: 0..0,
                cols: None,
                var: Some("countdown".into()),
                what: "var countdown".into(),
                task: "T10".into()
            }]
        );
        // A variable exclusion needs no picture, but does need compared variables.
        let state_only = text.replace("[\"vars\", \"picture\"]", "[\"vars\"]");
        assert!(Case::parse("l", &state_only).is_ok());
    }

    #[test]
    fn a_hash_inside_a_string_is_not_a_comment() {
        assert_eq!(strip_comment("a = \"x#y\" # z"), "a = \"x#y\" ");
        assert_eq!(
            parse_value("[{ a = 1, b = \"x\" }, 0x10, true]").unwrap(),
            Value::List(vec![
                Value::Table(vec![
                    ("a".into(), Value::Int(1)),
                    ("b".into(), Value::Str("x".into()))
                ]),
                Value::Int(16),
                Value::Bool(true)
            ])
        );
    }
}
