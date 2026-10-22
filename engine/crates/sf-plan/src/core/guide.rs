//! Guide paths (DESIGN §2.11, D21): cells to pass near in order, and the choice weight they give an
//! archived cell. Each point marks a region, its coarse cell (the archive key's col/2, row/2) ±1,
//! in its coins band when its coins are known and in any band when not; a cell takes the furthest
//! point index whose region it falls in. The constants are the experiment's
//! (`spike-explore/goexp.rs`, `guide_from`); T14 tunes them.

use std::collections::HashMap;

use super::archive::{CellKey, band};
use super::facts::Cell;

/// The weight on the path is `ON_PATH · e^(−(p_max − p) / FALLOFF)`.
const ON_PATH: f64 = 3.0;
const FALLOFF: f64 = 20.0;
/// Off the path: a guide from another start or mode may be wrong in places (§2.11).
const OFF_PATH: f64 = 0.15;

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
    regions: HashMap<(u16, u16, Option<u16>), u32>,
}

impl GuideIndex {
    pub fn new(points: &[GuidePoint]) -> Self {
        let mut regions = HashMap::new();
        for (i, p) in points.iter().enumerate() {
            let (cc, rr) = (p.cell.0 / 2, p.cell.1 / 2);
            let b = p.coins_left.map(band);
            for c in cc.saturating_sub(1)..=cc + 1 {
                for r in rr.saturating_sub(1)..=rr + 1 {
                    let e = regions.entry((c, r, b)).or_insert(i as u32);
                    *e = (*e).max(i as u32);
                }
            }
        }
        GuideIndex { regions }
    }

    /// The choice weight factor for `key`, `p_max` the furthest index any archived cell has.
    pub fn weight(&self, key: &CellKey, p_max: u32) -> f64 {
        match self.index(key) {
            Some(p) => ON_PATH * (-(p_max.saturating_sub(p) as f64) / FALLOFF).exp(),
            None => OFF_PATH,
        }
    }

    /// The furthest guide point whose region `key` falls in.
    pub fn index(&self, key: &CellKey) -> Option<u32> {
        let banded = self.regions.get(&(key.col, key.row, Some(key.band)));
        let any = self.regions.get(&(key.col, key.row, None));
        banded.max(any).copied()
    }
}
