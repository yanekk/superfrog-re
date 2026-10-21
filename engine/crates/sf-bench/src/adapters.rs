//! How each planner is started (DESIGN §4.2, D10). The old tools get only the level, `--snap`,
//! `--out`, `--frames` and, where the tool needs it to take coins at all, `--coins 0`: none of
//! the hints and per-level options (`--via/--avoid/--ground/--jump/--launch/--fly/--skip/--to/
//! --stomp/--hurt`), because the benchmark measures them unaided.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::sets::{Set, Start, StartKind};

/// The longest route any planner may write (`sf-plan --frames`'s default, DESIGN §5.2): W4-4's
/// committed route is ~24 000 frames, so the old tools' own defaults (12 000, 20 000) are raised.
pub const FRAMES: u32 = 30_000;

pub trait Adapter: Send + Sync {
    fn name(&self) -> &str;
    /// The cargo target the planner is: (package, `bin` or `example`, name).
    fn target(&self) -> (&'static str, &'static str, &'static str);
    /// The planner's command for `s`, writing its route to `out`. `seed` and `budget` reach only a
    /// planner that takes them.
    fn command(&self, s: &Start, seed: u32, out: &Path, budget: Duration) -> Command;
}

/// The built binary of a cargo target.
pub fn binary(target: (&str, &str, &str)) -> PathBuf {
    let dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::engine_dir().join("target"));
    match target.1 {
        "example" => dir.join("release/examples").join(target.2),
        _ => dir.join("release").join(target.2),
    }
}

/// Builds the targets (release), once before any is timed, so no build lands in a level's
/// wall-clock. Returns their binaries.
pub fn build(targets: &[(&str, &str, &str)]) -> Result<Vec<PathBuf>, String> {
    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.args(["build", "--release", "-q"]).current_dir(crate::engine_dir());
    for (pkg, kind, name) in targets {
        cmd.args(["-p", pkg, &format!("--{kind}"), name]);
    }
    let st = cmd.status().map_err(|e| format!("cargo build: {e}"))?;
    if !st.success() {
        return Err(format!("cargo build of {targets:?} failed ({st})"));
    }
    Ok(targets.iter().map(|&t| binary(t)).collect())
}

/// An old tool: an example of `sf-conform`.
pub struct OldTool {
    pub name: &'static str,
    /// It takes coins only when asked (`plan_exit` steers for the exit alone without `--coins 0`;
    /// `plan_exit_w6` and `explore` add that step themselves).
    pub coins: bool,
}

pub const PLAN_EXIT: OldTool = OldTool {
    name: "plan_exit",
    coins: true,
};
pub const PLAN_EXIT_W6: OldTool = OldTool {
    name: "plan_exit_w6",
    coins: false,
};
pub const EXPLORE: OldTool = OldTool {
    name: "explore",
    coins: false,
};

/// The three, in the order `best-old` runs them.
pub const OLD: [&OldTool; 3] = [&PLAN_EXIT, &PLAN_EXIT_W6, &EXPLORE];

impl Adapter for OldTool {
    fn name(&self) -> &str {
        self.name
    }

    fn target(&self) -> (&'static str, &'static str, &'static str) {
        ("sf-conform", "example", self.name)
    }

    fn command(&self, s: &Start, _seed: u32, out: &Path, _budget: Duration) -> Command {
        let mut c = Command::new(binary(self.target()));
        c.arg(s.level.to_string());
        match &s.kind {
            StartKind::AtLevel => {}
            StartKind::Snap(p, _) => {
                c.arg("--snap").arg(p);
            }
            StartKind::App => panic!("{} has no app mode (set C is the new planner's)", self.name),
        }
        if self.coins {
            c.args(["--coins", "0"]);
        }
        c.args(["--frames", &FRAMES.to_string()]).arg("--out").arg(out);
        c.current_dir(crate::engine_dir());
        c
    }
}

/// The new planner, `sf-plan` (T11 builds it; its options are DESIGN §2.1).
pub struct NewPlanner {
    /// `--follow-standalone`: on set B, the leg's standalone committed route as `--follow`
    /// (route repair, DESIGN §2.7).
    pub follow_standalone: bool,
}

impl Adapter for NewPlanner {
    fn name(&self) -> &str {
        "new"
    }

    fn target(&self) -> (&'static str, &'static str, &'static str) {
        ("sf-plan", "bin", "sf-plan")
    }

    fn command(&self, s: &Start, seed: u32, out: &Path, budget: Duration) -> Command {
        let mut c = Command::new(binary(self.target()));
        c.arg(s.level.to_string());
        match &s.kind {
            StartKind::AtLevel => {}
            StartKind::Snap(p, _) => {
                c.arg("--snap").arg(p);
                if self.follow_standalone {
                    let script = crate::verify::committed_script(Set::A, &s.level);
                    c.arg("--follow").arg(sf_conform::case::script_path(&script));
                }
            }
            StartKind::App => {
                c.arg("--app");
            }
        }
        c.args(["--seed", &seed.to_string()])
            .args(["--budget-min", &(budget.as_secs() / crate::MINUTE).to_string()])
            .arg("--no-cache")
            .arg("--out")
            .arg(out);
        c.current_dir(crate::engine_dir());
        c
    }
}
