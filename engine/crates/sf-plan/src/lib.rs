//! The smart planner (plans/smart-planner): one planner steered by a measured movement map.
//!
//! `core` is pure (DESIGN §3.1): no clock, file, environment, process, network or oracle, so all
//! of it is testable in milliseconds and reproducible to the byte. The shell (CLI, oracle checks,
//! cache files, threads) lives beside it.

pub mod core;
