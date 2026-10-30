//! The route viewer (`sf-routeview`): plays app-timed routes on the engine and writes a web page
//! drawing each over its level map: the frog's path, every coin, collectible and enemy, the events
//! along the way, a timeline, and a switcher over the smart planner's routes with their verdicts.
//! `sf-bench` runs it after each set-C run of the new planner.

pub mod dump;
pub mod guide;
pub mod json;
pub mod site;
pub mod verdict;

use std::path::{Path, PathBuf};

use sf_bench::results::LevelResult;
use sf_core::LevelId;

/// The 24 normal levels, in play order.
pub fn levels() -> Vec<LevelId> {
    (1..=6).flat_map(|w| (1..=4).map(move |l| format!("W{w}-{l}").parse().unwrap())).collect()
}

/// The new planner's latest guided set-C result per level in `results`.
pub fn planner_results(results: &Path) -> Vec<LevelResult> {
    let mut latest: Vec<LevelResult> = Vec::new();
    for r in sf_bench::results::load(results) {
        if r.planner == "new" && r.set == "C" && r.guided {
            latest.retain(|o| o.level != r.level);
            latest.push(r);
        }
    }
    latest
}

/// Which route file `--planner` draws for `level`, and what the page calls it: the latest
/// `sf-bench` run's (`routes/new-C-<level>.txt`, its best partial route when it failed, kept on
/// disk and git-ignored), else the committed one (`routes/app/<level>.txt`), else a re-run's
/// partial route in `partials`.
pub fn planner_route(bench_dir: &Path, partials: &Path, level: &str) -> Option<(PathBuf, &'static str)> {
    [
        (bench_dir.join("routes").join(format!("new-C-{level}.txt")), "the latest sf-bench run's route"),
        (bench_dir.join("routes/app").join(format!("{level}.txt")), "the committed route"),
        (partials.join(format!("{level}.txt")), "a partial route from a re-run, not the scored run"),
    ]
    .into_iter()
    .find(|(p, _)| p.is_file())
}

/// The level a route file is for: its name (`W2-3.txt`, `new-C-W2-3.txt`) or its header lines
/// (`# W2-3 by sf-plan …`).
pub fn level_of(path: &Path, text: &str) -> Option<LevelId> {
    let header: String = text.lines().take_while(|l| l.starts_with('#')).collect::<Vec<_>>().join(" ");
    let name = path.file_name()?.to_string_lossy().to_uppercase().replace("ALT", "alt");
    [name, header].iter().find_map(|s| find_level(s))
}

/// The first level id (`W1-4`, `W6-2alt`, `WW-1`) in `s` that is not inside a longer word.
fn find_level(s: &str) -> Option<LevelId> {
    let b = s.as_bytes();
    (0..b.len()).filter(|&i| b[i] == b'W' && (i == 0 || !b[i - 1].is_ascii_alphanumeric())).find_map(|i| {
        [7, 4].iter().find_map(|&n| {
            let w = s.get(i..i + n)?;
            let after = b.get(i + n).copied().unwrap_or(b' ');
            (!after.is_ascii_alphanumeric()).then(|| w.parse().ok()).flatten()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_come_from_names_or_headers() {
        let id = |p: &str, t: &str| level_of(Path::new(p), t).map(|l| l.to_string());
        assert_eq!(id("/x/W2-3.txt", ""), Some("W2-3".into()));
        assert_eq!(id("new-C-W6-4.txt", ""), Some("W6-4".into()));
        assert_eq!(id("w64-ordered-s3.txt", "# timing: app\n# W6-4 by sf-plan, seed 3\n0: -"), Some("W6-4".into()));
        assert_eq!(id("route.txt", "# timing: app\n0: -"), None);
    }

    #[test]
    fn the_latest_run_wins_then_the_committed_route_then_a_rerun() {
        let dir = std::env::temp_dir().join(format!("sf-routeview-lib-{}", std::process::id()));
        let (bench, partials) = (dir.join("bench"), dir.join("runs"));
        std::fs::create_dir_all(bench.join("routes/app")).unwrap();
        std::fs::create_dir_all(&partials).unwrap();
        assert_eq!(planner_route(&bench, &partials, "W1-1"), None);
        std::fs::write(partials.join("W1-1.txt"), "").unwrap();
        assert_eq!(planner_route(&bench, &partials, "W1-1").unwrap().1, "a partial route from a re-run, not the scored run");
        std::fs::write(bench.join("routes/app/W1-1.txt"), "").unwrap();
        assert_eq!(planner_route(&bench, &partials, "W1-1").unwrap().1, "the committed route");
        std::fs::write(bench.join("routes/new-C-W1-1.txt"), "").unwrap();
        assert_eq!(planner_route(&bench, &partials, "W1-1").unwrap().1, "the latest sf-bench run's route");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
