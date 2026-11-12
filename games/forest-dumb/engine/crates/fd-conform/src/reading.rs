//! A reading of many cases on one engine (`fd-conform run-all`, translator T29): each case's result
//! and first difference, grouped by the task that works from it and sorted by first difference, as
//! one line a case and as Markdown.

use std::collections::BTreeMap;

use crate::{Case, Options, Outcome, Verdict};

/// Which engine a case runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// fd-core's hand engine, by the pass runner (`run`).
    Hand,
    /// The translated program, on the frame bar (`translated_engine::run_translated`).
    Translated,
}

impl EngineKind {
    pub fn parse(s: &str) -> Option<EngineKind> {
        match s {
            "hand" => Some(EngineKind::Hand),
            "translated" => Some(EngineKind::Translated),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            EngineKind::Hand => "hand",
            EngineKind::Translated => "translated",
        }
    }
}

/// `case` on `engine`.
pub fn run_on(case: &Case, engine: EngineKind) -> Result<Verdict, String> {
    match engine {
        EngineKind::Hand => crate::run(case),
        EngineKind::Translated => crate::translated_engine::run_translated(
            case,
            &Options {
                every_channel: true,
                ..Options::default()
            },
        ),
    }
}

/// The task that works from a level 1 case's translated result, and the case's group, by its name
/// (translator PLAN: T30 character, camera, stretch, picture, idle; T31 the rest of play).
pub fn group(case: &str) -> (&'static str, &'static str) {
    const BY_PREFIX: [(&str, &str, &str); 16] = [
        ("power-on-", "T32", "power-on"),
        ("first-case", "T32", "title"),
        ("character-", "T30", "character"),
        ("stub-level", "T30", "character"),
        ("camera-", "T30", "camera"),
        ("stretch-", "T30", "stretch"),
        ("picture-", "T30", "picture"),
        ("w1-idle", "T30", "idle"),
        ("collect-", "T31", "collect"),
        ("contact-", "T31", "contact"),
        ("death-", "T31", "death"),
        ("enemy-", "T31", "enemy"),
        ("mechanic-", "T31", "mechanic"),
        ("level-end", "T31", "level end"),
        ("route-", "T31", "routes"),
        ("edge-latch-route", "T31", "routes"),
    ];
    BY_PREFIX
        .iter()
        .find(|(p, _, _)| case.starts_with(p))
        .map_or(("-", "other"), |&(_, t, g)| (t, g))
}

/// One case's result.
pub struct Row {
    pub case: String,
    pub verdict: Result<Verdict, String>,
}

impl Row {
    pub fn new(case: &str, verdict: Result<Verdict, String>) -> Row {
        Row {
            case: case.to_string(),
            verdict,
        }
    }

    pub fn passed(&self) -> bool {
        self.verdict.as_ref().is_ok_and(Verdict::passed)
    }

    /// `pass (N frames, P pictures)`, `fail: <the outcome>` or `not run: <why>`, on one line.
    pub fn result(&self) -> String {
        match &self.verdict {
            Ok(v) if v.passed() => format!("pass ({} states, {} pictures)", v.passes, v.pictures),
            Ok(v) => format!("fail: {}", one_line(&v.outcome.to_string())),
            Err(e) => format!("not run: {}", one_line(e)),
        }
    }

    /// The first difference's kind, the key the reading is grouped and sorted by: `stop: <how>`,
    /// `state: <var>`, `picture`, `Paula writes`, and so on; `pass`, `not run`.
    pub fn kind(&self) -> String {
        let Ok(v) = &self.verdict else {
            return "not run".to_string();
        };
        kind(&v.outcome)
    }

    /// The first difference of each other channel ([`Verdict::firsts`] after the first), as
    /// `<kind> at frame N`, joined; empty if none.
    pub fn then(&self) -> String {
        let Ok(v) = &self.verdict else {
            return String::new();
        };
        v.firsts
            .iter()
            .skip(1)
            .map(|o| match frame_of(o) {
                Some((pass, _)) => format!("{} at frame {pass}", kind(o)),
                None => kind(o),
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// The oracle frame of the first difference, if it names one.
    pub fn frame(&self) -> Option<u32> {
        frame_of(&self.verdict.as_ref().ok()?.outcome).map(|(_, f)| f)
    }

    /// Engine frames (or passes) run before the first difference.
    fn pass(&self) -> Option<u32> {
        frame_of(&self.verdict.as_ref().ok()?.outcome).map(|(p, _)| p)
    }
}

/// An outcome's kind: `stop: <how>`, `state: <var>`, `picture`, `Paula writes`, and so on.
fn kind(o: &Outcome) -> String {
    match o {
        Outcome::Same => "pass".to_string(),
        Outcome::Stopped { stop, .. } => format!("stop: {stop}"),
        Outcome::Differs { var, .. } => format!("state: {var}"),
        Outcome::PictureDiffers { .. } => "picture".to_string(),
        Outcome::WritesDiffer { .. } => "Paula writes".to_string(),
        Outcome::CustomWritesDiffer { .. } => "custom writes".to_string(),
        Outcome::GoalMissed { .. } => "goal missed".to_string(),
        Outcome::SceneDiffers { .. } => "scene".to_string(),
        Outcome::ComparedNothing { .. } => "compared nothing".to_string(),
        Outcome::NotIdle { .. } => "not idle".to_string(),
        Outcome::Unported { .. } => "unported object".to_string(),
        Outcome::ExclusionUnused { .. } => "exclusion unused".to_string(),
        Outcome::LoadApart { .. } => "loads apart".to_string(),
    }
}

/// `(pass, oracle frame)` an outcome names.
fn frame_of(o: &Outcome) -> Option<(u32, u32)> {
    match o {
        Outcome::Stopped { pass, frame, .. }
        | Outcome::Differs { pass, frame, .. }
        | Outcome::PictureDiffers { pass, frame, .. }
        | Outcome::WritesDiffer { pass, frame, .. }
        | Outcome::CustomWritesDiffer { pass, frame, .. }
        | Outcome::LoadApart { pass, frame, .. }
        | Outcome::Unported { pass, frame, .. } => Some((*pass, *frame)),
        _ => None,
    }
}

fn one_line(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

/// Every case's row on one engine.
pub struct Reading {
    pub engine: &'static str,
    pub rows: Vec<Row>,
}

impl Reading {
    pub fn all_passed(&self) -> bool {
        self.rows.iter().all(Row::passed)
    }

    /// `N cases on the <engine> engine: P pass, F fail, K not run`.
    pub fn summary(&self) -> String {
        let pass = self.rows.iter().filter(|r| r.passed()).count();
        let not_run = self.rows.iter().filter(|r| r.verdict.is_err()).count();
        format!(
            "{} cases on the {} engine: {pass} pass, {} fail, {not_run} not run",
            self.rows.len(),
            self.engine,
            self.rows.len() - pass - not_run
        )
    }

    /// The reading as Markdown: the summary, the count of each first difference, then a table a
    /// task, its cases sorted by first difference, then frame, then name.
    pub fn markdown(&self) -> String {
        let mut out = format!(
            "# Level 1 cases on the {} engine\n\nWritten by `fd-conform run-all --engine {}`. {}.\n\n",
            self.engine,
            self.engine,
            self.summary()
        );
        let mut kinds: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        for r in &self.rows {
            kinds.entry(r.kind()).or_default().push(&r.case);
        }
        out += "## First differences\n\n| First difference | Cases | Which |\n|---|---|---|\n";
        for (k, cases) in &kinds {
            out += &format!("| {} | {} | {} |\n", one_line(k), cases.len(), cases.join(", "));
        }
        let mut tasks: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
        for r in &self.rows {
            tasks.entry(group(&r.case).0).or_default().push(r);
        }
        for (task, mut rows) in tasks {
            rows.sort_by_key(|r| (r.kind(), r.frame(), r.case.clone()));
            let title = match task {
                "T30" => "T30: character, camera, stretch, picture, idle",
                "T31" => "T31: collect, contact, death, enemy, mechanic, level end, routes",
                _ => "Other cases",
            };
            out += &format!(
                "\n## {title}\n\n| Case | Group | First difference | Frame | Oracle frame | Then | Result |\n\
                 |---|---|---|---|---|---|---|\n"
            );
            for r in rows {
                let n = |x: Option<u32>| x.map_or("-".to_string(), |x| x.to_string());
                out += &format!(
                    "| {} | {} | {} | {} | {} | {} | {} |\n",
                    r.case,
                    group(&r.case).1,
                    one_line(&r.kind()),
                    n(r.pass()),
                    n(r.frame()),
                    one_line(&r.then()),
                    r.result()
                );
            }
        }
        out
    }
}
