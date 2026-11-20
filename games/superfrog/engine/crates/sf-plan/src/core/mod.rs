//! The planner's pure core (DESIGN §3.1, §3.2). Guarded by `tests/purity.rs`: if it fails, move the
//! code to the shell side; never relax the test.
//!
//! The explorer, its archive, guides, compasses, shortening and repair moved to the toolkit's
//! `amiga-plan` (forest-dumb-level-1 T17). What stays is Superfrog's: its level facts and the
//! `Frog` newtype the toolkit plays, the movement map builder and its moves, the coin choice and
//! danger. The moved modules are re-exported under their old paths, each with Superfrog-typed
//! wrappers (a `Game` and these `Facts` in, as before), so callers did not change.

pub mod archive;
pub mod coins;
pub mod danger;
pub mod explore;
pub mod facts;
pub mod graph;
pub mod guide;
pub mod map;
pub mod moves;
pub mod repair;
pub mod shorten;
pub mod walk;

pub use archive::{Archive, CellKey, band, cell_key};
pub use coins::{Choice, Dist, OnTheWay, Target, choose, far_cost};
pub use danger::{Outcome, WAITS, heart_cost, play, play_safe};
pub use explore::{Compass, Explorer, Goal, Options, Part, PartResult, Progress, Reason, Route};
pub use facts::{Cell, Coin, CoinKind, Facts, Frog, Switch, facts, frog_cell, landed, standing};
pub use graph::{Edge, MoveMap, Node, shortest};
pub use guide::{GuideIndex, GuidePoint, PACE_TAU, Pace};
pub use map::{Builder, MapPart, build, build_in};
pub use moves::{Dir, MoveFamily, MoveSet, default_moves};
