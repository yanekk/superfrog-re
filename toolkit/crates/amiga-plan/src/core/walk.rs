//! The walk compass: steps in cells to the exit, or to a collectible, over the open cells, so a
//! wall reads as far where the straight compass points through it. A breadth-first fill,
//! 4-neighbour, ignoring gravity and jumps; one fill per target and switch set, filled lazily and
//! cached. The fills are functions of the start state and the switch set only.

use std::collections::HashMap;

use super::archive::CellKey;
use super::{Cell, Facts, PlanGame};

/// No open path.
pub const SEALED: u16 = u16::MAX;

/// A map's size, `(cols, rows)`.
type Size = (u16, u16);

fn at((cols, _): Size, (c, r): Cell) -> usize {
    r as usize * cols as usize + c as usize
}

fn on((cols, rows): Size, c: Cell) -> bool {
    c.0 < cols && c.1 < rows
}

/// Steps from the nearest of `from` to every cell over `open` (`cols × rows`, row-major),
/// 4-neighbour; `SEALED` where none reaches. A source is a step 0 even if it is not open.
pub fn fill(size: (u16, u16), open: &[bool], from: &[Cell]) -> Vec<u16> {
    let at = |c| at(size, c);
    let on = |c| on(size, c);
    let mut d = vec![SEALED; size.0 as usize * size.1 as usize];
    let mut queue = std::collections::VecDeque::new();
    for &c in from {
        if on(c) && d[at(c)] == SEALED {
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
            if on(n) && open[at(n)] && d[at(n)] == SEALED {
                d[at(n)] = next;
                queue.push_back(n);
            }
        }
    }
    d
}

/// The open cells and their fills.
pub struct WalkCompass {
    size: Size,
    /// Open at the start (`PlanGame::open`).
    open: Vec<bool>,
    /// Per `Facts::gates` index: the cells open once it is pressed.
    gates: Vec<Vec<Cell>>,
    exits: Vec<Cell>,
    /// Collectibles, by `Facts::collectibles` index.
    coins: Vec<(usize, Cell)>,
    to_exit: HashMap<u32, Vec<u16>>,
    to_coin: HashMap<(usize, u32), Vec<u16>>,
}

impl WalkCompass {
    /// From the explorer's start state.
    pub fn new<G: PlanGame>(g: &G, facts: &Facts) -> WalkCompass {
        let (cols, rows) = (facts.cols, facts.rows);
        let open = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (c, r)))
            .map(|c| g.open(c))
            .collect();
        let coins = facts.collectibles.iter().copied().enumerate().collect();
        WalkCompass::from_grid(
            (cols, rows),
            open,
            facts.gates.clone(),
            facts.exits.clone(),
            coins,
        )
    }

    /// From a grid made by hand (tests): `open` is `cols × rows`, row-major.
    pub fn from_grid(
        size: (u16, u16),
        open: Vec<bool>,
        gates: Vec<Vec<Cell>>,
        exits: Vec<Cell>,
        coins: Vec<(usize, Cell)>,
    ) -> WalkCompass {
        assert_eq!(open.len(), size.0 as usize * size.1 as usize);
        WalkCompass {
            size,
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
        self.open[at(self.size, cell)]
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
                    open[at(self.size, c)] = true;
                }
            }
        }
        open
    }

    /// Steps from `key`'s 2×2 cells to the nearest exit: the least over them; `None` if sealed.
    pub fn exit_steps(&mut self, key: &CellKey) -> Option<u16> {
        if !self.to_exit.contains_key(&key.switches) {
            let d = fill(self.size, &self.grid(key.switches), &self.exits);
            self.to_exit.insert(key.switches, d);
        }
        let d = &self.to_exit[&key.switches];
        (0..2)
            .flat_map(|dc| (0..2).map(move |dr| (key.col * 2 + dc, key.row * 2 + dr)))
            .filter(|&c| on(self.size, c))
            .map(|c| d[at(self.size, c)])
            .filter(|&x| x != SEALED)
            .min()
    }

    /// Distance in cells from the player at `player` to the nearest live collectible (`live` holds
    /// a bit per `Facts::collectibles` index), `switches` pressed. Per collectible, the least steps
    /// over the player's cell and its 4 neighbours (mid-jump it may sit in a wall), else its
    /// straight distance. `f64::MAX` with none live, as `archive::coin_dist`.
    pub fn coin_dist(&mut self, player: Cell, live: &[u64], switches: u32) -> f64 {
        let (c, r) = player;
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
                let d = fill(self.size, &self.grid(switches), &[cell]);
                self.to_coin.insert((i, switches), d);
            }
            let d = &self.to_coin[&(i, switches)];
            let steps = near
                .iter()
                .filter(|&&n| on(self.size, n))
                .map(|&n| d[at(self.size, n)])
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

    /// The fills held, and their bytes (they sit outside the archive's count).
    pub fn fills(&self) -> (usize, u64) {
        let n = self.to_exit.len() + self.to_coin.len();
        (n, n as u64 * self.size.0 as u64 * self.size.1 as u64 * 2)
    }
}

/// Which collectibles are live in `g` (`PlanGame::collectible_live`), a bit per
/// `Facts::collectibles` index.
pub fn live_coins<G: PlanGame>(facts: &Facts, g: &G) -> Vec<u64> {
    let mut v = vec![0u64; facts.collectibles.len().div_ceil(64)];
    for (i, &k) in facts.collectibles.iter().enumerate() {
        if g.collectible_live(k) {
            v[i / 64] |= 1 << (i % 64);
        }
    }
    v
}
