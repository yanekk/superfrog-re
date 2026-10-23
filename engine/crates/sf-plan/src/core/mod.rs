//! The planner's pure core (DESIGN §3.1, §3.2). Guarded by `tests/purity.rs`: if it fails, move the
//! code to the shell side; never relax the test.

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

pub use archive::{Archive, CellKey, band, cell_key};
pub use coins::{Choice, Dist, OnTheWay, Target, choose, far_cost};
pub use danger::{Outcome, WAITS, heart_cost, play, play_safe};
pub use explore::{Compass, Explorer, Goal, Options, Part, PartResult, Progress, Reason, Route};
pub use facts::{Cell, Coin, CoinKind, Facts, Switch, facts, frog_cell, landed, standing};
pub use graph::{Edge, MoveMap, Node, shortest};
pub use guide::{GuideIndex, GuidePoint, PACE_TAU, Pace};
pub use map::{Builder, MapPart, build, build_in};
pub use moves::{Dir, MoveFamily, MoveSet, default_moves};
