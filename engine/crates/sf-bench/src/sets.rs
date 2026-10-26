//! The benchmark sets (DESIGN §4): the 24 normal levels (D1: not the bonus level, not the witch),
//! each from a start.
//!
//! - A: each level's `AtLevel` sync point, timed on the original.
//! - B: each level's leg of the whole game, from the snapshot `examples/whole_game` saves at the
//!   leg's start (`leg-NN.snap`), carrying the earlier legs' state.
//! - C: each level as Game ▸ Start Level… starts it, timed on the engine (`--app`, T12).

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use sf_core::LevelId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Set {
    A,
    B,
    C,
}

impl fmt::Display for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Set::A => "A",
            Set::B => "B",
            Set::C => "C",
        })
    }
}

impl FromStr for Set {
    type Err = String;
    fn from_str(s: &str) -> Result<Set, String> {
        match s {
            "A" | "a" => Ok(Set::A),
            "B" | "b" => Ok(Set::B),
            "C" | "c" => Ok(Set::C),
            _ => Err(format!("set {s}? (A, B or C)")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartKind {
    /// The level's `AtLevel` sync point.
    AtLevel,
    /// A whole-game leg snapshot, and the frame of the whole game it was taken at (its
    /// `Snapshot::frame`), which is the leg's `AtFrame` sync point.
    Snap(PathBuf, u32),
    /// `Game::start_level` (set C, T12).
    App,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    pub level: LevelId,
    pub kind: StartKind,
}

/// The 24 normal levels, in the order the game plays them.
pub const LEVELS: [&str; 24] = [
    "W1-1", "W1-2", "W1-3", "W1-4", "W2-1", "W2-2", "W2-3", "W2-4", "W3-1", "W3-2", "W3-3", "W3-4",
    "W4-1", "W4-2", "W4-3", "W4-4", "W5-1", "W5-2", "W5-3", "W5-4", "W6-1", "W6-2", "W6-3", "W6-4",
];

pub fn levels() -> Vec<LevelId> {
    LEVELS.iter().map(|l| l.parse().unwrap()).collect()
}

/// A normal level's index among `whole_game`'s legs (its `leg-NN.snap`): the bonus level (20)
/// comes between W5-4 and W6-1, the witch (25) last.
pub fn leg_index(level: &LevelId) -> usize {
    let i = LEVELS
        .iter()
        .position(|l| *l == level.to_string())
        .unwrap_or_else(|| panic!("{level} is not a normal level"));
    if i >= 20 { i + 1 } else { i }
}

/// The leg the whole-game run stops before so that every normal leg's snapshot is saved
/// (`whole_game --until` stops before saving that leg's own: `--until W6-4` would leave W6-4 out).
pub const LEGS_UNTIL: &str = "WW-1";

/// Where the leg snapshots are kept: beside the oracle's caches, shared by all worktrees.
pub fn default_legs_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    home.join(".cache/superfrog-re/bench-legs")
}

pub fn leg_path(dir: &Path, level: &LevelId) -> PathBuf {
    dir.join(format!("leg-{:02}.snap", leg_index(level)))
}

/// The set's starts, in level order; set B's from the leg snapshots in `legs`, which must exist.
pub fn starts_from(set: Set, legs: &Path) -> Result<Vec<Start>, String> {
    levels()
        .into_iter()
        .map(|level| {
            let kind = match set {
                Set::A => StartKind::AtLevel,
                Set::C => StartKind::App,
                Set::B => {
                    let p = leg_path(legs, &level);
                    let snap = sf_conform::sync::load_snapshot_file(&p)?;
                    StartKind::Snap(p, snap.frame)
                }
            };
            Ok(Start { level, kind })
        })
        .collect()
}

/// Whether every normal leg's snapshot is in `legs`.
pub fn legs_present(legs: &Path) -> bool {
    levels().iter().all(|l| leg_path(legs, l).is_file())
}

/// The set's starts, in level order. For set B the leg snapshots are made first if any is
/// missing: `whole_game --until WW-1` run once into `legs` (about 10 minutes on the oracle).
pub fn starts(set: Set) -> Vec<Start> {
    starts_in(set, &default_legs_dir())
}

pub fn starts_in(set: Set, legs: &Path) -> Vec<Start> {
    if set == Set::B && !legs_present(legs) {
        make_legs(legs).unwrap_or_else(|e| panic!("{e}"));
    }
    starts_from(set, legs).unwrap_or_else(|e| panic!("{e}"))
}

/// Runs `whole_game` to save every leg's snapshot in `legs`. Its script goes to the same folder,
/// never over the committed `game-whole.txt`.
pub fn make_legs(legs: &Path) -> Result<(), String> {
    std::fs::create_dir_all(legs).map_err(|e| format!("{}: {e}", legs.display()))?;
    let bin = crate::adapters::build(&[("sf-conform", "example", "whole_game")])?;
    eprintln!("sf-bench: saving the whole game's leg snapshots in {} (once)", legs.display());
    let mut cmd = std::process::Command::new(&bin[0]);
    cmd.arg("--scratch")
        .arg(legs)
        .arg("--until")
        .arg(LEGS_UNTIL)
        .arg("--out")
        .arg(legs.join("game-whole-until.txt"))
        .current_dir(crate::engine_dir())
        .stdout(std::process::Stdio::null());
    let run = crate::child::run(cmd, std::time::Duration::from_secs(90 * crate::MINUTE), None)?;
    if run.timed_out || !run.status.is_some_and(|s| s.success()) || !legs_present(legs) {
        return Err(format!(
            "whole_game did not save every leg in {} ({:?})",
            legs.display(),
            run.status
        ));
    }
    Ok(())
}
