//! The smart planner (plans/smart-planner): one planner, a whole-level explorer on the engine
//! (DESIGN §2.4, D19).
//!
//! `core` is pure (DESIGN §3.1): no clock, file, environment, process, network or oracle, so all
//! of it is testable in milliseconds and reproducible to the byte. The shell (CLI, oracle checks,
//! cache files, threads) lives beside it.

pub mod cache;
pub mod core;
pub mod guide_file;
pub mod oracle;
pub mod run;
pub mod threads;
pub mod trace;
