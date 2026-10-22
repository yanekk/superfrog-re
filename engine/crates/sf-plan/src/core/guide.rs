//! Guide paths (DESIGN §2.11, D21): cells to pass near in order, and the choice weight they give an
//! archived cell. T08 declares the types and the call from `explore`; T09 fills the bodies.

use super::archive::CellKey;
use super::facts::Cell;

/// One point of a guide path: a frog cell, and the coins left there when known (a point without
/// them matches any band).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GuidePoint {
    pub cell: Cell,
    pub coins_left: Option<u16>,
}

/// The guide's lookup: (coarse col, coarse row, band | any) → the furthest point index there.
#[derive(Debug, Clone, Default)]
pub struct GuideIndex {
    points: Vec<GuidePoint>,
}

impl GuideIndex {
    pub fn new(points: &[GuidePoint]) -> Self {
        GuideIndex {
            points: points.to_vec(),
        }
    }

    /// The choice weight factor for `key`, `p_max` the furthest index any archived cell has.
    /// Stub until T09: 1.0, so a guide changes nothing yet.
    pub fn weight(&self, key: &CellKey, p_max: u32) -> f64 {
        let _ = (key, p_max, &self.points);
        1.0
    }

    /// The furthest guide point whose region `key` falls in. Stub until T09.
    pub fn index(&self, key: &CellKey) -> Option<u32> {
        let _ = key;
        None
    }
}
