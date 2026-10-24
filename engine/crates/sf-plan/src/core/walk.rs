//! The walk compass (D25, T28): steps in cells to the exit, or to a coin, over the open cells, so
//! a wall reads as far where the straight compass points through it. A breadth-first fill,
//! 4-neighbour, ignoring gravity and jumps; one fill per target and switch set, filled lazily and
//! cached. The fills are functions of the start state and the switch set only (M8).

use std::collections::HashMap;

use sf_core::State;

use super::archive::CellKey;
use super::facts::{COLS, Cell, CoinKind, Facts, ROWS, live};
use super::map::in_gate_block;

/// `$3a` and up are ground and walls; 6 the hazard marker (`facts`).
const GROUND: u16 = 0x3a;
const DEADLY: u16 = 6;
/// No open path.
pub const SEALED: u16 = u16::MAX;

fn at((c, r): Cell) -> usize {
    r as usize * COLS as usize + c as usize
}

/// Steps from the nearest of `from` to every cell over `open` (`COLS × ROWS`, row-major),
/// 4-neighbour; `SEALED` where none reaches. A source is a step 0 even if it is not open.
pub fn fill(open: &[bool], from: &[Cell]) -> Vec<u16> {
    let mut d = vec![SEALED; COLS as usize * ROWS as usize];
    let mut queue = std::collections::VecDeque::new();
    for &c in from {
        if c.0 < COLS && c.1 < ROWS && d[at(c)] == SEALED {
            d[at(c)] = 0;
            queue.push_back(c);
        }
    }
    while let Some((c, r)) = queue.pop_front() {
        let next = d[at((c, r))] + 1;
        let around = [
            (c.wrapping_sub(1), r),
            (c + 1, r),
            (c, r.wrapping_sub(1)),
            (c, r + 1),
        ];
        for n in around {
            if n.0 < COLS && n.1 < ROWS && open[at(n)] && d[at(n)] == SEALED {
                d[at(n)] = next;
                queue.push_back(n);
            }
        }
    }
    d
}

/// The open cells and their fills.
pub struct WalkCompass {
    /// Open at the start: marker below `$3a`, not a hazard.
    open: Vec<bool>,
    /// Per `Facts::switches` index: its gate block's cells that are ground at the start, open once
    /// it is pressed.
    gates: Vec<Vec<Cell>>,
    exits: Vec<Cell>,
    /// Object coins, by `Facts::coins` index.
    coins: Vec<(usize, Cell)>,
    to_exit: HashMap<u32, Vec<u16>>,
    to_coin: HashMap<(usize, u32), Vec<u16>>,
}

impl WalkCompass {
    /// From the explorer's start state.
    pub fn new(st: &State, facts: &Facts) -> WalkCompass {
        let cells = || (0..ROWS).flat_map(|r| (0..COLS).map(move |c| (c, r)));
        let open = cells()
            .map(|(c, r)| {
                let m = live(st, c, r);
                m < GROUND && m != DEADLY
            })
            .collect();
        let gates = facts
            .switches
            .iter()
            .map(|s| {
                cells()
                    .filter(|&cell| in_gate_block(s.gate, cell))
                    .filter(|&(c, r)| live(st, c, r) >= GROUND)
                    .collect()
            })
            .collect();
        WalkCompass::from_grid(open, gates, facts.exits.clone(), object_coins(facts))
    }

    /// From a grid made by hand (tests): `open` is `COLS × ROWS`, row-major.
    pub fn from_grid(
        open: Vec<bool>,
        gates: Vec<Vec<Cell>>,
        exits: Vec<Cell>,
        coins: Vec<(usize, Cell)>,
    ) -> WalkCompass {
        assert_eq!(open.len(), COLS as usize * ROWS as usize);
        WalkCompass {
            open,
            gates,
            exits,
            coins,
            to_exit: HashMap::new(),
            to_coin: HashMap::new(),
        }
    }

    /// Whether `cell` is open with `switches` pressed.
    pub fn is_open(&self, cell: Cell, switches: u32) -> bool {
        self.open[at(cell)]
            || self
                .gates
                .iter()
                .enumerate()
                .any(|(k, g)| switches & 1 << k != 0 && g.contains(&cell))
    }

    fn grid(&self, switches: u32) -> Vec<bool> {
        let mut open = self.open.clone();
        for (k, g) in self.gates.iter().enumerate() {
            if switches & 1 << k != 0 {
                for &c in g {
                    open[at(c)] = true;
                }
            }
        }
        open
    }

    /// Steps from `key`'s 2×2 cells to the nearest exit: the least over them; `None` if sealed.
    pub fn exit_steps(&mut self, key: &CellKey) -> Option<u16> {
        if !self.to_exit.contains_key(&key.switches) {
            let d = fill(&self.grid(key.switches), &self.exits);
            self.to_exit.insert(key.switches, d);
        }
        let d = &self.to_exit[&key.switches];
        (0..2)
            .flat_map(|dc| (0..2).map(move |dr| (key.col * 2 + dc, key.row * 2 + dr)))
            .filter(|c| c.0 < COLS && c.1 < ROWS)
            .map(|c| d[at(c)])
            .filter(|&x| x != SEALED)
            .min()
    }

    /// Distance in cells from the frog at `frog` to the nearest live object coin (`live` holds a
    /// bit per `Facts::coins` index), `switches` pressed. Per coin, the least steps over the frog's
    /// cell and its 4 neighbours (mid-jump it may sit in a wall), else its straight distance.
    /// `f64::MAX` with no coin live, as `archive::coin_dist`.
    pub fn coin_dist(&mut self, frog: Cell, live: &[u64], switches: u32) -> f64 {
        let (c, r) = frog;
        let near = [
            (c, r),
            (c.wrapping_sub(1), r),
            (c + 1, r),
            (c, r.wrapping_sub(1)),
            (c, r + 1),
        ];
        let mut best = f64::MAX;
        for j in 0..self.coins.len() {
            let (i, cell) = self.coins[j];
            if live.get(i / 64).is_none_or(|w| w & 1 << (i % 64) == 0) {
                continue;
            }
            if !self.to_coin.contains_key(&(i, switches)) {
                let d = fill(&self.grid(switches), &[cell]);
                self.to_coin.insert((i, switches), d);
            }
            let d = &self.to_coin[&(i, switches)];
            let steps = near
                .iter()
                .filter(|n| n.0 < COLS && n.1 < ROWS)
                .map(|&n| d[at(n)])
                .filter(|&x| x != SEALED)
                .min();
            let x = match steps {
                Some(s) => s as f64,
                None => (cell.0.abs_diff(c) + cell.1.abs_diff(r)) as f64,
            };
            best = best.min(x);
        }
        best
    }

    /// The fills held, and their bytes (they sit outside the archive's count, D22).
    pub fn fills(&self) -> (usize, u64) {
        let n = self.to_exit.len() + self.to_coin.len();
        (n, n as u64 * COLS as u64 * ROWS as u64 * 2)
    }
}

/// `Facts::coins`' object coins, with their indices.
fn object_coins(facts: &Facts) -> Vec<(usize, Cell)> {
    facts
        .coins
        .iter()
        .enumerate()
        .filter(|(_, k)| matches!(k.kind, CoinKind::Object(_)))
        .map(|(i, k)| (i, k.cell))
        .collect()
}

/// Which object coins are live in `st` (their marker 8 or 9 still in the map), a bit per
/// `Facts::coins` index.
pub fn live_coins(facts: &Facts, st: &State) -> Vec<u64> {
    let mut v = vec![0u64; facts.coins.len().div_ceil(64)];
    for (i, k) in facts.coins.iter().enumerate() {
        if matches!(k.kind, CoinKind::Object(_)) && matches!(live(st, k.cell.0, k.cell.1), 8 | 9) {
            v[i / 64] |= 1 << (i % 64);
        }
    }
    v
}
