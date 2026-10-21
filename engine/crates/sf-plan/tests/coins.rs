//! `core::coins::choose` on hand-built worlds: points in a plane, map frames the straight-line
//! distance ×10, some coins with no map path, and coins taken on the way where a test says so.

use std::time::Instant;

use sf_plan::core::*;

struct World {
    start: (i32, i32),
    exit: (i32, i32),
    coins: Vec<Coin>,
    at: Vec<(i32, i32)>,
    /// Coins the map has no path to or from.
    cut: Vec<usize>,
    /// Coins on the corridor `y == 0`: a leg along it takes the ones it passes.
    corridor: bool,
}

fn obj(n: usize) -> Vec<Coin> {
    (0..n)
        .map(|i| Coin {
            cell: (i as u16, 0),
            kind: CoinKind::Object(8),
        })
        .collect()
}

impl World {
    fn new(start: (i32, i32), exit: (i32, i32), at: Vec<(i32, i32)>) -> World {
        World {
            start,
            exit,
            coins: obj(at.len()),
            at,
            cut: Vec::new(),
            corridor: false,
        }
    }

    fn pos(&self, t: Target) -> (i32, i32) {
        match t {
            Target::Node(_) => self.start,
            Target::Coin(c) => self.at[c],
            Target::Exit => self.exit,
        }
    }

    fn dist(&self, a: Target, b: Target) -> Option<u32> {
        if [a, b]
            .iter()
            .any(|t| matches!(t, Target::Coin(c) if self.cut.contains(c)))
        {
            return None;
        }
        let (p, q) = (self.pos(a), self.pos(b));
        let (dx, dy) = ((p.0 - q.0) as f64, (p.1 - q.1) as f64);
        Some(((dx * dx + dy * dy).sqrt() * 10.0).round() as u32)
    }

    fn on_the_way(&self, a: Target, b: Target) -> Vec<usize> {
        let (p, q) = (self.pos(a), self.pos(b));
        if !self.corridor || p.1 != 0 || q.1 != 0 {
            return Vec::new();
        }
        let (lo, hi) = (p.0.min(q.0), p.0.max(q.0));
        (0..self.at.len())
            .filter(|&c| self.at[c].1 == 0 && lo < self.at[c].0 && self.at[c].0 < hi)
            .collect()
    }

    fn choose(&self, needed: u16) -> Choice {
        choose(&self.coins, needed, 0, &|a, b| self.dist(a, b), &|a, b| {
            self.on_the_way(a, b)
        })
    }

    /// The route's cost as `choose` counts it, from `self.dist` (no far legs).
    fn cost(&self, order: &[usize]) -> u64 {
        let mut t = vec![Target::Node(0)];
        t.extend(order.iter().map(|&c| Target::Coin(c)));
        t.push(Target::Exit);
        t.windows(2)
            .map(|w| self.dist(w[0], w[1]).unwrap() as u64)
            .sum()
    }
}

fn sorted(v: &[usize]) -> Vec<usize> {
    let mut v = v.to_vec();
    v.sort_unstable();
    v
}

#[test]
fn a_line_of_coins_on_the_way_beats_a_far_cluster() {
    // Coins 0..=8 on the straight line to the exit, a cluster of 5 high above it.
    let mut at: Vec<(i32, i32)> = (1..=9).map(|i| (10 * i, 0)).collect();
    at.extend((0..5).map(|i| (50 + i, 60)));
    let w = World::new((0, 0), (100, 0), at);
    let ch = w.choose(3);
    assert_eq!(ch.order.len(), 3, "{ch:?}");
    assert!(ch.order.iter().all(|&c| c < 9), "{ch:?}");
    assert_eq!(ch.cost, 1000);
    assert_eq!(ch.cost, w.cost(&ch.order));
    // Taken along the way, in order.
    assert!(
        ch.order.windows(2).all(|p| w.at[p[0]].0 < w.at[p[1]].0),
        "{ch:?}"
    );
    assert!(ch.far.is_empty());
}

#[test]
fn a_cheap_cluster_off_to_one_side_beats_scattered_near_coins() {
    // Scattered coins either side of the line, each nearer than the cluster, but three of them
    // cost more than the whole cluster.
    let at = vec![(20, -20), (50, 20), (80, -20), (50, 45), (52, 45), (54, 45)];
    let w = World::new((0, 0), (100, 0), at);
    // Brute force: every ordered choice of three.
    let mut best = u64::MAX;
    for a in 0..6 {
        for b in 0..6 {
            for c in 0..6 {
                if a != b && b != c && a != c {
                    best = best.min(w.cost(&[a, b, c]));
                }
            }
        }
    }
    // The nearest single coin is a scattered one, so nearest-first would not find the cluster.
    let single = |c: usize| w.cost(&[c]);
    assert!((0..3).all(|s| (3..6).all(|k| single(s) < single(k))));
    let ch = w.choose(3);
    assert_eq!(sorted(&ch.order), vec![3, 4, 5], "{ch:?}");
    assert_eq!(ch.cost, best);
}

#[test]
fn far_coins_only_when_the_reachable_ones_cannot_meet_the_count() {
    let at = vec![(20, 5), (40, -5), (60, 5), (50, 30), (70, -30)];
    let mut w = World::new((0, 0), (100, 0), at);
    w.cut = vec![3, 4];

    let ch = w.choose(2);
    assert!(ch.order.iter().all(|&c| c < 3), "{ch:?}");
    assert_eq!(ch.far, vec![3, 4]);
    assert_eq!(ch.cost, w.cost(&ch.order));

    let ch = w.choose(3);
    assert_eq!(sorted(&ch.order), vec![0, 1, 2]);
    assert_eq!(ch.far, vec![3, 4]);

    // Four: all three reachable plus one far coin, whose two legs cost `far_cost` each.
    let ch = w.choose(4);
    assert_eq!(ch.order.len(), 4, "{ch:?}");
    assert_eq!(ch.order.iter().filter(|&&c| c >= 3).count(), 1, "{ch:?}");
    assert!(ch.cost >= 2 * far_cost(0), "{ch:?}");
    assert_eq!(ch.far, vec![3, 4]);

    // More than the level has: everything, never a refusal (D16).
    let ch = w.choose(9);
    assert_eq!(sorted(&ch.order), vec![0, 1, 2, 3, 4]);
}

#[test]
fn far_cost_outweighs_any_route_of_map_paths() {
    assert!(far_cost(1000) > 1000 * (u16::MAX as u64 + 1));
    assert!(far_cost(0) > 0);
    // A route through every reachable coin of a long level still costs less than one far leg.
    let at: Vec<(i32, i32)> = (0..40)
        .map(|i| (if i % 2 == 0 { 0 } else { 300 }, i))
        .collect();
    let mut w = World::new((0, 0), (300, 84), at);
    w.at.push((150, 42));
    w.coins = obj(41);
    w.cut = vec![40];
    let ch = w.choose(40);
    assert!(!ch.order.contains(&40), "{ch:?}");
    assert!(ch.cost < far_cost(0), "{ch:?}");
}

#[test]
fn coins_taken_on_the_way_count() {
    // A dead-end corridor: the start, coins at 10, 20, 30 along it, and the exit up a shaft from
    // the start. Going to the end takes the other two on the way.
    let mut w = World::new((0, 0), (0, 50), vec![(10, 0), (20, 0), (30, 0)]);
    w.corridor = true;
    let ch = w.choose(3);
    assert_eq!(ch.order, vec![2], "{ch:?}");
    assert_eq!(
        ch.cost,
        300 + w.dist(Target::Coin(2), Target::Exit).unwrap() as u64
    );
}

#[test]
fn tile_coins_are_never_targets() {
    // Tile coins add to frog+$7e only, never to the exit's count (T03).
    let mut w = World::new((0, 0), (100, 0), vec![(10, 0), (20, 0), (50, 40), (60, 40)]);
    w.coins[0].kind = CoinKind::Tile;
    w.coins[1].kind = CoinKind::Tile;
    let ch = w.choose(2);
    assert_eq!(sorted(&ch.order), vec![2, 3], "{ch:?}");
    // A tile coin with no map path is not listed as far either.
    w.cut = vec![0];
    assert!(w.choose(2).far.is_empty());
}

/// A W6-sized level: 190 coins over 300×84 cells from a fixed generator, 1 in 19 unreachable,
/// coins on the row-0 corridor credited on the way.
fn big() -> World {
    let mut s = 0x2545_f491u32;
    let mut rnd = |n: u32| {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s % n) as i32
    };
    let at: Vec<(i32, i32)> = (0..190)
        .map(|i| (rnd(300), if i % 7 == 0 { 0 } else { rnd(84) }))
        .collect();
    let mut w = World::new((5, 0), (295, 80), at);
    w.cut = (0..190).filter(|i| i % 19 == 3).collect();
    w.corridor = true;
    w
}

#[test]
fn same_input_same_choice() {
    // Ties everywhere: a symmetric ring of coins about the start-exit line.
    let at: Vec<(i32, i32)> = (0..12)
        .map(|i| (10 * (i / 2 + 1), if i % 2 == 0 { 10 } else { -10 }))
        .collect();
    let w = World::new((0, 0), (70, 0), at);
    for needed in [1, 3, 6, 12] {
        assert_eq!(w.choose(needed), w.choose(needed), "needed {needed}");
    }
    let w = big();
    assert_eq!(w.choose(120), w.choose(120));
}

#[test]
fn a_w6_sized_level_is_chosen_within_a_second() {
    let w = big();
    for needed in [60, 150, 181] {
        let t = Instant::now();
        let ch = w.choose(needed);
        let took = t.elapsed();
        assert!(took.as_secs_f64() < 1.0, "needed {needed}: {took:?}");
        assert!(
            ch.order.iter().all(|c| !w.cut.contains(c)),
            "needed {needed}"
        );
        assert_eq!(ch.far, w.cut);
        // The count is met: targets plus the corridor coins legs pass.
        let mut taken: Vec<usize> = ch.order.clone();
        let mut t = vec![Target::Node(0)];
        t.extend(ch.order.iter().map(|&c| Target::Coin(c)));
        t.push(Target::Exit);
        for p in t.windows(2) {
            taken.extend(w.on_the_way(p[0], p[1]));
        }
        assert!(sorted(&taken).windows(2).filter(|p| p[0] != p[1]).count() + 1 >= needed as usize);
    }
}
