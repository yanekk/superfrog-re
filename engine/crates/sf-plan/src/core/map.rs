//! The movement map, measured (DESIGN §2.2, D15, D17, D18): a breadth-first walk from the level's
//! start state over real moves, every move played on the engine with enemies and everything else
//! live. A node's seed state is the one a real move reached it in, then left to stop; the frog is
//! never placed by writing its position (D18).
//!
//! The walk goes in layers. Each layer's work is a list of jobs (a frontier seed, a move), split
//! into contiguous parts the shell may run on threads (`Builder::parts`, `Builder::build_part`);
//! the results are absorbed in part order (`Builder::absorb`), so the map is the same for any part
//! count (DESIGN §3.1, M8). The core never spawns.
//!
//! The walk's key is the node plus the switches pressed in the seed state, so cells behind a gate
//! are measured from a state where a real move pressed its switch (the game's own handler) and the
//! map is built per switch set reachable from the start (§2.2).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::Range;

use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, State};

use super::facts::{Cell, Facts, cell_addr, cell_word, frog_cell, landed, live};
use super::graph::{Edge, MoveMap, Node};
use super::moves::{Macro, MoveFamily, MoveSet};

/// `level_state` (h0+$a06): −2 while the level is played.
const LEVEL_STATE: u32 = h0(0xa06);
/// The hearts left (h0+$830).
const HEARTS: u32 = h0(0x830);
/// Frames a landed frog is left to stop before its state seeds a node.
const STOP_CAP: u16 = 40;

/// Switches pressed in `st`, as bits over `facts.switches`: a pressed switch's cells no longer
/// carry its marker (`switch` rewrites the lever's cells).
pub fn pressed(facts: &Facts, st: &State) -> u32 {
    facts
        .switches
        .iter()
        .enumerate()
        .filter(|(_, s)| live(st, s.cells[0].0, s.cells[0].1) != s.marker as u16)
        .fold(0, |m, (k, _)| m | 1 << k)
}

fn wings(st: &State) -> bool {
    st.w(FROG + f::FLIGHT) != 0
}

fn node(st: &State) -> Node {
    Node {
        cell: frog_cell(st),
        wings: wings(st),
    }
}

fn in_level(st: &State) -> bool {
    st.l(LEVEL_STATE) == 0xffff_fffe
}

/// Why a move is not an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reject {
    /// A life lost or the frog dying.
    Died,
    /// A heart lost (an enemy, a fixed hazard): danger is judged live later (§2.5), never mapped.
    HeartLost,
    /// The frog's cell touched a `Facts::hazards` cell on the way (§2.2).
    Hazard,
    /// Still in the air after its cap.
    NoLand,
}

/// What playing inputs on a copy of a state did, frame by frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// The cell the frog ended in and whether it holds wings.
    pub end: Node,
    pub landed: bool,
    /// The level ended with the frog alive: it went through the exit.
    pub exited: bool,
    pub reject: Option<Reject>,
    /// `Facts::coins` indices whose cell changed on the way (taken).
    pub coins: Vec<u16>,
    /// `Facts::switches` bits whose gate block (`in_gate_block`) the frog's cell entered.
    pub near_gates: u32,
    pub frames: u32,
}

/// Steps a copy of a state, watching what the map needs: the frog's cell against hazards and
/// gates, life and hearts, the level ending.
struct Watch<'a> {
    facts: &'a Facts,
    hazards: &'a HashSet<Cell>,
    lives: u16,
    hearts: u16,
    coins_at_start: Vec<u16>,
    near_gates: u32,
    frames: u32,
    reject: Option<Reject>,
    exited: bool,
}

impl<'a> Watch<'a> {
    fn new(g: &Game, facts: &'a Facts, hazards: &'a HashSet<Cell>) -> Self {
        let st = g.state();
        Watch {
            facts,
            hazards,
            lives: st.w(FROG + f::LIVES),
            hearts: st.w(HEARTS),
            coins_at_start: facts.coins.iter().map(|c| cell_word(st, c.cell)).collect(),
            near_gates: 0,
            frames: 0,
            reject: None,
            exited: false,
        }
    }

    /// One frame; false once the move is over (rejected or out of the level).
    fn step(&mut self, g: &mut Game, joy: Joy) -> bool {
        g.step_unseen(FrameInput::joy(joy));
        self.frames += 1;
        let st = g.state();
        if (st.w(FROG + f::LIVES) as i16) < self.lives as i16 || st.w(FROG + f::DYING) != 0 {
            self.reject = Some(Reject::Died);
            return false;
        }
        if st.w(HEARTS) < self.hearts {
            self.reject = Some(Reject::HeartLost);
            return false;
        }
        let cell = frog_cell(st);
        if self.hazards.contains(&cell) {
            self.reject = Some(Reject::Hazard);
            return false;
        }
        for (k, s) in self.facts.switches.iter().enumerate() {
            if in_gate_block(s.gate, cell) {
                self.near_gates |= 1 << k;
            }
        }
        if !in_level(st) {
            self.exited = true;
            return false;
        }
        true
    }

    fn finish(self, g: &Game) -> Trace {
        let st = g.state();
        let coins = self
            .facts
            .coins
            .iter()
            .zip(&self.coins_at_start)
            .enumerate()
            .filter(|(_, (c, was))| cell_word(st, c.cell) != **was)
            .map(|(k, _)| k as u16)
            .collect();
        Trace {
            end: node(st),
            landed: landed(st),
            exited: self.exited,
            reject: self.reject,
            coins,
            near_gates: self.near_gates,
            frames: self.frames,
        }
    }
}

/// Plays `inputs` exactly on a copy of `start` and reports what happened; stops early on a
/// rejection or the level ending. The replay check of every map edge (§2.2, tests/map.rs).
pub fn replay(start: &Game, facts: &Facts, inputs: &[(u8, u16)]) -> Trace {
    let hazards: HashSet<Cell> = facts.hazards.iter().copied().collect();
    let mut g = start.clone();
    let mut w = Watch::new(&g, facts, &hazards);
    'all: for &(joy, n) in inputs {
        for _ in 0..n {
            if !w.step(&mut g, Joy(joy)) {
                break 'all;
            }
        }
    }
    w.finish(&g)
}

/// One move played from a seed: the edge it makes (if any), and the state it leaves for a new
/// node's seed (if its key is new to the part).
#[derive(Clone)]
pub struct Probe {
    pub result: Result<Found, Reject>,
    /// Frames stepped, the stop included: the build's machine-independent cost.
    pub frames: u32,
}

#[derive(Clone)]
pub struct Found {
    /// Held inputs, the tail to landing included: replaying them reproduces the move exactly.
    pub inputs: Vec<(u8, u16)>,
    pub frames: u32,
    pub to: Node,
    pub coins: Vec<u16>,
    pub needs_switches: u32,
    pub exited: bool,
    /// The landed state, stopped, keyed: a seed for a node not yet walked.
    pub seed: Option<(Key, Game)>,
    /// When the frog slid on into another cell before it stopped: the move with the stop, as an
    /// edge to that cell, so the node the stopped state seeds has a way in.
    pub stopped: Option<Stopped>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stopped {
    pub inputs: Vec<(u8, u16)>,
    pub frames: u32,
    pub to: Node,
    pub coins: Vec<u16>,
}

/// A walked node: the map node plus the switches pressed in its seed state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    pub node: Node,
    pub pressed: u32,
}

fn push(v: &mut Vec<(u8, u16)>, joy: Joy, n: u16) {
    match v.last_mut() {
        Some((j, k)) if *j == joy.0 && *k < u16::MAX - n => *k += n,
        _ if n > 0 => v.push((joy.0, n)),
        _ => {}
    }
}

/// Frames a move that lost a heart is retried after, standing on its seed: an enemy's gap
/// (DESIGN §2.5's waits). A variant is its own standing-start move from the seed's node, its wait
/// recorded in its inputs (W4-2's updraft at 63-66,74 has an enemy beside its only seed, T04).
const WAITS: [u16; 3] = [12, 25, 50];

/// Plays a move from a seed, and if it loses a heart, again after each of `WAITS` until one
/// does not.
fn play(seed: &Game, from: Key, mac: &Macro, facts: &Facts, hazards: &HashSet<Cell>) -> Probe {
    let mut p = play_after(seed, from, mac, facts, hazards, 0);
    for wait in WAITS {
        if !matches!(p.result, Err(Reject::HeartLost)) {
            break;
        }
        let spent = p.frames;
        p = play_after(seed, from, mac, facts, hazards, wait);
        p.frames += spent;
    }
    p
}

/// Plays a move from a seed after `wait` frames standing: its holds, then its tail until landed;
/// then, for the seed, nothing until the frog stops (not part of the edge: an edge ends landed at
/// whatever speed, D15).
fn play_after(seed: &Game, from: Key, mac: &Macro, facts: &Facts, hazards: &HashSet<Cell>, wait: u16) -> Probe {
    let mut g = seed.clone();
    let mut w = Watch::new(&g, facts, hazards);
    let mut inputs = Vec::new();
    let mut going = true;
    for _ in 0..wait {
        push(&mut inputs, Joy::NONE, 1);
        if !w.step(&mut g, Joy::NONE) {
            going = false;
            break;
        }
    }
    'holds: for &(joy, n) in &mac.holds {
        if !going {
            break;
        }
        for _ in 0..n {
            push(&mut inputs, joy, 1);
            if !w.step(&mut g, joy) {
                going = false;
                break 'holds;
            }
        }
    }
    let mut air = 0;
    while going && !landed(g.state()) {
        if air == mac.land_cap {
            w.reject = Some(Reject::NoLand);
            break;
        }
        push(&mut inputs, mac.tail, 1);
        going = w.step(&mut g, mac.tail);
        air += 1;
    }
    let mut frames = w.frames;
    let tr = w.finish(&g);
    if let Some(r) = tr.reject {
        return Probe {
            result: Err(r),
            frames,
        };
    }
    let needs_switches = from.pressed & tr.near_gates;
    let mut seed = None;
    let mut stopped = None;
    if !tr.exited {
        // Left to stop: the node's state is the frog standing (edges start stopped, D15/D18). A
        // frog that cannot stop there (a spring throws it on, W6-3's 118,14 and 150,12) seeds the
        // node with its landing frame instead, so moves start on the pad (§2.2's launches).
        let at_land = g.clone();
        let mut sw = Watch::new(&g, facts, hazards);
        let mut k = 0;
        let mut ok = true;
        while k < STOP_CAP && !(landed(g.state()) && g.state().w(FROG + f::VX) == 0) {
            if !sw.step(&mut g, Joy::NONE) {
                ok = false;
                break;
            }
            k += 1;
        }
        frames += sw.frames;
        let settled = ok && landed(g.state()) && g.state().w(FROG + f::VX) == 0;
        let stop = sw.finish(&g);
        if settled && stop.end != tr.end {
            let mut inputs = inputs.clone();
            push(&mut inputs, Joy::NONE, stop.frames as u16);
            let mut coins: Vec<u16> = tr.coins.iter().chain(&stop.coins).copied().collect();
            coins.sort_unstable();
            coins.dedup();
            stopped = Some(Stopped {
                inputs,
                frames: tr.frames + stop.frames,
                to: stop.end,
                coins,
            });
        }
        let g = if settled { g } else { at_land };
        let st = g.state();
        let key = Key {
            node: node(st),
            pressed: pressed(facts, st),
        };
        seed = Some((key, g));
    }
    Probe {
        result: Ok(Found {
            frames: tr.frames,
            inputs,
            to: tr.end,
            coins: tr.coins,
            needs_switches,
            exited: tr.exited,
            seed,
            stopped,
        }),
        frames,
    }
}

/// A slice of the current layer's jobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapPart {
    pub layer: u32,
    pub jobs: Range<u32>,
}

/// Counts for the build's FINDINGS row; all machine-independent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildStats {
    pub layers: u32,
    /// Seeds walked.
    pub keys: u32,
    pub moves_played: u64,
    pub frames: u64,
    pub rejected: BTreeMap<Reject, u32>,
    /// Walked keys per switch set pressed in their seeds.
    pub per_switch_set: BTreeMap<u32, u32>,
}

/// An edge as found, before nodes are numbered.
#[derive(Clone)]
struct RawEdge {
    from: Node,
    to: Node,
    inputs: Vec<(u8, u16)>,
    frames: u32,
    coins: Vec<u16>,
    needs_switches: u32,
    needs_wings: bool,
}

/// The breadth-first walk, one layer at a time. `parts` → `build_part` (any thread, any order) →
/// `absorb` (in part order), until `parts` is empty; then `finish`.
pub struct Builder {
    facts: Facts,
    hazards: HashSet<Cell>,
    moves: Vec<(MoveFamily, Macro)>,
    layer: u32,
    frontier: Vec<(Key, Game)>,
    /// (frontier index, move index).
    jobs: Vec<(u32, u16)>,
    /// Seeds walked per slot (`Builder::slot`).
    walked: HashMap<(Node, u32), u8>,
    nodes: BTreeSet<Node>,
    edges: Vec<RawEdge>,
    stats: BuildStats,
    /// Per switch, the map words its handler wrote at the first real press found.
    presses: BTreeMap<usize, Vec<(u32, u16)>>,
    /// Switch sets a real move has reached.
    worlds: BTreeSet<u32>,
    /// Walked seeds within `GATE_REACH` of a gate, kept to be walked again in each world where
    /// their gate is open.
    gate_seeds: Vec<(Key, Game)>,
}

/// Seeds walked per slot: a node (cell and wings) with the switches pressed whose gates are near
/// it. Two (a second arrival, at another time, among other enemy positions) doubled W1-1's build
/// and did not bring back W2-1's pad shaft, which it was tried for (T04).
pub const SEEDS_PER_SLOT: u8 = 1;

/// How far from a gate (in cells, either way) a pressed switch is taken to change what moves do:
/// a node this near a gate is walked per switch set of its near gates, any other once per slot
/// whatever is pressed (walking every switch set fully took W2-2 36.8 M frames and W2-3, four
/// switches, over an hour unfinished, T04).
pub const GATE_REACH: u16 = 10;

/// Whether `cell` is in the block a gate at `gate` opens, with a cell's margin: `switch` rewrites
/// five rows from the row above the gate, the two columns right of it (W2-1's 143,37 opens
/// 144-145,36-40).
fn in_gate_block(gate: Cell, cell: Cell) -> bool {
    (gate.0.saturating_sub(1)..=gate.0 + 3).contains(&cell.0)
        && (gate.1.saturating_sub(2)..=gate.1 + 4).contains(&cell.1)
}

/// Cells a switch's handler may write: its gate's block (`switch` rewrites the five rows from the
/// row above the gate, the two columns right of it, world by world) and the lever's cells, with a
/// cell's margin.
fn press_region(facts: &Facts, k: usize) -> Vec<Cell> {
    let s = &facts.switches[k];
    let block = |c0: u16, c1: u16, r0: u16, r1: u16| {
        (r0..=r1).flat_map(move |r| (c0..=c1).map(move |c| (c, r)))
    };
    let (g, top, bottom) = (s.gate, s.cells[0], s.cells[1]);
    let sub = |a: u16, b: u16| a.saturating_sub(b);
    block(sub(g.0, 1), g.0 + 3, sub(g.1, 2), g.1 + 4)
        .filter(move |&c| in_gate_block(g, c))
        .chain(block(sub(top.0, 1), top.0 + 1, sub(top.1, 1), bottom.1 + 1))
        .filter(|&(c, r)| c < super::facts::COLS && r < super::facts::ROWS)
        .filter(|cell| !facts.coins.iter().any(|k| k.cell == *cell))
        .collect()
}

impl Builder {
    pub fn new(start: &Game, facts: &Facts, moves: &MoveSet) -> Builder {
        let st = start.state();
        let key = Key {
            node: node(st),
            pressed: pressed(facts, st),
        };
        let mut b = Builder {
            facts: facts.clone(),
            hazards: facts.hazards.iter().copied().collect(),
            moves: moves.families.iter().map(|&m| (m, m.expand())).collect(),
            layer: 0,
            frontier: Vec::new(),
            jobs: Vec::new(),
            walked: HashMap::new(),
            nodes: BTreeSet::new(),
            edges: Vec::new(),
            stats: BuildStats::default(),
            presses: BTreeMap::new(),
            worlds: BTreeSet::from([key.pressed]),
            gate_seeds: Vec::new(),
        };
        let mut first = Vec::new();
        b.admit(key, start.clone(), &mut first);
        b.frontier = first;
        b.plan_jobs();
        b
    }

    fn plan_jobs(&mut self) {
        self.jobs = self
            .frontier
            .iter()
            .enumerate()
            .flat_map(|(i, (key, _))| {
                let w = key.node.wings;
                self.moves
                    .iter()
                    .enumerate()
                    .filter(move |(_, (_, m))| w || !m.needs_wings)
                    .map(move |(k, _)| (i as u32, k as u16))
            })
            .collect();
    }

    /// The current layer's jobs cut into `n` contiguous parts (fewer if there are fewer jobs);
    /// empty when the walk is done.
    pub fn parts(&self, n: usize) -> Vec<MapPart> {
        let len = self.jobs.len() as u32;
        let n = (n.max(1) as u32).min(len);
        (0..n)
            .map(|k| MapPart {
                layer: self.layer,
                jobs: len * k / n..len * (k + 1) / n,
            })
            .collect()
    }

    /// Plays a part's jobs. A seed state is kept only for a key the walk has not seen and the part
    /// has not already kept, so a layer holds few states at once; or for a move that presses a
    /// switch whose writes are not yet known (`absorb` reads them off it).
    pub fn build_part(&self, part: &MapPart) -> Vec<Probe> {
        assert_eq!(part.layer, self.layer, "a part of another layer");
        let mut kept: HashMap<(Node, u32), u8> = HashMap::new();
        self.jobs[part.jobs.start as usize..part.jobs.end as usize]
            .iter()
            .map(|&(i, m)| {
                let (from, seed) = &self.frontier[i as usize];
                let mut p = play(seed, *from, &self.moves[m as usize].1, &self.facts, &self.hazards);
                if let Ok(found) = &mut p.result
                    && let Some((key, _)) = &found.seed
                {
                    let slot = self.slot(key);
                    let had = self.walked.get(&slot).copied().unwrap_or(0);
                    let k = kept.entry(slot).or_default();
                    // Near a gate every seed is kept: `absorb` may write a press into it.
                    if self.first_press(from.pressed, key.pressed) || self.near_gates(u32::MAX, key.node.cell) {
                    } else if had + *k >= SEEDS_PER_SLOT {
                        found.seed = None;
                    } else {
                        *k += 1;
                    }
                }
                p
            })
            .collect()
    }

    /// The layer's results, in part order; the next layer is planned from the new seeds.
    pub fn absorb(&mut self, parts: Vec<Vec<Probe>>) {
        let probes: Vec<Probe> = parts.into_iter().flatten().collect();
        assert_eq!(probes.len(), self.jobs.len(), "a layer's results are incomplete");
        let mut next = Vec::new();
        let jobs = std::mem::take(&mut self.jobs);
        let frontier = std::mem::take(&mut self.frontier);
        for (&(i, m), p) in jobs.iter().zip(probes) {
            self.stats.moves_played += 1;
            self.stats.frames += p.frames as u64;
            let from = frontier[i as usize].0;
            let found = match p.result {
                Ok(f) => f,
                Err(r) => {
                    *self.stats.rejected.entry(r).or_default() += 1;
                    continue;
                }
            };
            if let Some((key, g)) = &found.seed
                && self.first_press(from.pressed, key.pressed)
            {
                self.record_press(&frontier[i as usize].1, g, from.pressed, key.pressed);
            }
            if let Some((key, _)) = &found.seed
                && self.worlds.insert(key.pressed)
            {
                // A switch set reached by a real move: every seed walked near one of its newly open
                // gates is walked again with the presses written in, so a cell behind the gate is
                // not missed for want of the one seed that pressed it (W6-3's 161,7, T04).
                let world = key.pressed;
                let born: Vec<(Key, Game)> = self
                    .gate_seeds
                    .iter()
                    .filter(|(k, _)| self.opens_near(k, world))
                    .filter_map(|(k, g)| self.variant(g, k.pressed, world))
                    .collect();
                for (k, g) in born {
                    self.admit(k, g, &mut next);
                }
            }
            if let Some((key, g)) = found.seed {
                self.admit(key, g, &mut next);
            }
            let needs_wings = self.moves[m as usize].1.needs_wings;
            if let Some(s) = found.stopped
                && s.to != from.node
            {
                self.nodes.insert(s.to);
                self.edges.push(RawEdge {
                    from: from.node,
                    to: s.to,
                    inputs: s.inputs,
                    frames: s.frames,
                    coins: s.coins,
                    needs_switches: found.needs_switches,
                    needs_wings,
                });
            }
            if found.to == from.node {
                continue;
            }
            self.nodes.insert(found.to);
            self.edges.push(RawEdge {
                from: from.node,
                to: found.to,
                inputs: found.inputs,
                frames: found.frames,
                coins: found.coins,
                needs_switches: found.needs_switches,
                needs_wings,
            });
        }

        self.stats.layers += 1;
        self.layer += 1;
        self.frontier = next;
        self.plan_jobs();
    }

    /// Bits of the switches whose gates are within `GATE_REACH` of `cell`, of those in `bits`.
    fn gates_near(&self, bits: u32, cell: Cell) -> u32 {
        self.facts
            .switches
            .iter()
            .enumerate()
            .filter(|(k, s)| {
                bits & 1 << k != 0
                    && s.gate.0.abs_diff(cell.0) <= GATE_REACH
                    && s.gate.1.abs_diff(cell.1) <= GATE_REACH
            })
            .fold(0, |m, (k, _)| m | 1 << k)
    }

    fn near_gates(&self, bits: u32, cell: Cell) -> bool {
        self.gates_near(bits, cell) != 0
    }

    /// Where a seed counts: its node and the pressed switches whose gates are near it.
    fn slot(&self, key: &Key) -> (Node, u32) {
        (key.node, key.pressed & self.gates_near(key.pressed, key.node.cell))
    }

    /// World `world` presses more than `k`'s switches, one of the extra ones with its gate near.
    fn opens_near(&self, k: &Key, world: u32) -> bool {
        k.pressed & !world == 0 && k.pressed != world && self.near_gates(world & !k.pressed, k.node.cell)
    }

    /// `g` (walked with switches `from`) with the presses of `to`'s other switches written in, as
    /// their handlers wrote them; `None` if a press is not known or does not read as pressed.
    fn variant(&self, g: &Game, from: u32, to: u32) -> Option<(Key, Game)> {
        let mut g = g.clone();
        for k in 0..self.facts.switches.len() {
            if (to & !from) & 1 << k != 0 {
                for &(addr, w) in self.presses.get(&k)? {
                    g.poke(addr, &w.to_be_bytes());
                }
            }
        }
        let st = g.state();
        let key = Key {
            node: node(st),
            pressed: pressed(&self.facts, st),
        };
        (key.pressed == to).then_some((key, g))
    }

    /// A seed into the next layer, unless its slot has its `SEEDS_PER_SLOT` already. A seed near a
    /// gate is kept, and walked again with the presses written in for every bigger world reached.
    fn admit(&mut self, key: Key, g: Game, next: &mut Vec<(Key, Game)>) {
        let slot = self.slot(&key);
        let n = self.walked.entry(slot).or_default();
        if *n >= SEEDS_PER_SLOT {
            return;
        }
        *n += 1;
        self.stats.keys += 1;
        self.nodes.insert(key.node);
        *self.stats.per_switch_set.entry(key.pressed).or_default() += 1;
        if self.near_gates(u32::MAX, key.node.cell) {
            self.gate_seeds.push((key, g.clone()));
            let bigger: Vec<u32> = self.worlds.iter().copied().filter(|&w| self.opens_near(&key, w)).collect();
            for w in bigger {
                if let Some((k, v)) = self.variant(&g, key.pressed, w) {
                    self.admit(k, v, next);
                }
            }
        }
        next.push((key, g));
    }

    /// A move from switch set `from` to `to` presses a switch whose writes are not yet recorded.
    fn first_press(&self, from: u32, to: u32) -> bool {
        let new = to & !from;
        new != 0 && (0..self.facts.switches.len()).any(|k| new & 1 << k != 0 && !self.presses.contains_key(&k))
    }

    /// The handler's writes for each switch first pressed by a move from `before` to `after`: the
    /// words of its region that changed (a coin's cell never counts).
    fn record_press(&mut self, before: &Game, after: &Game, from: u32, to: u32) {
        for k in 0..self.facts.switches.len() {
            if (to & !from) & 1 << k == 0 || self.presses.contains_key(&k) {
                continue;
            }
            let writes = press_region(&self.facts, k)
                .into_iter()
                .filter(|&c| cell_word(before.state(), c) != cell_word(after.state(), c))
                .map(|c| (cell_addr(c), cell_word(after.state(), c)))
                .collect();
            self.presses.insert(k, writes);
        }
    }

    /// The current layer's seeds (for checks that replay from them).
    pub fn frontier(&self) -> &[(Key, Game)] {
        &self.frontier
    }

    /// The current layer's jobs, (frontier index, move index into the set).
    pub fn jobs(&self) -> &[(u32, u16)] {
        &self.jobs
    }

    pub fn stats(&self) -> &BuildStats {
        &self.stats
    }

    /// The map: nodes numbered in their sorted order, and per (from, to, preconditions, coins
    /// touched) only the fewest-frames edge, the first found on a tie.
    pub fn finish(self) -> MoveMap {
        let nodes: Vec<Node> = self.nodes.into_iter().collect();
        let index: HashMap<Node, u32> = nodes.iter().enumerate().map(|(k, &n)| (n, k as u32)).collect();
        let mut kept: Vec<Edge> = Vec::new();
        let mut at: HashMap<(u32, u32, u32, bool, Vec<u16>), usize> = HashMap::new();
        for e in self.edges {
            let edge = Edge {
                from: index[&e.from],
                to: index[&e.to],
                inputs: e.inputs,
                frames: e.frames,
                coins: e.coins,
                needs_switches: e.needs_switches,
                needs_wings: e.needs_wings,
            };
            let k = (edge.from, edge.to, edge.needs_switches, edge.needs_wings, edge.coins.clone());
            match at.get(&k) {
                Some(&j) if kept[j].frames <= edge.frames => {}
                Some(&j) => kept[j] = edge,
                None => {
                    at.insert(k, kept.len());
                    kept.push(edge);
                }
            }
        }
        MoveMap::new(nodes, kept)
    }
}

/// Runs the walk to the end with `n` parts a layer, each part through `run` in part order. `run`
/// is how the shell puts parts on threads; here, one after another.
pub fn build_in(start: &Game, facts: &Facts, moves: &MoveSet, n: usize) -> (MoveMap, BuildStats) {
    let mut b = Builder::new(start, facts, moves);
    loop {
        let parts = b.parts(n);
        if parts.is_empty() {
            break;
        }
        let results = parts.iter().map(|p| b.build_part(p)).collect();
        b.absorb(results);
    }
    let stats = b.stats().clone();
    (b.finish(), stats)
}

/// The map on one thread.
pub fn build(start: &Game, facts: &Facts, moves: &MoveSet) -> MoveMap {
    build_in(start, facts, moves, 1).0
}

/// What the map reaches from a node, every switch and the wings allowed (the most the map can
/// show without an enemy's help: D13's check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    /// Nodes reachable.
    pub nodes: usize,
    /// `Facts::coins` indices some reachable edge touches.
    pub coins: BTreeSet<u16>,
    /// A reachable node stands within one cell of an exit cell.
    pub exit: bool,
}

pub fn reach(map: &MoveMap, facts: &Facts, from: u32, switches: u32, wings: bool) -> Reach {
    let d = super::graph::shortest(map, from, switches, wings);
    let ok = |n: u32| d[n as usize] != u32::MAX;
    let coins = map
        .edges
        .iter()
        .filter(|e| ok(e.from) && ok(e.to))
        .filter(|e| e.needs_switches & !switches == 0 && (wings || !e.needs_wings))
        .flat_map(|e| e.coins.iter().copied())
        .collect();
    let exit = map.nodes.iter().enumerate().any(|(k, n)| {
        ok(k as u32)
            && facts
                .exits
                .iter()
                .any(|x| x.0.abs_diff(n.cell.0) <= 1 && x.1.abs_diff(n.cell.1) <= 1)
    });
    Reach {
        nodes: d.iter().filter(|&&x| x != u32::MAX).count(),
        coins,
        exit,
    }
}

/// The map node a state stands in, if the map has it.
pub fn node_of(map: &MoveMap, st: &State) -> Option<u32> {
    map.nodes.binary_search(&node(st)).ok().map(|k| k as u32)
}
