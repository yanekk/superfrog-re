//! The smart-planner benchmark (plans/smart-planner DESIGN §4.2): a planner run on a benchmark set
//! under the unaided rules, each route verified as the completion cases are verified (§4.1), each
//! level's numbers appended to a resumable results file (§3.4), and the side-by-side table
//! rendered from it.
//!
//! Every planner, old and new, runs as a child process ([`child`]), so the benchmark never links
//! what it measures and a planner that ignores its budget can be killed.

pub mod adapters;
pub mod child;
pub mod results;
pub mod run;
pub mod sets;
pub mod table;
pub mod verify;

/// Seconds in a minute, for budgets given in minutes.
pub const MINUTE: u64 = 60;

/// `engine/`.
pub fn engine_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The repository's root.
pub fn repo_dir() -> std::path::PathBuf {
    engine_dir().join("..")
}
