//! T00 smart-planner spike: throwaway, deleted at the task's end.
//! `cargo run --release -p sf-conform --example zz_map_spike -- <cmd> <level> [opts]`
//! cmds: probe, spin, all (Q1, Q2, Q4, Q5, Q7), bands (Q6). `--nodraw` sets the local-only
//! sf-core hack that skips the level picture in `Scene::step` (never committed).
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{h0, h1};
use sf_core::{Game, LevelId, Start};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

const LEVEL_STATE: u32 = h0(0xa06);
const HEARTS: u32 = h0(0x830);
const ROW0: u32 = 0x218e0 + 0x4c0;
const OBJ_SLOTS: u32 = h0(0x1ba6c);
static WAITS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SIM: AtomicU64 = AtomicU64::new(0);

fn step(g: &mut Game, x: FrameInput) {
    g.step_unseen(x);
    SIM.fetch_add(1, Ordering::Relaxed);
}

fn marker(st: &sf_core::State, c: i32, r: i32) -> u16 {
    if !(0..300).contains(&c) || !(0..84).contains(&r) {
        return 0x3e;
    }
    st.w(h1(ROW0) + r as u32 * 0x260 + 2 * c as u32) & 0x3f
}

fn cell(st: &sf_core::State) -> (i32, i32) {
    let w = |o| st.w(FROG + o) as i16 as i32;
    ((w(f::X) + 0x17).max(0) / 16, (w(f::Y) - 0x10).max(0) / 16)
}

fn alive(st: &sf_core::State, lives: u16) -> bool {
    (st.w(FROG + f::LIVES) as i16) >= lives as i16 && st.w(FROG + f::DYING) == 0
}

fn in_level(st: &sf_core::State) -> bool {
    st.l(LEVEL_STATE) == 0xffff_fffe
}

fn landed(st: &sf_core::State) -> bool {
    (st.w(FROG + f::ON_GROUND) != 0 || st.w(FROG + f::ON_PLATFORM) != 0) && st.w(FROG + f::AIR) == 0
}

/// Speed band: 0 stopped, 1 under half the limit, 2 at or over half.
fn band(st: &sf_core::State) -> u8 {
    let v = (st.w(FROG + f::VX) as i16).unsigned_abs();
    let max = st.w(FROG + f::MAX_SPEED).max(1);
    if v == 0 {
        0
    } else if 2 * v < max {
        1
    } else {
        2
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct Node {
    c: i16,
    r: i16,
    band: u8,
    facing: u8,
    wings: u8,
}

impl Node {
    fn cell(self) -> (i16, i16) {
        (self.c, self.r)
    }
}

fn has_wings(st: &sf_core::State) -> u8 {
    (st.w(FROG + f::FLIGHT) != 0) as u8
}

fn node(st: &sf_core::State, bands: bool) -> Node {
    let (c, r) = cell(st);
    Node {
        c: c as i16,
        r: r as i16,
        band: if bands { band(st) } else { 0 },
        facing: if bands { (st.w(FROG + f::FACING) != 0) as u8 } else { 0 },
        wings: has_wings(st),
    }
}

/// A macro move: held joystick states with their frame counts; afterwards `tail` is held until
/// the frog lands.
#[derive(Clone, Debug)]
struct Move {
    name: String,
    holds: Vec<(Joy, u16)>,
    tail: Joy,
    wings_only: bool,
}

fn moves() -> Vec<Move> {
    let mut v = Vec::new();
    let mk = |name: String, holds: Vec<(Joy, u16)>, tail: Joy, w: bool| Move {
        name,
        holds,
        tail,
        wings_only: w,
    };
    for (d, dn) in [(Joy::RIGHT, "R"), (Joy::LEFT, "L")] {
        for n in [6u16, 12, 24] {
            v.push(mk(format!("walk{dn}{n}"), vec![(d, n)], Joy::NONE, false));
        }
        for h in [4u16, 10, 20] {
            v.push(mk(format!("jump{dn}{h}"), vec![(d | Joy::UP, h)], d, false));
        }
        v.push(mk(format!("hop{dn}"), vec![(d | Joy::UP, 20)], Joy::NONE, false));
        for run in [8u16, 16, 32] {
            v.push(mk(format!("run{dn}{run}"), vec![(d, run), (d | Joy::UP, 20)], d, false));
        }
        // Flight: a jump, then fire pressed again and again (each press a flap).
        for k in [4usize, 8, 16] {
            for (dd, tag) in [(d, ""), (Joy::NONE, "U")] {
                let mut h = vec![(d | Joy::UP, 6)];
                for _ in 0..k {
                    h.push((dd | Joy::FIRE, 4));
                    h.push((dd, 4));
                }
                v.push(mk(format!("fly{tag}{dn}{k}"), h, Joy::NONE, true));
            }
        }
    }
    for h in [4u16, 20] {
        v.push(mk(format!("jumpU{h}"), vec![(Joy::UP, h)], Joy::NONE, false));
    }
    v.push(mk("down".into(), vec![(Joy::DOWN, 8)], Joy::NONE, false));
    if WAITS.load(Ordering::Relaxed) {
        // Wait on the spot, then jump: a moving block's timing (D14's 12/25/50).
        for w in [25u16, 50, 100, 200] {
            v.push(mk(format!("ride{w}"), vec![(Joy::NONE, w)], Joy::NONE, false));
        }
        for w in [12u16, 25, 50, 100, 200] {
            for (d, dn) in [(Joy::RIGHT, "R"), (Joy::LEFT, "L"), (Joy::NONE, "U")] {
                v.push(mk(format!("wait{w}jump{dn}"), vec![(Joy::NONE, w), (d | Joy::UP, 20)], d, false));
                if d != Joy::NONE {
                    v.push(mk(format!("wait{w}walk{dn}"), vec![(Joy::NONE, w), (d, 24)], Joy::NONE, false));
                }
            }
        }
    }
    v.push(mk("stop".into(), vec![(Joy::NONE, 1)], Joy::NONE, false));
    v
}

const LAND_CAP: u32 = 150;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fail {
    Died,
    NoLand,
}

struct Played {
    frames: u32,
    game: Game,
    coins: Vec<(i16, i16)>,
    hearts_lost: bool,
}

fn inputs_of(m: &Move) -> Vec<FrameInput> {
    let mut v = Vec::new();
    for &(j, n) in &m.holds {
        for _ in 0..n {
            v.push(FrameInput::joy(j));
        }
    }
    v
}

/// Plays `m` from `g`: dies, leaves the level, or never lands = `Err`.
fn play(g: &Game, m: &Move, stop_after: bool, coin_cells: &[(i16, i16)]) -> Result<Played, Fail> {
    let mut g = g.clone();
    let lives = g.state().w(FROG + f::LIVES);
    let hearts = g.state().w(HEARTS);
    let had: Vec<(i16, i16)> = coin_cells
        .iter()
        .copied()
        .filter(|&(c, r)| matches!(marker(g.state(), c as i32, r as i32), 8 | 9))
        .collect();
    let mut frames = 0;
    let ok = |g: &Game| alive(g.state(), lives) && in_level(g.state());
    for x in inputs_of(m) {
        step(&mut g, x);
        frames += 1;
        if !ok(&g) {
            return Err(Fail::Died);
        }
    }
    let mut k = 0;
    while !landed(g.state()) {
        step(&mut g, FrameInput::joy(m.tail));
        frames += 1;
        k += 1;
        if !ok(&g) {
            return Err(Fail::Died);
        }
        if k > LAND_CAP {
            return Err(Fail::NoLand);
        }
    }
    if stop_after {
        // Cells-only map: the node's state is the frog stopped.
        let mut k = 0;
        while (g.state().w(FROG + f::VX) != 0 || !landed(g.state())) && k < 40 {
            step(&mut g, FrameInput::default());
            frames += 1;
            k += 1;
            if !ok(&g) {
                return Err(Fail::Died);
            }
        }
    }
    let st = g.state();
    let coins = had
        .into_iter()
        .filter(|&(c, r)| !matches!(marker(st, c as i32, r as i32), 8 | 9))
        .collect();
    Ok(Played {
        frames,
        coins,
        hearts_lost: st.w(HEARTS) < hearts,
        game: g,
    })
}

fn par_map<T: Sync, R: Send>(items: &[T], threads: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let mut out: Vec<(usize, R)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut v = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= items.len() {
                            break;
                        }
                        v.push((i, f(&items[i])));
                    }
                    v
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    out.sort_by_key(|x| x.0);
    out.into_iter().map(|x| x.1).collect()
}

#[derive(Clone, Debug)]
struct Edge {
    from: Node,
    mv: usize,
    to: Node,
    frames: u32,
    coins: Vec<(i16, i16)>,
}

#[derive(Clone, Copy, Debug)]
struct SlotInfo {
    passive: u16,
    stomp: u16,
    hp: u16,
    touch: u32,
    update: u32,
}

#[derive(Default)]
struct Map {
    nodes: BTreeMap<Node, Option<Game>>,
    parent: BTreeMap<Node, (Node, usize)>,
    edges: Vec<Edge>,
    secs: f64,
    sim: u64,
    hearts_lost: usize,
    died: usize,
    noland: usize,
    slots: BTreeMap<u32, SlotInfo>,
}

fn slots_of(st: &sf_core::State) -> Vec<(u32, SlotInfo)> {
    (0..20)
        .map(|k| OBJ_SLOTS + k * 0x7c)
        .filter(|&s| st.l(s + 0x40) != 0)
        .map(|s| {
            (
                st.l(s + 0x40),
                SlotInfo {
                    passive: st.w(s + 0x30),
                    stomp: st.w(s + 0x2e),
                    hp: st.w(s + 0x1c),
                    touch: st.l(s + 0x10).wrapping_sub(h0(0)),
                    update: st.l(s + 0x04).wrapping_sub(h0(0)),
                },
            )
        })
        .collect()
}

/// Breadth-first over nodes reached by real moves from `start`.
fn build_bfs(start: &Game, bands: bool, threads: usize, keep: bool, coin_cells: &[(i16, i16)]) -> Map {
    let t0 = Instant::now();
    let sim0 = SIM.load(Ordering::Relaxed);
    let ms = moves();
    let mut map = Map::default();
    let n0 = node(start.state(), bands);
    map.nodes.insert(n0, Some(start.clone()));
    let mut frontier = vec![n0];
    let mut layer = 0;
    while !frontier.is_empty() {
        let jobs: Vec<(Node, usize)> = frontier
            .iter()
            .flat_map(|&n| {
                let w = n.wings != 0;
                ms.iter()
                    .enumerate()
                    .filter(move |(_, m)| w || !m.wings_only)
                    .map(move |(i, _)| (n, i))
            })
            .collect();
        let nodes = &map.nodes;
        let ms_ref = &ms;
        let results = par_map(&jobs, threads, |&(n, i)| {
            let g = nodes[&n].as_ref().unwrap();
            play(g, &ms_ref[i], !bands, coin_cells).map(|p| {
                let to = node(p.game.state(), bands);
                let slots = slots_of(p.game.state());
                (to, p, slots)
            })
        });
        let mut next = Vec::new();
        for (&(n, i), r) in jobs.iter().zip(results) {
            let (to, p, slots) = match r {
                Ok(x) => x,
                Err(Fail::Died) => {
                    map.died += 1;
                    continue;
                }
                Err(Fail::NoLand) => {
                    map.noland += 1;
                    continue;
                }
            };
            for (a, s) in slots {
                map.slots.entry(a).or_insert(s);
            }
            if p.hearts_lost {
                map.hearts_lost += 1;
                continue;
            }
            if to == n {
                continue;
            }
            map.edges.push(Edge {
                from: n,
                mv: i,
                to,
                frames: p.frames,
                coins: p.coins,
            });
            if !map.nodes.contains_key(&to) {
                map.nodes.insert(to, Some(p.game));
                map.parent.insert(to, (n, i));
                next.push(to);
            }
        }
        if !keep {
            for n in &frontier {
                if *n != n0 {
                    map.nodes.insert(*n, None);
                }
            }
        }
        layer += 1;
        if layer % 10 == 0 {
            eprintln!(
                "  bfs layer {layer}: {} nodes, {} edges, frontier {}, {:.0}s",
                map.nodes.len(),
                map.edges.len(),
                next.len(),
                t0.elapsed().as_secs_f64()
            );
        }
        frontier = next;
    }
    map.secs = t0.elapsed().as_secs_f64();
    map.sim = SIM.load(Ordering::Relaxed) - sim0;
    map
}

fn standable(st: &sf_core::State, c: i32, r: i32) -> bool {
    let m = marker(st, c, r);
    let b = marker(st, c, r + 1);
    m < 0x3a && m != 6 && (b >= 0x3a || b == 44)
}

/// The frog placed by writing its position into `start`: settled, or `None`.
fn poke_at(start: &Game, c: i32, r: i32) -> Option<Game> {
    let mut g = start.clone();
    let x = (c * 16 - 0x17 + 8) as u16;
    let y = (r * 16 + 0x10 + 8) as u16;
    for (o, v) in [(f::X, x), (f::Y, y), (f::VX, 0), (f::VY, 0), (f::AIR, 0), (f::PREV_X, x), (f::PREV_Y, y)] {
        g.poke(FROG + o, &v.to_be_bytes());
    }
    g.poke(FROG + f::ON_GROUND, &1u16.to_be_bytes());
    let lives = g.state().w(FROG + f::LIVES);
    for _ in 0..30 {
        step(&mut g, FrameInput::default());
        if !alive(g.state(), lives) || !in_level(g.state()) {
            return None;
        }
    }
    (landed(g.state()) && cell(g.state()) == (c, r)).then_some(g)
}

/// The poke-seeded map: every standable cell, every move (no wings).
fn build_poke(start: &Game, threads: usize, coin_cells: &[(i16, i16)]) -> (Map, usize) {
    let t0 = Instant::now();
    let sim0 = SIM.load(Ordering::Relaxed);
    let ms = moves();
    let st = start.state();
    let cells: Vec<(i32, i32)> = (0..84)
        .flat_map(|r| (0..300).map(move |c| (c, r)))
        .filter(|&(c, r)| standable(st, c, r))
        .collect();
    let ms_ref = &ms;
    let out = par_map(&cells, threads, |&(c, r)| {
        let Some(g) = poke_at(start, c, r) else {
            return None;
        };
        let n = node(g.state(), false);
        let mut es = vec![];
        for (i, m) in ms_ref.iter().enumerate() {
            if m.wings_only && n.wings == 0 {
                continue;
            }
            if let Ok(p) = play(&g, m, true, coin_cells) {
                let to = node(p.game.state(), false);
                if to != n && !p.hearts_lost {
                    es.push(Edge {
                        from: n,
                        mv: i,
                        to,
                        frames: p.frames,
                        coins: p.coins,
                    });
                }
            }
        }
        Some((n, es))
    });
    let mut map = Map::default();
    for (n, es) in out.into_iter().flatten() {
        map.nodes.insert(n, None);
        map.edges.extend(es);
    }
    map.secs = t0.elapsed().as_secs_f64();
    map.sim = SIM.load(Ordering::Relaxed) - sim0;
    (map, cells.len())
}

/// The level entered on the engine and played to where the frog stands; `clear`: markers
/// removed from the map first (enemies frozen: they never spawn).
fn level_start(id: LevelId, clear: &[u16]) -> Game {
    let display = sf_conform::display_data();
    let mut g = if !clear.is_empty() {
        let mut lv = sf_conform::game_data().levels[&id].clone();
        for c in lv.t7mp.body.iter_mut() {
            if clear.contains(&(*c >> 10)) {
                *c &= 0x3ff;
            }
        }
        Game::with_override(display, lv)
    } else {
        Game::with_display(display, Start::Level(id))
    };
    let mut k = 0;
    while k < 400 && !(landed(g.state()) && g.frame() > 150) {
        step(&mut g, FrameInput::default());
        k += 1;
    }
    g
}

fn world_of(id: LevelId) -> u32 {
    let s = id.to_string();
    match s.as_bytes()[1] {
        b'B' => 6,
        b'W' => 5,
        c => (c - b'1') as u32,
    }
}

fn spawning_markers(id: LevelId) -> Vec<u16> {
    let display = sf_conform::display_data();
    let rom = &display.rom().h0;
    let w = |o: u32| u16::from_be_bytes([rom[o as usize], rom[o as usize + 1]]);
    let table = 0x178ec + 0x100 * world_of(id);
    (1..64u16)
        .filter(|&m| {
            let at = table + 4 * m as u32;
            at + 2 + w(at + 2) as i16 as i32 as u32 != 0x17fec
        })
        .collect()
}

/// The original map marker of a slot's origin cell, if it is a map cell.
fn origin(id: LevelId, a: u32) -> Option<(i32, i32, u16)> {
    let d = a.wrapping_sub(h1(ROW0));
    if d >= 84 * 0x260 || d % 2 != 0 || (d % 0x260) / 2 >= 300 {
        return None;
    }
    let (r, c) = ((d / 0x260) as i32, ((d % 0x260) / 2) as i32);
    let t = &sf_conform::game_data().levels[&id].t7mp;
    Some((c, r, t.cell(c as u16, r as u16) >> 10))
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Q7: a random walk along `map`'s edges from the live start, enemies live, `n` edges played.
fn live_walk(start: &Game, map: &Map, n: usize, seed: u64, coin_cells: &[(i16, i16)]) -> String {
    let ms = moves();
    let mut out_of: BTreeMap<Node, Vec<&Edge>> = BTreeMap::new();
    for e in &map.edges {
        out_of.entry(e.from).or_default().push(e);
    }
    let mut rng = Rng(seed);
    let (mut g, mut cur) = (start.clone(), node(start.state(), false));
    let (mut ok, mut elsewhere, mut hurt, mut died, mut noland, mut restarts, mut coin_edges) =
        (0, 0, 0, 0, 0, 0, 0);
    let mut nodes_seen = BTreeSet::new();
    let mut hist = [0usize; 5];
    for _ in 0..n {
        let Some(es) = out_of.get(&cur) else {
            restarts += 1;
            g = start.clone();
            cur = node(g.state(), false);
            continue;
        };
        let e = es[rng.below(es.len())];
        nodes_seen.insert(cur);
        match play(&g, &ms[e.mv], true, coin_cells) {
            Err(Fail::Died) => {
                died += 1;
                g = start.clone();
                cur = node(g.state(), false);
                restarts += 1;
            }
            Err(Fail::NoLand) => {
                noland += 1;
                g = start.clone();
                cur = node(g.state(), false);
                restarts += 1;
            }
            Ok(p) => {
                let to = node(p.game.state(), false);
                if !e.coins.is_empty() {
                    coin_edges += 1;
                }
                if p.hearts_lost {
                    hurt += 1;
                } else if to != e.to {
                    elsewhere += 1;
                    hist[miss(to, e.to)] += 1;
                } else {
                    ok += 1;
                }
                // Hearts run out: start again rather than die.
                if p.game.state().w(HEARTS) <= 1 {
                    g = start.clone();
                    cur = node(g.state(), false);
                    restarts += 1;
                } else {
                    g = p.game;
                    cur = to;
                }
            }
        }
    }
    let fail = elsewhere + hurt + died + noland;
    format!(
        "played {n}: ok {ok}, landed elsewhere {elsewhere}, lost a heart {hurt}, died {died}, \
         never landed {noland}; failure {:.1}%; restarts {restarts}; distinct start nodes {}; \
         edges taking a coin {coin_edges}; {}\n",
        100.0 * fail as f64 / n as f64,
        nodes_seen.len(),
        miss_line(&hist)
    )
}

/// How far a landing missed: 0 same node, 1 adjacent cell, 2 two or three cells, 3 further, 4
/// same cell other node fields (wings, band).
fn miss(a: Node, b: Node) -> usize {
    let d = (a.c - b.c).abs().max((a.r - b.r).abs());
    match d {
        0 if a == b => 0,
        0 => 4,
        1 => 1,
        2 | 3 => 2,
        _ => 3,
    }
}

fn miss_line(h: &[usize; 5]) -> String {
    format!("misses by distance: adjacent cell {}, 2-3 cells {}, further {}, same cell other fields {}", h[1], h[2], h[3], h[4])
}

fn coin_cells_of(st: &sf_core::State) -> Vec<(i16, i16)> {
    (0..84)
        .flat_map(|r| (0..300).map(move |c| (c, r)))
        .filter(|&(c, r)| matches!(marker(st, c, r), 8 | 9))
        .map(|(c, r)| (c as i16, r as i16))
        .collect()
}

/// The stall coins: W1-4's sky and cave coins (rust-engine T154), W6-3's coins above open air
/// (four open cells under them).
fn stall_coins(id: LevelId, st: &sf_core::State, coins: &[(i16, i16)]) -> Vec<((i16, i16), &'static str)> {
    let s = id.to_string();
    coins
        .iter()
        .filter_map(|&(c, r)| {
            let inr = |c0, c1, r0, r1| (c0..=c1).contains(&c) && (r0..=r1).contains(&r);
            if s == "W1-4" {
                if inr(6, 29, 6, 17) || inr(59, 62, 12, 17) || inr(136, 138, 13, 19) || inr(153, 160, 1, 2) {
                    return Some(((c, r), "sky"));
                }
                if inr(124, 131, 33, 33) {
                    return Some(((c, r), "cave"));
                }
                None
            } else {
                let open = (1..=4).all(|k| {
                    let m = marker(st, c as i32, r as i32 + k);
                    m < 0x3a && m != 44
                });
                open.then_some(((c, r), "above-air"))
            }
        })
        .collect()
}

fn path_to(map: &Map, n: Node) -> Vec<usize> {
    let mut v = vec![];
    let mut cur = n;
    while let Some(&(p, mv)) = map.parent.get(&cur) {
        v.push(mv);
        cur = p;
    }
    v.reverse();
    v
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args[0].as_str();
    let id: LevelId = args[1].parse().unwrap();
    let opt = |k: &str| args.iter().position(|a| a == k).map(|i| args[i + 1].clone());
    let flag = |k: &str| args.iter().any(|a| a == k);
    if flag("--nodraw") {
        sf_core::scene::SPIKE_NO_DRAW.store(true, Ordering::Relaxed);
    }
    if flag("--waits") {
        WAITS.store(true, Ordering::Relaxed);
    }
    let threads: usize = opt("--threads").map_or(10, |v| v.parse().unwrap());
    let out_dir = opt("--out").unwrap_or("/tmp".into());
    let write = |name: &str, s: &str| {
        let p = format!("{out_dir}/{id}-{name}.txt");
        std::fs::write(&p, s).unwrap();
        print!("== {p}\n{s}");
    };
    match cmd {
        "probe" => {
            let g = level_start(id, &[]);
            let st = g.state();
            println!("frame {} cell {:?} coins {} hearts {} lives {}", g.frame(), cell(st),
                st.w(FROG + f::COINS_LEFT), st.w(HEARTS), st.w(FROG + f::LIVES));
            println!("spawning markers {:?}", spawning_markers(id));
        }
        "spin" => {
            let g = level_start(id, &[]);
            let secs: f64 = opt("--secs").map_or(20.0, |v| v.parse().unwrap());
            let t = Instant::now();
            let mut n = 0u64;
            while t.elapsed().as_secs_f64() < secs {
                let mut g2 = g.clone();
                for i in 0..1000 {
                    let j = match (i / 50) % 4 { 0 => Joy::RIGHT, 1 => Joy::RIGHT | Joy::UP, 2 => Joy::LEFT, _ => Joy::NONE };
                    g2.step_unseen(FrameInput::joy(j));
                    n += 1;
                }
            }
            println!("spin {n} frames {:.0} f/s", n as f64 / t.elapsed().as_secs_f64());
        }
        "bfs" => {
            let start = level_start(id, &[]);
            let cc = coin_cells_of(start.state());
            let m = build_bfs(&start, flag("--bands"), threads, false, &cc);
            write(if flag("--nodraw") { "q3-build-nodraw" } else { "q3-build-drawn" }, &map_line(if flag("--bands") { "bands" } else { "cells only" }, &m));
        }
        "pokereach" => {
            let start = level_start(id, &[]);
            let st = start.state().clone();
            let cc = coin_cells_of(&st);
            let (pm, _) = build_poke(&start, threads, &cc);
            let mut adj: BTreeMap<(i16, i16), Vec<(i16, i16)>> = BTreeMap::new();
            for e in &pm.edges {
                adj.entry(e.from.cell()).or_default().push(e.to.cell());
            }
            let s0 = node(st_ref(&start), false).cell();
            let mut seen = BTreeSet::from([s0]);
            let mut q = vec![s0];
            while let Some(c) = q.pop() {
                for &d in adj.get(&c).map(|v| v.as_slice()).unwrap_or(&[]) {
                    if seen.insert(d) {
                        q.push(d);
                    }
                }
            }
            let exits: Vec<(i32, i32)> = (0..84).flat_map(|r| (0..300).map(move |c| (c, r))).filter(|&(c, r)| marker(&st, c, r) == 54).collect();
            let exit = seen.iter().any(|&(c, r)| exits.iter().any(|&(ec, er)| (c as i32 - ec).abs() <= 1 && (r as i32 - er).abs() <= 1));
            let coins: BTreeSet<(i16, i16)> = pm.edges.iter().filter(|e| seen.contains(&e.from.cell())).flat_map(|e| e.coins.iter().copied()).collect();
            write("pokereach", &format!("poke graph from the start: {} cells of {}; exit reached {exit}; coin cells touched {} of {}\n", seen.len(), pm.nodes.len(), coins.len(), cc.len()));
        }
        "route" => {
            // A committed route replayed on the engine from the level's entry, against the map.
            let script = sf_conform::InputScript::load(std::path::Path::new(&opt("--script").unwrap())).unwrap();
            let start = level_start(id, &[]);
            let cc = coin_cells_of(start.state());
            let m = build_bfs(&start, false, threads, false, &cc);
            let cells: BTreeSet<(i16, i16)> = m.nodes.keys().map(|n| n.cell()).collect();
            let mut g = Game::with_display(sf_conform::display_data(), Start::Level(id));
            let lives = g.state().w(FROG + f::LIVES);
            let mut out = String::new();
            let mut last = (-1, -1);
            for fr in 0..opt("--frames").map_or(9000, |v| v.parse().unwrap()) {
                let x = script.input_at(fr);
                step(&mut g, x);
                let st = g.state();
                if !alive(st, lives) || !in_level(st) && fr > 200 {
                    let _ = writeln!(out, "{fr}: ended (alive {})", alive(st, lives));
                    break;
                }
                let c = cell(st);
                if c != last && landed(st) {
                    let inmap = cells.contains(&(c.0 as i16, c.1 as i16));
                    let _ = writeln!(out, "{fr}: {},{} {} input {} plat {} vx {} coins {}", c.0, c.1, if inmap { "map" } else { "OFF" },
                        sf_conform::InputScript::format_input(x), st.w(FROG + f::ON_PLATFORM), st.w(FROG + f::VX) as i16, st.w(FROG + f::COINS_LEFT));
                    last = c;
                }
            }
            write("route", &out);
        }
        "explore" => {
            let start = level_start(id, &[]);
            let cc = coin_cells_of(start.state());
            let m = build_bfs(&start, false, threads, true, &cc);
            let cells: BTreeSet<(i16, i16)> = m.nodes.keys().map(|n| n.cell()).collect();
            let acts = [Joy::NONE, Joy::RIGHT, Joy::LEFT, Joy::UP, Joy::RIGHT | Joy::UP, Joy::LEFT | Joy::UP, Joy::DOWN, Joy::FIRE];
            let names = ["-", "R", "L", "U", "RU", "LU", "D", "F"];
            let starts: Vec<(Node, Game)> = m.nodes.iter().map(|(n, g)| (*n, g.clone().unwrap())).collect();
            let bursts: usize = opt("--bursts").map_or(40, |v| v.parse().unwrap());
            let found = par_map(&starts, threads, |(n, g)| {
                let mut rng = Rng(0x9e37_79b9 ^ ((n.c as u64) << 20) ^ ((n.r as u64) << 8) | 1);
                let lives = g.state().w(FROG + f::LIVES);
                let mut new: Vec<((i16, i16), String)> = vec![];
                for _ in 0..bursts {
                    let mut g = g.clone();
                    let mut trail = String::new();
                    'b: for _ in 0..1 + rng.below(16) {
                        let a = rng.below(acts.len());
                        let k = 4 * (1 + rng.below(6));
                        trail += &format!("{}x{} ", names[a], k);
                        for _ in 0..k {
                            step(&mut g, FrameInput::joy(acts[a]));
                            let st = g.state();
                            if !alive(st, lives) || !in_level(st) {
                                break 'b;
                            }
                            let c = cell(st);
                            let c = (c.0 as i16, c.1 as i16);
                            if landed(st) && !cells.contains(&c) && !new.iter().any(|x| x.0 == c) {
                                new.push((c, trail.clone()));
                            }
                        }
                    }
                }
                (*n, new)
            });
            let mut all_new: BTreeMap<(i16, i16), (Node, String)> = BTreeMap::new();
            for (n, v) in found {
                for (c, t) in v {
                    all_new.entry(c).or_insert((n, t));
                }
            }
            let mut out = format!("map cells {}; explorer ({bursts} random bursts per node) found {} landed cells the map lacks\n", cells.len(), all_new.len());
            for (c, (n, t)) in all_new.iter().take(40) {
                let _ = writeln!(out, "  {},{} from {},{}: {t}", c.0, c.1, n.c, n.r);
            }
            write("explore", &out);
        }
        "ascii" => {
            let start = level_start(id, &[]);
            let st = start.state().clone();
            let cc = coin_cells_of(&st);
            let m = build_bfs(&start, false, threads, false, &cc);
            let cells: BTreeSet<(i16, i16)> = m.nodes.keys().map(|n| n.cell()).collect();
            let (c0, c1): (i32, i32) = (opt("--c0").map_or(0, |v| v.parse().unwrap()), opt("--c1").map_or(300, |v| v.parse().unwrap()));
            for r in 0..84 {
                let mut line = format!("{r:2} ");
                for c in c0..c1 {
                    let mk = marker(&st, c, r);
                    let ch = if cells.contains(&(c as i16, r as i16)) { 'o' }
                        else if matches!(mk, 8 | 9) { 'c' } else if mk == 6 { '^' } else if mk == 44 { 'L' }
                        else if (48..=51).contains(&mk) { 'S' } else if mk == 54 { 'E' } else if mk == 62 { '=' } else if mk >= 0x3a { '#' }
                        else if mk == 38 { '2' } else if mk == 0 { ' ' } else { char::from_digit((mk % 36) as u32, 36).unwrap() };
                    line.push(ch);
                }
                println!("{line}");
            }
        }
        "all" => all(id, threads, &write),
        "bands" => bands(id, threads, &write),
        _ => panic!("{cmd}?"),
    }
}

fn map_line(name: &str, m: &Map) -> String {
    let cells: BTreeSet<(i16, i16)> = m.nodes.keys().map(|n| n.cell()).collect();
    let wings = m.nodes.keys().filter(|n| n.wings != 0).count();
    let coin_edges = m.edges.iter().filter(|e| !e.coins.is_empty()).count();
    format!(
        "{name}: nodes {} (with wings {wings}), cells {}, edges {} (taking a coin {coin_edges}), \
         build {:.1}s, frames simulated {}, moves dropped: lost heart {}, died {}, never landed {}\n",
        m.nodes.len(),
        cells.len(),
        m.edges.len(),
        m.secs,
        m.sim,
        m.hearts_lost,
        m.died,
        m.noland
    )
}

fn all(id: LevelId, threads: usize, write: &dyn Fn(&str, &str)) {
    let ms = moves();
    let start = level_start(id, &[]);
    let st0 = start.state().clone();
    let coin_cells = coin_cells_of(&st0);
    let needed = st0.w(FROG + f::COINS_LEFT);
    let mut head = format!(
        "{id}: start cell {:?} frame {}, coins needed {needed}, coin cells {}, standable cells (static rule) {}, threads {threads}, load {}\n",
        cell(&st0),
        start.frame(),
        coin_cells.len(),
        (0..84).flat_map(|r| (0..300).map(move |c| (c, r))).filter(|&(c, r)| standable(&st0, c, r)).count(),
        std::process::Command::new("sysctl").args(["-n", "vm.loadavg"]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
    );
    // Live map, cells only.
    let live = build_bfs(&start, false, threads, true, &coin_cells);
    head += &map_line("live (enemies live, seeded by real moves)", &live);
    let coins_touched: BTreeSet<(i16, i16)> = live.edges.iter().flat_map(|e| e.coins.iter().copied()).collect();
    let coins_nowings: BTreeSet<(i16, i16)> = live
        .edges
        .iter()
        .filter(|e| e.from.wings == 0)
        .flat_map(|e| e.coins.iter().copied())
        .collect();
    let exits: Vec<(i16, i16)> = (0..84)
        .flat_map(|r| (0..300).map(move |c| (c, r)))
        .filter(|&(c, r)| marker(&st0, c, r) == 54)
        .map(|(c, r)| (c as i16, r as i16))
        .collect();
    let exit_near = live.nodes.keys().any(|n| exits.iter().any(|&(c, r)| (n.c - c).abs() <= 1 && (n.r - r).abs() <= 1));
    head += &format!(
        "  coin cells touched by an edge: {} (without wings {}) of {}; exit cell within one cell of a node: {exit_near}\n",
        coins_touched.len(),
        coins_nowings.len(),
        coin_cells.len()
    );
    write("maps", &head);

    // Q5 and the enemy classification.
    let spawns = spawning_markers(id);
    let mut kinds: BTreeMap<u16, Vec<SlotInfo>> = BTreeMap::new();
    let mut walked_cells: BTreeSet<(i32, i32)> = BTreeSet::new();
    for (&a, s) in &live.slots {
        if let Some((c, r, m)) = origin(id, a) {
            kinds.entry(m).or_default().push(*s);
            walked_cells.insert((c, r));
        }
    }
    let enemy_markers: Vec<u16> = kinds
        .iter()
        .filter(|(m, v)| !matches!(m, 0 | 8 | 9 | 29 | 45) && v.iter().any(|s| s.passive == 0))
        .map(|(m, _)| *m)
        .collect();
    let t = &sf_conform::game_data().levels[&id].t7mp;
    let mut marker_cells: BTreeMap<u16, usize> = BTreeMap::new();
    for r in 0..84 {
        for c in 0..300 {
            let m = t.cell(c, r) >> 10;
            if spawns.contains(&m) {
                *marker_cells.entry(m).or_default() += 1;
            }
        }
    }
    let mut q5 = format!("spawning markers (cells in the map): {marker_cells:?}\n");
    for (m, v) in &kinds {
        let s = v[0];
        let _ = writeln!(
            q5,
            "  marker {m:2}: {} slots seen; passive {:x} stompable {:x} hp {:x} touch h0+${:x} update h0+${:x}",
            v.len(),
            s.passive,
            s.stomp,
            s.hp,
            s.touch,
            s.update
        );
    }
    let _ = writeln!(q5, "enemy markers (a slot with contact that hurts): {enemy_markers:?}");
    // Parked: the frog held at its start for 2000 frames.
    let mut g = start.clone();
    let mut parked: BTreeSet<(i32, i32, u16)> = BTreeSet::new();
    let mut parked_first = 0;
    for k in 0..2000 {
        for (a, _) in slots_of(g.state()) {
            if let Some(o) = origin(id, a) {
                parked.insert(o);
            }
        }
        if k == 0 {
            parked_first = parked.len();
        }
        step(&mut g, FrameInput::default());
    }
    let enemy_cells_total: usize = enemy_markers.iter().map(|m| marker_cells.get(m).copied().unwrap_or(0)).sum();
    let parked_enemy = parked.iter().filter(|o| enemy_markers.contains(&o.2)).count();
    let walked_enemy = walked_cells
        .iter()
        .filter(|&&(c, r)| enemy_markers.contains(&(t.cell(c as u16, r as u16) >> 10)))
        .count();
    let _ = writeln!(
        q5,
        "Q5: enemy marker cells in the map {enemy_cells_total}; ever in a slot with the frog parked 2000 frames: {parked_enemy} \
         (all objects {}, at frame 0 {parked_first}); ever in a slot over the live map's moves (the level walked): {walked_enemy} (all objects {})",
        parked.len(),
        walked_cells.len()
    );
    write("q5-enemies", &q5);

    // Q2: enemies frozen (their markers cleared before the level is entered).
    let fstart = level_start(id, &enemy_markers);
    let frozen = build_bfs(&fstart, false, threads, false, &coin_cells);
    let mut q2 = map_line("live", &live) + &map_line("frozen", &frozen);
    let key = |e: &Edge| (e.from, e.mv);
    let fmap: BTreeMap<(Node, usize), Node> = frozen.edges.iter().map(|e| (key(e), e.to)).collect();
    let lmap: BTreeMap<(Node, usize), Node> = live.edges.iter().map(|e| (key(e), e.to)).collect();
    let both = lmap.keys().filter(|k| fmap.contains_key(k)).count();
    let differ = lmap.iter().filter(|(k, to)| fmap.get(k).is_some_and(|t| t != *to)).count();
    let mut h2 = [0usize; 5];
    for (k, to) in &lmap {
        if let Some(t) = fmap.get(k) {
            h2[miss(*to, *t)] += 1;
        }
    }
    let lcells: BTreeSet<(i16, i16)> = live.nodes.keys().map(|n| n.cell()).collect();
    let fcells: BTreeSet<(i16, i16)> = frozen.nodes.keys().map(|n| n.cell()).collect();
    let _ = writeln!(
        q2,
        "Q2: edges (from, move) in both {both}; landing differs {differ} ({:.1}%); live-only edges {}; frozen-only edges {}; \
         cells live-only {}, frozen-only {}",
        100.0 * differ as f64 / both.max(1) as f64,
        lmap.len() - both,
        fmap.len() - both,
        lcells.difference(&fcells).count(),
        fcells.difference(&lcells).count()
    );
    let _ = writeln!(q2, "  {}", miss_line(&h2));
    let platforms = st0_markers(&st0, &[44]);
    let _ = writeln!(q2, "marker 44 (lift) cells: {platforms}; marker 44 and 62 are tile markers (not in the spawn list {spawns:?}), never objects");
    write("q2-freeze", &q2);

    // Q7: live failure, from states reached by real moves, enemies live.
    let mut q7 = String::from("live walk on the live map:   ");
    q7 += &live_walk(&start, &live, 500, 0x9e37_79b9_7f4a_7c15, &coin_cells);
    q7 += "live walk on the frozen map: ";
    q7 += &live_walk(&start, &frozen, 500, 0x9e37_79b9_7f4a_7c15, &coin_cells);
    write("q7-live-failure", &q7);
    drop(frozen);

    // Q1: the poke-seeded map.
    let (poke, standable_n) = build_poke(&start, threads, &coin_cells);
    let mut q1 = map_line("poke (frog placed by writing X/Y)", &poke);
    let pcells: BTreeSet<(i16, i16)> = poke.nodes.keys().chain(poke.edges.iter().map(|e| &e.to)).map(|n| n.cell()).collect();
    // False edges: a poke edge replayed from the live map's real state at its start node.
    let checkable: Vec<&Edge> = poke
        .edges
        .iter()
        .filter(|e| live.nodes.get(&e.from).is_some_and(|g| g.is_some()))
        .collect();
    let res = par_map(&checkable, threads, |e| {
        let g = live.nodes[&e.from].as_ref().unwrap();
        match play(g, &ms[e.mv], true, &coin_cells) {
            Ok(p) if !p.hearts_lost => Some(miss(node(p.game.state(), false), e.to)),
            _ => None,
        }
    });
    let false_n = res.iter().filter(|&&ok| ok != Some(0)).count();
    let mut h1 = [0usize; 5];
    for r in res.iter().flatten() {
        h1[*r] += 1;
    }
    let failed_live = res.iter().filter(|r| r.is_none()).count();
    let _ = writeln!(
        q1,
        "Q1: standable cells {standable_n}, placed {}; cells covered poke {} vs live {} (live only {}, poke only {}); \
         poke edges checkable from a real state {}: false {false_n} ({:.1}%)",
        poke.nodes.len(),
        pcells.len(),
        lcells.len(),
        lcells.difference(&pcells).count(),
        pcells.difference(&lcells).count(),
        checkable.len(),
        100.0 * false_n as f64 / checkable.len().max(1) as f64
    );
    let _ = writeln!(q1, "  of the false: died, hurt or never landed {failed_live}; {}", miss_line(&h1));
    write("q1-seeding", &q1);
    drop(poke);

    // Q4: the stall coins on the live map.
    let stall = stall_coins(id, &st0, &coin_cells);
    let mut q4 = format!("Q4 stall coins: {}\n", stall.len());
    let (mut reach, mut reach_nw, mut replayed) = (0, 0, 0);
    for &((c, r), kind) in &stall {
        let edge = live
            .edges
            .iter()
            .filter(|e| e.coins.contains(&(c, r)))
            .min_by_key(|e| (path_to(&live, e.from).len(), e.frames));
        let Some(e) = edge else {
            let _ = writeln!(q4, "  {c},{r} {kind}: unreachable on the map");
            continue;
        };
        reach += 1;
        if e.from.wings == 0 {
            reach_nw += 1;
        }
        // Replay the path from the start, a fresh game.
        let mut path = path_to(&live, e.from);
        path.push(e.mv);
        let mut g = start.clone();
        let mut frames = 0;
        let mut ok = true;
        for &mv in &path {
            match play(&g, &ms[mv], true, &coin_cells) {
                Ok(p) => {
                    frames += p.frames;
                    g = p.game;
                }
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        let taken = ok && !matches!(marker(g.state(), c as i32, r as i32), 8 | 9);
        if taken {
            replayed += 1;
        }
        let _ = writeln!(
            q4,
            "  {c},{r} {kind}: reachable by {} moves, {frames} frames, last {} from {},{} wings {}; replays and takes it: {taken}",
            path.len(),
            ms[e.mv].name,
            e.from.c,
            e.from.r,
            e.from.wings
        );
    }
    let _ = writeln!(q4, "Q4: {reach}/{} reachable ({reach_nw} without wings), {replayed} replayed from the start taking the coin", stall.len());
    write("q4-stall", &q4);
}

fn st_ref(g: &Game) -> &sf_core::State {
    g.state()
}

fn st0_markers(st: &sf_core::State, ms: &[u16]) -> usize {
    (0..84).flat_map(|r| (0..300).map(move |c| (c, r))).filter(|&(c, r)| ms.contains(&marker(st, c, r))).count()
}

fn bands(id: LevelId, threads: usize, write: &dyn Fn(&str, &str)) {
    let start = level_start(id, &[]);
    let coin_cells = coin_cells_of(start.state());
    let cells = build_bfs(&start, false, threads, false, &coin_cells);
    let banded = build_bfs(&start, true, threads, false, &coin_cells);
    let mut s = map_line("cells only", &cells) + &map_line("cell + speed band + facing", &banded);
    // Edges whose landing cell changes with arrival speed: the same (cell, facing, wings, move)
    // from a moving band against the stopped band.
    let mut by: BTreeMap<((i16, i16), u8, u8, usize), BTreeMap<u8, (i16, i16)>> = BTreeMap::new();
    for e in &banded.edges {
        by.entry((e.from.cell(), e.from.facing, e.from.wings, e.mv))
            .or_default()
            .insert(e.from.band, e.to.cell());
    }
    let (mut moving, mut differ) = (0, 0);
    for v in by.values() {
        let Some(stopped) = v.get(&0) else { continue };
        for (&b, to) in v {
            if b != 0 {
                moving += 1;
                if to != stopped {
                    differ += 1;
                }
            }
        }
    }
    let bcells: BTreeSet<(i16, i16)> = banded.nodes.keys().map(|n| n.cell()).collect();
    let ccells: BTreeSet<(i16, i16)> = cells.nodes.keys().map(|n| n.cell()).collect();
    let _ = writeln!(
        s,
        "Q6: moving-band edges with a stopped counterpart {moving}; landing cell differs {differ} ({:.1}%); \
         cells reached only with bands {}, only cells-only {}",
        100.0 * differ as f64 / moving.max(1) as f64,
        bcells.difference(&ccells).count(),
        ccells.difference(&bcells).count()
    );
    write("q6-bands", &s);
}
