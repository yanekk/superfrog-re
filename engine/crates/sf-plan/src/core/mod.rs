//! The planner's pure core (DESIGN §3.1, §3.2). Guarded by `tests/purity.rs`: if it fails, move the
//! code to the shell side; never relax the test.

pub mod coins;
pub mod facts;
pub mod graph;

pub use coins::{Choice, Dist, OnTheWay, Target, choose, far_cost};
pub use facts::{Cell, Coin, CoinKind, Facts, Switch, facts, frog_cell, landed, standing};
pub use graph::{Edge, MoveMap, Node, shortest};
