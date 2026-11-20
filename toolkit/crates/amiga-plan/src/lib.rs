//! The route planner's game-free half: a whole-level explorer over a game's engine, its archive of
//! cells, guides, the walk and map compasses, shortening, and the run shell that puts a round on
//! threads under a wall-clock budget.
//!
//! A game plugs in by implementing [`PlanGame`] on its engine (a newtype in its own planner crate)
//! and handing the explorer its level [`Facts`]. `core` is pure: no clock, file, environment,
//! process or network (tests/purity.rs), so a search is a function of its start, facts and options
//! and is reproducible to the byte on any thread count.

pub mod core;
pub mod guide_file;
pub mod run;

pub use crate::core::*;
