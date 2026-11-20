//! The planner's pure core. Guarded by `tests/purity.rs`: if it fails, move the code to the shell
//! side (`run`); never relax the test.

pub mod archive;
pub mod explore;
pub mod graph;
pub mod guide;
pub mod repair;
pub mod shorten;
pub mod walk;

pub use amiga_input::{FrameInput, Joy};
pub use archive::{Archive, CellKey, band, cell_key};
pub use explore::{Compass, Explorer, Goal, Options, Part, PartResult, Progress, Reason, Route};
pub use graph::{Edge, MoveMap, Node, shortest};
pub use guide::{GuideIndex, GuidePoint, PACE_TAU, Pace};

/// A map cell, `(col, row)`, top-left origin.
pub type Cell = (u16, u16);

/// What the planner needs of a game: its engine stepped a frame at a time, and what it reads off a
/// state. Implemented by a game's planner crate on a newtype around its engine (the trait and the
/// engine live in two other crates). `Clone` is the planner's snapshot: a clone must step exactly as
/// the original would.
pub trait PlanGame: Clone + Send + Sync {
    /// One frame of play with `i` held.
    fn step(&mut self, i: FrameInput);
    /// The bytes one stored state costs the archive's memory count. Measured once, from the start.
    fn state_bytes(&self) -> u64;
    /// The player's cell on the map.
    fn cell(&self) -> Cell;
    /// Lives left, compared signed: a game-over mark below zero reads as fewer, a life gained as
    /// more, never as a loss.
    fn lives(&self) -> i16;
    /// The player is dying (a life is about to go).
    fn dying(&self) -> bool;
    /// Health left within the current life; a route that loses less of it wins.
    fn health(&self) -> u16;
    /// The level is being played: false once it has ended (an exit, or anything else ending it).
    fn playing(&self) -> bool;
    /// Collectibles still needed before the exit opens.
    fn collectibles_left(&self) -> u16;
    /// A flag the player holds that changes what moves can do (an item carried); part of a cell's
    /// key and a movement map node.
    fn held_flag(&self) -> bool;
    /// On ground or a platform top, at any speed.
    fn landed(&self) -> bool;
    /// Whether the player can pass through cell `c` as the map now stands (not a wall, not a fixed
    /// hazard). Read once, from the explorer's start, by the walk compass.
    fn open(&self, c: Cell) -> bool;
    /// Whether the collectible at `c` (one of `Facts::collectibles`) is still there.
    fn collectible_live(&self, c: Cell) -> bool;
    /// The switches pressed now, a bit per `Facts::gates` index.
    fn pressed(&self, f: &Facts) -> u32;
    /// The joystick states a random burst draws from.
    const ACTS: &'static [Joy];
}

/// A level as the planner sees it, fixed for a search: the map's size, where the exits are, the
/// collectibles that count toward opening the exit, and per switch the cells its press opens.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    pub cols: u16,
    pub rows: u16,
    pub exits: Vec<Cell>,
    pub collectibles: Vec<Cell>,
    /// Per switch: the cells closed before its press and open after it.
    pub gates: Vec<Vec<Cell>>,
}
