//! Coin choice (DESIGN §2.3): before moving, the cheapest set of coins that meets the exit's count
//! and the order to take them, on map distances.
//!
//! Only `CoinKind::Object` coins count toward `coins_needed` (T03 finding: a tile coin adds to
//! `frog+$7e` only), so only they are targets. A coin the map cannot reach costs `far_cost` per leg
//! rather than being ruled out (D16), and coins a leg takes on the way count as taken (D17).
//!
//! Everything is bounded by work counted in steps, never by a clock, and ties go to the lowest
//! index, so the same input gives the same choice to the byte (M8).

use super::facts::{Coin, CoinKind};

/// A place a leg of the route starts or ends at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A map node (the frog's start for the choice).
    Node(u32),
    /// A coin, by its index in `Facts::coins`.
    Coin(usize),
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// `Facts::coins` indices, in the order to take them. Coins a leg takes on the way are not
    /// listed: they are credited, not targeted.
    pub order: Vec<usize>,
    /// Map frames from the start through `order` to the exit, each leg with no map path costing
    /// `far_cost`.
    pub cost: u64,
    /// Counting coins with no map path from the start, chosen or not, ascending: the search must
    /// explore for any of these that `order` holds (D16).
    pub far: Vec<usize>,
}

/// Map frames between two targets, a coin being reached by the cheapest edge that takes it; `None`
/// if the map has no path. The caller (T08) builds it over the MoveMap.
pub type Dist<'a> = &'a dyn Fn(Target, Target) -> Option<u32>;
/// Coins taken on the way along the map path between two targets (Edge::coins), for crediting.
pub type OnTheWay<'a> = &'a dyn Fn(Target, Target) -> Vec<usize>;

/// The cost of a leg with no map path, given the longest map path between the level's targets.
/// It is more than 65 536 such paths, and a route has at most `u16::MAX + 1` legs, so any route
/// with a far leg costs more than any route without one: far coins are chosen only when the
/// reachable ones cannot meet the count (D16).
pub fn far_cost(map_max_path: u32) -> u64 {
    (map_max_path as u64 + 1) << 16
}

/// Work limits, in candidate moves looked at. Sized so a W6 level (190 coins) is about 0.1 s
/// (sf-plan is optimised in test builds); improvement converges well inside its limit there.
const START_WORK: u64 = 2_000_000;
const IMPROVE_WORK: u64 = 20_000_000;

/// Pick the coins to take and their order: cheapest insertion from `from` toward the exit until
/// `needed` counting coins are taken (seeded from several first coins, so a cheap cluster beats
/// scattered near coins), then trimming coins taken on the way, 2-opt and swap-in/swap-out until
/// nothing gains or the work limit is reached. Always returns a choice (D16); if the level has
/// fewer counting coins than `needed`, it takes them all.
pub fn choose(coins: &[Coin], needed: u16, from: u32, dist: Dist, on_the_way: OnTheWay) -> Choice {
    let p = Problem::new(coins, needed, from, dist, on_the_way);
    let far = p
        .ids
        .iter()
        .enumerate()
        .filter(|&(i, _)| !p.finite[p.at(0, i as u32 + 1)])
        .map(|(_, &c)| c)
        .collect();
    let mut best = p.construct();
    p.improve(&mut best);
    Choice {
        order: best.iter().map(|&t| p.ids[t as usize - 1]).collect(),
        cost: p.cost(&best),
        far,
    }
}

/// The choice over local target ids: 0 the start, `1..=m` the counting coins in `Facts::coins`
/// order, `m + 1` the exit. A route is the coin ids between the start and the exit.
struct Problem {
    /// Facts index of local coin `i + 1`.
    ids: Vec<usize>,
    needed: usize,
    /// Targets: `m + 2`. Legs are `leg[from * n + to]`.
    n: usize,
    leg: Vec<u64>,
    finite: Vec<bool>,
    /// Local coin ids each leg takes on the way.
    otw: Vec<Vec<u32>>,
}

impl Problem {
    fn new(coins: &[Coin], needed: u16, from: u32, dist: Dist, on_the_way: OnTheWay) -> Problem {
        let ids: Vec<usize> = (0..coins.len())
            .filter(|&i| matches!(coins[i].kind, CoinKind::Object(_)))
            .collect();
        let mut local = vec![u32::MAX; coins.len()];
        for (i, &c) in ids.iter().enumerate() {
            local[c] = i as u32 + 1;
        }
        let m = ids.len();
        let n = m + 2;
        let target = |t: usize| match t {
            0 => Target::Node(from),
            t if t == m + 1 => Target::Exit,
            t => Target::Coin(ids[t - 1]),
        };
        let mut raw = vec![None; n * n];
        let mut otw = vec![Vec::new(); n * n];
        // Legs leave the start or a coin and reach a coin or the exit; nothing reaches the start.
        for a in 0..=m {
            for b in 1..n {
                if a == b {
                    continue;
                }
                let d = dist(target(a), target(b));
                if d.is_some() {
                    let mut v: Vec<u32> = on_the_way(target(a), target(b))
                        .into_iter()
                        .filter_map(|c| local.get(c).copied().filter(|&l| l != u32::MAX))
                        .collect();
                    v.sort_unstable();
                    v.dedup();
                    otw[a * n + b] = v;
                }
                raw[a * n + b] = d;
            }
        }
        let far = far_cost(raw.iter().flatten().copied().max().unwrap_or(0));
        Problem {
            ids,
            needed: needed as usize,
            n,
            leg: raw.iter().map(|d| d.map_or(far, |d| d as u64)).collect(),
            finite: raw.iter().map(|d| d.is_some()).collect(),
            otw,
        }
    }

    fn at(&self, a: u32, b: u32) -> usize {
        a as usize * self.n + b as usize
    }

    fn l(&self, a: u32, b: u32) -> u64 {
        self.leg[self.at(a, b)]
    }

    /// What putting `c` between `a` and `b` adds (negative where the map's legs are not a metric).
    fn delta(&self, a: u32, c: u32, b: u32) -> i64 {
        (self.l(a, c) + self.l(c, b)) as i64 - self.l(a, b) as i64
    }

    fn exit(&self) -> u32 {
        self.n as u32 - 1
    }

    /// The route's targets including the start and the exit.
    fn full(&self, route: &[u32]) -> Vec<u32> {
        let mut v = Vec::with_capacity(route.len() + 2);
        v.push(0);
        v.extend_from_slice(route);
        v.push(self.exit());
        v
    }

    fn cost(&self, route: &[u32]) -> u64 {
        self.full(route)
            .windows(2)
            .map(|w| self.l(w[0], w[1]))
            .sum()
    }

    /// Distinct counting coins the route takes, visited or on the way.
    fn taken(&self, route: &[u32]) -> usize {
        let mut seen = vec![false; self.n];
        let mut k = 0;
        let full = self.full(route);
        let mut mark = |c: u32| {
            if !seen[c as usize] {
                seen[c as usize] = true;
                k += 1;
            }
        };
        for &c in route {
            mark(c);
        }
        for w in full.windows(2) {
            for &c in &self.otw[self.at(w[0], w[1])] {
                mark(c);
            }
        }
        k
    }

    fn goal(&self) -> usize {
        self.needed.min(self.n - 2)
    }

    fn feasible(&self, route: &[u32]) -> bool {
        self.taken(route) >= self.goal()
    }

    /// Lexicographic (cost, targets): of two equal-cost routes the one with fewer targets wins, so
    /// a coin taken on the way is not also a target.
    fn better(&self, a: &[u32], b: &[u32]) -> bool {
        (self.cost(a), a.len()) < (self.cost(b), b.len())
    }

    /// Cheapest insertion from each of a set of first coins (and from none), the best kept.
    fn construct(&self) -> Vec<u32> {
        let m = self.n as u32 - 2;
        // First coins by how little they add to start→exit, ties by id.
        let mut seeds: Vec<(u64, u32)> = (1..=m)
            .map(|c| (self.l(0, c) + self.l(c, m + 1), c))
            .collect();
        seeds.sort_unstable();
        let per_start = (m as u64 + 1) * (self.goal() as u64 + 1);
        let starts = (START_WORK / per_start).clamp(1, m as u64 + 1) as usize;
        let mut best = self.insert(Vec::new());
        for &(_, s) in seeds.iter().take(starts.saturating_sub(1)) {
            let r = self.insert(vec![s]);
            if self.better(&r, &best) {
                best = r;
            }
        }
        best
    }

    /// Cheapest insertion until the count is met. Keeps, per candidate coin, its best insertion
    /// (added cost, coin it follows), refreshed only where the route changed.
    fn insert(&self, mut route: Vec<u32>) -> Vec<u32> {
        let n = self.n;
        let exit = self.exit();
        // The route as a linked list: next[t] is the target after t.
        let mut next = vec![u32::MAX; n];
        let mut on = vec![false; n];
        // How many times each coin is taken: visits plus legs that pass it.
        let mut cnt = vec![0u32; n];
        let mut taken = 0usize;
        let full = self.full(&route);
        for w in full.windows(2) {
            next[w[0] as usize] = w[1];
        }
        let add = |c: u32, d: i32, cnt: &mut Vec<u32>, taken: &mut usize| {
            let was = cnt[c as usize] > 0;
            cnt[c as usize] = (cnt[c as usize] as i32 + d) as u32;
            match (was, cnt[c as usize] > 0) {
                (false, true) => *taken += 1,
                (true, false) => *taken -= 1,
                _ => {}
            }
        };
        for &c in &route {
            on[c as usize] = true;
            add(c, 1, &mut cnt, &mut taken);
        }
        for w in full.windows(2) {
            for &c in &self.otw[self.at(w[0], w[1])] {
                add(c, 1, &mut cnt, &mut taken);
            }
        }
        let delta = |a: u32, c: u32, b: u32| self.delta(a, c, b);
        // Best (delta, after) per coin; `None` means recompute over the whole route.
        let mut best: Vec<Option<(i64, u32)>> = vec![None; n];
        let goal = self.goal();
        while taken < goal {
            let mut pick: Option<(i64, u32, u32)> = None;
            for c in 1..exit {
                if on[c as usize] || cnt[c as usize] > 0 {
                    continue;
                }
                let b = match best[c as usize] {
                    Some(b) => b,
                    None => {
                        let mut a = 0u32;
                        let mut bb = (i64::MAX, 0u32);
                        while a != exit {
                            let d = delta(a, c, next[a as usize]);
                            if d < bb.0 {
                                bb = (d, a);
                            }
                            a = next[a as usize];
                        }
                        best[c as usize] = Some(bb);
                        bb
                    }
                };
                if pick.is_none_or(|p| b.0 < p.0) {
                    pick = Some((b.0, c, b.1));
                }
            }
            let Some((_, x, a)) = pick else { break };
            let b = next[a as usize];
            for &c in &self.otw[self.at(a, b)] {
                add(c, -1, &mut cnt, &mut taken);
            }
            for &c in self.otw[self.at(a, x)]
                .iter()
                .chain(&self.otw[self.at(x, b)])
            {
                add(c, 1, &mut cnt, &mut taken);
            }
            add(x, 1, &mut cnt, &mut taken);
            on[x as usize] = true;
            next[a as usize] = x;
            next[x as usize] = b;
            // Only insertions that followed `a` lost their edge; the rest compare with the two
            // new ones.
            for c in 1..exit {
                let Some((d, after)) = best[c as usize] else {
                    continue;
                };
                if after == a {
                    best[c as usize] = None;
                    continue;
                }
                let mut nb = (d, after);
                for (s, e) in [(a, x), (x, b)] {
                    let nd = delta(s, c, e);
                    if nd < nb.0 {
                        nb = (nd, s);
                    }
                }
                best[c as usize] = Some(nb);
            }
        }
        route.clear();
        let mut t = next[0];
        while t != exit {
            route.push(t);
            t = next[t as usize];
        }
        route
    }

    /// Trim, 2-opt and swap-in/swap-out, first improvement, until a round gains nothing or the
    /// work limit is spent.
    fn improve(&self, route: &mut Vec<u32>) {
        let mut work = 0u64;
        while work < IMPROVE_WORK {
            if self.trim(route, &mut work)
                || self.two_opt(route, &mut work)
                || self.swap(route, &mut work)
            {
                continue;
            }
            break;
        }
    }

    /// Drop one target if the count still holds and the route is no worse.
    fn trim(&self, route: &mut Vec<u32>, work: &mut u64) -> bool {
        let full = self.full(route);
        for i in 1..full.len() - 1 {
            *work += 1;
            let (a, x, b) = (full[i - 1], full[i], full[i + 1]);
            if self.delta(a, x, b) >= 0 {
                let mut r = route.clone();
                r.remove(i - 1);
                *work += r.len() as u64;
                if self.feasible(&r) {
                    *route = r;
                    return true;
                }
            }
        }
        false
    }

    /// Reverse one stretch of the route, if it is cheaper and the count still holds. Legs are
    /// directed, so a stretch's reversed cost is read from prefix sums both ways.
    fn two_opt(&self, route: &mut Vec<u32>, work: &mut u64) -> bool {
        let full = self.full(route);
        let k = full.len();
        // fwd[t]: legs full[0]→…→full[t]; rev[t]: the same legs walked backwards.
        let mut fwd = vec![0u64; k];
        let mut rev = vec![0u64; k];
        for t in 1..k {
            fwd[t] = fwd[t - 1] + self.l(full[t - 1], full[t]);
            rev[t] = rev[t - 1] + self.l(full[t], full[t - 1]);
        }
        let cost = fwd[k - 1];
        // Reverse full[i..=j], 1 ≤ i < j ≤ k-2.
        for i in 1..k.saturating_sub(2) {
            for j in i + 1..k - 1 {
                *work += 1;
                let new = cost - (fwd[j + 1] - fwd[i - 1])
                    + self.l(full[i - 1], full[j])
                    + (rev[j] - rev[i])
                    + self.l(full[i], full[j + 1]);
                if new < cost {
                    let mut r = route.clone();
                    r[i - 1..j].reverse();
                    *work += r.len() as u64;
                    if self.feasible(&r) {
                        *route = r;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Replace one target by a coin not on the route, or by itself elsewhere (a move), inserted
    /// where it is cheapest, if that is cheaper and the count still holds. Each coin's three
    /// cheapest insertions are found once per call, so each (target, coin) pair is O(1): removing a
    /// target spoils at most two of them.
    fn swap(&self, route: &mut Vec<u32>, work: &mut u64) -> bool {
        let exit = self.exit();
        let full = self.full(route);
        let k = full.len();
        let mut on = vec![false; self.n];
        for &c in route.iter() {
            on[c as usize] = true;
        }
        // Per coin, its three cheapest (delta, after full[p]) insertions, never next to itself.
        let mut top = vec![[(i64::MAX, usize::MAX); 3]; self.n];
        for c in 1..exit {
            *work += k as u64;
            let t = &mut top[c as usize];
            for p in 0..k - 1 {
                if full[p] == c || full[p + 1] == c {
                    continue;
                }
                let d = self.delta(full[p], c, full[p + 1]);
                if d < t[2].0 {
                    t[2] = (d, p);
                    t.sort_unstable();
                }
            }
        }
        for j in 1..k - 1 {
            let (a, x, b) = (full[j - 1], full[j], full[j + 1]);
            let saved = self.delta(a, x, b);
            for c in 1..exit {
                if on[c as usize] {
                    continue;
                }
                *work += 1;
                // Into the edge the removal makes, or the cheapest old one it left intact.
                let mut best = (self.delta(a, c, b), j - 1);
                if let Some(&(d, p)) = top[c as usize]
                    .iter()
                    .find(|&&(_, p)| p != usize::MAX && p != j - 1 && p != j)
                    && d < best.0
                {
                    best = (d, if p < j { p } else { p - 1 });
                }
                if best.0 >= saved {
                    continue;
                }
                let mut r = route.clone();
                r.remove(j - 1);
                r.insert(best.1, c);
                *work += r.len() as u64;
                if self.feasible(&r) {
                    *route = r;
                    return true;
                }
            }
        }
        // A move of a target to elsewhere on the route.
        for j in 1..k - 1 {
            let (a, x, b) = (full[j - 1], full[j], full[j + 1]);
            let saved = self.delta(a, x, b);
            *work += 1;
            let Some(&(d, p)) = top[x as usize].iter().find(|&&(_, p)| p != usize::MAX) else {
                continue;
            };
            if d < saved {
                let mut r = route.clone();
                r.remove(j - 1);
                r.insert(if p < j { p } else { p - 1 }, x);
                *work += r.len() as u64;
                if self.feasible(&r) {
                    *route = r;
                    return true;
                }
            }
        }
        false
    }
}
