//! Guide paths (DESIGN §2.11, D21): cells to pass near in order, and the choice weight they give an
//! archived cell. Each point marks a region, its coarse cell (the archive key's col/2, row/2) ±1,
//! in its coins band or a lower one and with its switches pressed when it knows them, in any band
//! or switch set when not; a cell takes the furthest point index whose region it falls in. Lower
//! bands deviate from DESIGN §2.11's "its band" (owner, 2026-10-22, T23): W1-4 took its coins out of
//! the route's order, and a frog ahead on coins fell off the path. Switches (owner,
//! 2026-10-22, T09): a route that goes back over its own ground once a switch opens a gate (W6-2)
//! otherwise credits the ground after the switch to a frog that never pressed it, which then stalls
//! at the closed gate with the switch far behind on the path. The constants are the experiment's
//! (`spike-explore/goexp.rs`, `guide_from`); T14 tunes them.

use std::collections::HashMap;

use super::archive::{CellKey, band};
use super::facts::Cell;

/// The weight on the path is `ON_PATH · e^(−(p_max − p) / FALLOFF)`.
const ON_PATH: f64 = 3.0;
const FALLOFF: f64 = 20.0;
/// Off the path: a guide from another start or mode may be wrong in places (§2.11).
const OFF_PATH: f64 = 0.15;
/// Frames slower than the fastest arrival as far along the guide at which a cell's weight falls
/// by e (`Pace`, T14).
pub const PACE_TAU: f64 = 240.0;

/// The pace factor (owner, 2026-10-22, T14; an addition to DESIGN §2.11's weight): a cell at
/// guide index p reached in `frames` weighs e^(−(frames − best) / PACE_TAU), `best` the fewest
/// frames of any archived cell at index p or further. Measured against the archive, not the
/// guide route's own time: when the search cannot keep the route's pace (W1-4's flying), a
/// weight against the route's time pulled it back to early cells, and W1-4 found no exit in 30
/// minutes where it found one in 1 117 rounds without it.
#[derive(Debug, Clone, Default)]
pub struct Pace {
    /// (index, fewest frames at that index or further), index descending.
    best: Vec<(u32, u32)>,
}

impl Pace {
    /// From each archived cell's guide index and frames.
    pub fn new(cells: impl IntoIterator<Item = (u32, u32)>) -> Pace {
        let mut at: HashMap<u32, u32> = HashMap::new();
        for (p, f) in cells {
            let e = at.entry(p).or_insert(f);
            *e = (*e).min(f);
        }
        let mut best: Vec<(u32, u32)> = at.into_iter().collect();
        best.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        let mut m = u32::MAX;
        for e in best.iter_mut() {
            m = m.min(e.1);
            e.1 = m;
        }
        Pace { best }
    }

    /// The factor for a cell at index `p` reached in `frames`.
    pub fn weight(&self, p: u32, frames: u32) -> f64 {
        // The entries at p or further are a prefix; its last holds their fewest frames.
        let k = self.best.partition_point(|e| e.0 >= p);
        match k.checked_sub(1) {
            Some(k) => (-(frames.saturating_sub(self.best[k].1) as f64) / PACE_TAU).exp(),
            None => 1.0,
        }
    }
}

/// One point of a guide path: a frog cell, and the coins left and the switches pressed there
/// (`CellKey::switches` bits) when known; a point without them matches any band, any switch set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GuidePoint {
    pub cell: Cell,
    pub coins_left: Option<u16>,
    pub switches: Option<u32>,
}

impl GuidePoint {
    /// Whether `key` falls in this point's region, as `GuideIndex` matches one: its coarse cell ±1,
    /// in its band or a lower one when its coins are known, with its switches when known.
    pub fn covers(&self, key: &CellKey) -> bool {
        let (cc, rr) = (self.cell.0 / 2, self.cell.1 / 2);
        key.col.abs_diff(cc) <= 1
            && key.row.abs_diff(rr) <= 1
            && self.coins_left.is_none_or(|c| key.band <= band(c))
            && self.switches.is_none_or(|s| key.switches == s)
    }
}

/// The guide's lookup: (coarse col, coarse row, switches | any) → its points' bands, each with the
/// furthest point index at that band or a higher one (`None` for points without coins, any band).
#[derive(Debug, Clone, Default)]
pub struct GuideIndex {
    regions: HashMap<(u16, u16, Option<u32>), Vec<(Option<u16>, u32)>>,
}

impl GuideIndex {
    pub fn new(points: &[GuidePoint]) -> Self {
        let mut furthest: HashMap<(u16, u16, Option<u32>, Option<u16>), u32> = HashMap::new();
        for (i, p) in points.iter().enumerate() {
            let (cc, rr) = (p.cell.0 / 2, p.cell.1 / 2);
            let b = p.coins_left.map(band);
            for c in cc.saturating_sub(1)..=cc + 1 {
                for r in rr.saturating_sub(1)..=rr + 1 {
                    let e = furthest.entry((c, r, p.switches, b)).or_insert(i as u32);
                    *e = (*e).max(i as u32);
                }
            }
        }
        let mut regions: HashMap<_, Vec<(Option<u16>, u32)>> = HashMap::new();
        for ((c, r, s, b), i) in furthest {
            regions.entry((c, r, s)).or_default().push((b, i));
        }
        // `None` first, then the highest band first, each index raised to the furthest before it.
        for v in regions.values_mut() {
            v.sort_unstable_by(|a, b| {
                b.0.map_or(u32::MAX, u32::from)
                    .cmp(&a.0.map_or(u32::MAX, u32::from))
            });
            let mut m = 0;
            for e in v.iter_mut() {
                m = m.max(e.1);
                e.1 = m;
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

    /// The furthest guide point whose region `key` falls in: a point with its coins known matches
    /// a cell in its band or a lower one (no more coins left than the route had there).
    pub fn index(&self, key: &CellKey) -> Option<u32> {
        // The points at the cell's band or above are a prefix; its last entry holds their furthest.
        let at = |v: &Vec<(Option<u16>, u32)>| {
            let k = v.partition_point(|e| e.0.is_none_or(|b| b >= key.band));
            k.checked_sub(1).map(|k| v[k].1)
        };
        [Some(key.switches), None]
            .into_iter()
            .filter_map(|s| self.regions.get(&(key.col, key.row, s)).and_then(at))
            .max()
    }
}
