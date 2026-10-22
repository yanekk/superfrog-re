//! The planner proper (DESIGN §2.4, D19): a whole-level explorer on the engine. One archive of
//! cells; each round chooses cells by novelty, progress in coins, a pull toward the goal, the guide
//! (§2.11) and hearts, and plays a random burst of joystick holds from each, offering the state
//! after every hold to the archive, until a burst leaves the level alive.
//!
//! Deterministic for a seed (M8): the round's choices and each burst's seed come from one
//! generator; every burst starts from the archive as it stood at the round's start; the shell runs
//! a round's parts on threads (`round_parts`, `run_part`) and `merge` takes their results in choice
//! order, so the route is the same for any thread or part count. The constants are the
//! experiment's (`plans/smart-planner/spike-explore/`); T14 tunes them, never per level.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, State};

use super::archive::{Archive, COINS_LEFT, CellKey, HEARTS, Offer, cell_key};
use super::facts::{Cell, Facts, frog_cell};
use super::graph::{MoveMap, shortest};
use super::guide::{GuideIndex, GuidePoint};

/// `level_state` (h0+$a06): −2 while the level is played.
const LEVEL_STATE: u32 = h0(0xa06);
const PLAYING: u32 = 0xffff_fffe;

/// The burst's joystick states (DESIGN §2.4): none, the four directions, up with left or right,
/// fire, fire with left or right.
const ACTS: [Joy; 10] = [
    Joy::NONE,
    Joy::RIGHT,
    Joy::LEFT,
    Joy::UP,
    Joy(Joy::RIGHT.0 | Joy::UP.0),
    Joy(Joy::LEFT.0 | Joy::UP.0),
    Joy::DOWN,
    Joy::FIRE,
    Joy(Joy::RIGHT.0 | Joy::FIRE.0),
    Joy(Joy::LEFT.0 | Joy::FIRE.0),
];
const MAX_HOLDS: usize = 16;
const HOLD_FRAMES: [u32; 4] = [4, 8, 16, 32];
/// The chance a hold keeps the last hold's joystick state.
const KEEP: f64 = 0.6;
/// The idle frames the frog must survive after the level ends for it to count as an exit.
const EXIT_TAIL: u32 = 40;
/// Every this many frames a `seed_inputs` replay offers its state.
const SEED_EVERY: usize = 8;
/// The map compass's frames per cell, to put its distance in the pull's units (cells): the frog
/// runs about 2 pixels a frame, a 16-pixel cell in 8.
const FRAMES_PER_CELL: f64 = 8.0;

/// The pull toward the goal, `d` in cells (DESIGN §2.4).
fn pull(d: f64) -> f64 {
    4.0 * (-d / 12.0).exp() + 0.2
}

/// What gives the explorer its sense of direction (DESIGN §2.2).
#[derive(Debug, Clone, Default)]
pub enum Compass {
    /// |Δcol| + |Δrow| to the nearest exit cell.
    #[default]
    Straight,
    /// Edge frames to the nearest exit node over T04's map, straight where the map has no path.
    Map(MoveMap),
}

/// What the search is for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Goal {
    /// The level left alive (DESIGN §2.4).
    #[default]
    Exit,
    /// A state with this key within `within_frames` of the start (T22's shortcut pass).
    Key { key: CellKey, within_frames: u32 },
}

#[derive(Debug, Clone)]
pub struct Options {
    pub seed: u64,
    /// No route longer than this (DESIGN §5.2).
    pub max_frames: u32,
    /// The archive's own count of stored bytes stays under this (D22).
    pub max_mem_bytes: u64,
    /// Fixed by options, never by the machine (M8).
    pub bursts_per_round: u32,
    pub compass: Compass,
    /// The guide path (§2.11); empty: unguided.
    pub guide: Vec<GuidePoint>,
    /// `--follow`'s inputs (§2.7): replayed from the start, every 8th state archived until death.
    /// Joystick only: keys held in them are not replayed.
    pub seed_inputs: Option<Vec<FrameInput>>,
    pub goal: Goal,
    /// Hints (§2.10): cells to pass within one cell of, in order, before the goal counts; boxes
    /// (corners inclusive) a burst entering ends in.
    pub via: Vec<Cell>,
    pub avoid: Vec<(Cell, Cell)>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            seed: 1,
            max_frames: 30_000,
            max_mem_bytes: 4 << 30,
            bursts_per_round: 80,
            compass: Compass::Straight,
            guide: Vec::new(),
            seed_inputs: None,
            goal: Goal::Exit,
            via: Vec::new(),
            avoid: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// From the explorer's start to the frame the level ends (or the goal key is reached).
    pub inputs: Vec<FrameInput>,
    /// Engine frames simulated, bursts and rebuilds both.
    pub engine_frames: u64,
    pub hearts_lost: u8,
    pub rounds: u64,
    pub cells: u32,
}

/// Why no route (DESIGN §2.9), from the archive's furthest entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reason {
    pub coins_left: u16,
    /// The least compass distance, in cells, of an entry with the coins met.
    pub exit_distance_with_coins_met: Option<u16>,
    pub cells: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    Working,
    Exit(Route),
}

/// A burst chosen: (choice index, entry, burst seed).
type Pick = (u32, u32, u64);

/// One round's share of bursts, the picks of one entry together.
#[derive(Debug, Clone, Default)]
pub struct Part {
    picks: Vec<Pick>,
}

impl Part {
    pub fn len(&self) -> usize {
        self.picks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.picks.is_empty()
    }
}

/// A state a burst offers after a hold.
struct Held {
    len: usize,
    key: CellKey,
    hearts: u16,
    coins_left: u16,
    game: Option<Game>,
}

struct Burst {
    pick: u32,
    /// The chosen entry's checkpoint and suffix, as the round started.
    base: u32,
    prefix: Vec<u8>,
    inputs: Vec<u8>,
    held: Vec<Held>,
    /// The burst's frames to the goal, and the hearts there.
    exit: Option<(usize, u16)>,
}

pub struct PartResult {
    bursts: Vec<Burst>,
    /// Checkpoint states rebuilt for this part's bursts, to store again.
    rebuilt: Vec<(u32, Game)>,
    frames: u64,
}

/// The map compass's distances, per switch set and wings, cached.
struct MapCompass {
    /// The map with its edges reversed: shortest paths from the exits are distances to them.
    reversed: MoveMap,
    exits: Vec<u32>,
    by_cell: HashMap<Cell, Vec<u32>>,
    cache: HashMap<(u32, bool), Vec<u32>>,
}

impl MapCompass {
    fn new(map: MoveMap, facts: &Facts) -> MapCompass {
        let exits = map
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                facts
                    .exits
                    .iter()
                    .any(|x| x.0.abs_diff(n.cell.0) <= 1 && x.1.abs_diff(n.cell.1) <= 1)
            })
            .map(|(k, _)| k as u32)
            .collect();
        let mut by_cell: HashMap<Cell, Vec<u32>> = HashMap::new();
        for (k, n) in map.nodes.iter().enumerate() {
            by_cell.entry(n.cell).or_default().push(k as u32);
        }
        let edges = map
            .edges
            .into_iter()
            .map(|mut e| {
                std::mem::swap(&mut e.from, &mut e.to);
                e
            })
            .collect();
        MapCompass {
            reversed: MoveMap::new(map.nodes, edges),
            exits,
            by_cell,
            cache: HashMap::new(),
        }
    }

    /// Edge frames from `key`'s cells to the nearest exit node, if the map has a path.
    fn frames(&mut self, key: &CellKey) -> Option<u32> {
        let d = self
            .cache
            .entry((key.switches, key.wings))
            .or_insert_with(|| {
                let mut d = vec![u32::MAX; self.reversed.nodes.len()];
                for &x in &self.exits {
                    for (a, b) in
                        d.iter_mut()
                            .zip(shortest(&self.reversed, x, key.switches, key.wings))
                    {
                        *a = (*a).min(b);
                    }
                }
                d
            });
        let cells =
            (0..2).flat_map(|dc| (0..2).map(move |dr| (key.col * 2 + dc, key.row * 2 + dr)));
        cells
            .filter_map(|c| self.by_cell.get(&c))
            .flatten()
            .filter(|&&n| self.reversed.nodes[n as usize].wings == key.wings)
            .map(|&n| d[n as usize])
            .filter(|&x| x != u32::MAX)
            .min()
    }
}

/// xorshift64: the explorer's one generator (no random seeding but `Options::seed`, DESIGN §3.1).
#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// How one frame ended.
enum Step {
    Go,
    End,
    Goal,
}

/// A play from an archived state, frame by frame: the checks every burst and the seed replay make.
struct Walk<'a> {
    facts: &'a Facts,
    opt: &'a Options,
    lives: u16,
    via: u8,
    /// Frames from the explorer's start.
    frames: u32,
    tail_frames: u64,
}

impl<'a> Walk<'a> {
    fn new(facts: &'a Facts, opt: &'a Options, g: &Game, via: u8, frames: u32) -> Walk<'a> {
        Walk {
            facts,
            opt,
            lives: g.state().w(FROG + f::LIVES),
            via,
            frames,
            tail_frames: 0,
        }
    }

    fn step(&mut self, g: &mut Game, joy: u8) -> Step {
        let (facts, opt) = (self.facts, self.opt);
        g.step_unseen(FrameInput::joy(Joy(joy)));
        self.frames += 1;
        let st = g.state();
        // A 1-up gained is not a loss (rust-engine FINDINGS T205): compare with the last frame.
        let lives = st.w(FROG + f::LIVES);
        if (lives as i16) < self.lives as i16 || st.w(FROG + f::DYING) != 0 {
            return Step::End;
        }
        self.lives = lives;
        let cell = frog_cell(st);
        if opt.avoid.iter().any(|b| in_box(b, cell)) {
            return Step::End;
        }
        if let Some(&v) = opt.via.get(self.via as usize)
            && v.0.abs_diff(cell.0) <= 1
            && v.1.abs_diff(cell.1) <= 1
        {
            self.via += 1;
        }
        let via_done = self.via as usize == opt.via.len();
        if let Goal::Key { key, within_frames } = opt.goal {
            if via_done && self.frames <= within_frames && cell_key(facts, st, self.via) == key {
                return Step::Goal;
            }
            if self.frames >= within_frames {
                return Step::End;
            }
        }
        if st.l(LEVEL_STATE) != PLAYING {
            return if opt.goal == Goal::Exit && via_done && self.survives(g) {
                Step::Goal
            } else {
                Step::End
            };
        }
        if self.frames >= opt.max_frames {
            return Step::End;
        }
        Step::Go
    }

    /// The frog lives through `EXIT_TAIL` idle frames after the level ends.
    fn survives(&mut self, g: &Game) -> bool {
        let mut t = g.clone();
        for _ in 0..EXIT_TAIL {
            t.step_unseen(FrameInput::NONE);
            self.tail_frames += 1;
            let st = t.state();
            if (st.w(FROG + f::LIVES) as i16) < self.lives as i16 || st.w(FROG + f::DYING) != 0 {
                return false;
            }
        }
        true
    }

    fn key(&self, st: &State) -> CellKey {
        cell_key(self.facts, st, self.via)
    }
}

fn in_box(&((c0, r0), (c1, r1)): &(Cell, Cell), (c, r): Cell) -> bool {
    (c0.min(c1)..=c0.max(c1)).contains(&c) && (r0.min(r1)..=r0.max(r1)).contains(&r)
}

pub struct Explorer {
    facts: Facts,
    opt: Options,
    archive: Archive,
    rng: Rng,
    compass: Option<MapCompass>,
    guide: Option<GuideIndex>,
    start_hearts: u16,
    rounds: u64,
    frames: u64,
    found: Option<Route>,
}

impl Explorer {
    pub fn new(start: Game, facts: Facts, mut opt: Options) -> Explorer {
        let compass = match std::mem::take(&mut opt.compass) {
            Compass::Straight => None,
            Compass::Map(m) => Some(MapCompass::new(m, &facts)),
        };
        let guide = (!opt.guide.is_empty()).then(|| GuideIndex::new(&opt.guide));
        let key = cell_key(&facts, start.state(), 0);
        let archive = Archive::new(&start, key, opt.max_mem_bytes);
        let seed = opt.seed;
        let mut ex = Explorer {
            start_hearts: start.state().w(HEARTS),
            facts,
            opt,
            archive,
            rng: Rng::new(seed),
            compass,
            guide,
            rounds: 0,
            frames: 0,
            found: None,
        };
        let d = ex.dist(&key);
        ex.archive.entries_mut()[0].dist = d;
        if let Some(seed) = ex.opt.seed_inputs.take() {
            ex.seed_from(&start, &seed);
        }
        ex
    }

    /// `seed_inputs` (DESIGN §2.7): replayed from the start, every 8th state offered, until the
    /// play ends; an exit on the way is the route at once.
    fn seed_from(&mut self, start: &Game, inputs: &[FrameInput]) {
        let mut g = start.clone();
        let (mut base, mut from) = (0u32, 0usize);
        let mut since = Vec::new();
        let mut kept = Vec::new();
        let mut exit = None;
        let mut w = Walk::new(&self.facts, &self.opt, &g, 0, 0);
        for (k, x) in inputs.iter().enumerate() {
            since.push(x.joy.0);
            match w.step(&mut g, x.joy.0) {
                Step::End => break,
                Step::Goal => {
                    exit = Some((k + 1, g.state().w(HEARTS)));
                    break;
                }
                Step::Go => {}
            }
            if (k + 1) % SEED_EVERY != 0 {
                continue;
            }
            // Each offer is from the last checkpoint the seed took, so suffixes stay short.
            let st = g.state();
            let (key, hearts) = (w.key(st), st.w(HEARTS));
            let a = &mut self.archive;
            let want = a.improves(&key, hearts, w.frames) && a.wants_state(k + 1 - from);
            let o = Offer {
                key,
                hearts,
                coins_left: st.w(COINS_LEFT),
                base,
                inputs: &since[from..],
                game: want.then(|| g.clone()),
            };
            let (i, cp) = a.offer(o, 0);
            kept.extend(i);
            if let Some(cp) = cp {
                (base, from) = (cp, k + 1);
            }
        }
        self.frames += since.len() as u64 + w.tail_frames;
        for i in kept {
            self.set_dist(i);
        }
        if let Some((len, hearts)) = exit {
            self.found = Some(self.route(since[..len].to_vec(), hearts));
        }
        self.archive.evict();
    }

    fn route(&self, inputs: Vec<u8>, hearts: u16) -> Route {
        Route {
            inputs: inputs
                .into_iter()
                .map(|j| FrameInput::joy(Joy(j)))
                .collect(),
            engine_frames: self.frames,
            hearts_lost: self.start_hearts.saturating_sub(hearts).min(u8::MAX as u16) as u8,
            rounds: self.rounds,
            cells: self.archive.entries().len() as u32,
        }
    }

    /// The goal found before any round (by `seed_inputs`).
    pub fn found(&self) -> Option<&Route> {
        self.found.as_ref()
    }

    pub fn archive(&self) -> &Archive {
        &self.archive
    }

    /// The archive, for tests that drop its states.
    pub fn archive_mut(&mut self) -> &mut Archive {
        &mut self.archive
    }

    pub fn rounds(&self) -> u64 {
        self.rounds
    }

    pub fn engine_frames(&self) -> u64 {
        self.frames
    }

    fn straight(&self, key: &CellKey, to: &[Cell]) -> f64 {
        let (c, r) = ((key.col * 2) as f64, (key.row * 2) as f64);
        to.iter()
            .map(|x| (c - x.0 as f64).abs() + (r - x.1 as f64).abs())
            .fold(f64::MAX, f64::min)
    }

    /// `key`'s distance in cells to what it heads for: the next `via` cell, else the goal.
    fn dist(&mut self, key: &CellKey) -> f64 {
        if let Some(&v) = self.opt.via.get(key.via as usize) {
            return self.straight(key, &[v]);
        }
        match self.opt.goal {
            Goal::Key { key: k, .. } => self.straight(key, &[(k.col * 2, k.row * 2)]),
            Goal::Exit => match self.compass.as_mut().and_then(|m| m.frames(key)) {
                Some(fr) => fr as f64 / FRAMES_PER_CELL,
                None => self.straight(key, &self.facts.exits),
            },
        }
    }

    fn set_dist(&mut self, i: u32) {
        let key = self.archive.entries()[i as usize].key;
        let d = self.dist(&key);
        self.archive.entries_mut()[i as usize].dist = d;
    }

    /// Each entry's choice weight (DESIGN §2.4).
    fn weights(&self) -> Vec<f64> {
        let es = self.archive.entries();
        let via_n = self.opt.via.len() as u8;
        let via_max = es.iter().map(|e| e.key.via).max().unwrap_or(0);
        let min_band = es
            .iter()
            .filter(|e| e.key.via == via_n)
            .map(|e| e.key.band)
            .min()
            .unwrap_or(0);
        let p_max = self.guide.as_ref().map_or(0, |g| {
            es.iter().filter_map(|e| g.index(&e.key)).max().unwrap_or(0)
        });
        es.iter()
            .map(|e| {
                let k = &e.key;
                let novelty =
                    1.0 / (1.0 + e.chosen as f64).sqrt() + 0.5 / (1.0 + e.reached as f64).sqrt();
                let done = k.via == via_n;
                let behind = (via_max - k.via) as f64
                    + if done {
                        (k.band - min_band) as f64
                    } else {
                        0.0
                    };
                let toward = match self.opt.goal {
                    _ if !done => pull(e.dist),
                    Goal::Exit if k.band == 0 => pull(e.dist),
                    Goal::Key { key, .. } if k.band == key.band => pull(e.dist),
                    _ => 1.0,
                };
                let guide = self.guide.as_ref().map_or(1.0, |g| g.weight(k, p_max));
                let hearts = 0.25f64.powi(self.start_hearts.saturating_sub(e.hearts) as i32);
                novelty * (-0.8 * behind).exp() * toward * guide * hearts
            })
            .collect()
    }

    /// One round's bursts, chosen now and split into `n` parts for the shell's threads.
    pub fn round_parts(&mut self, n: usize) -> Vec<Part> {
        let w = self.weights();
        let mut cum = Vec::with_capacity(w.len());
        let mut total = 0.0;
        for x in &w {
            total += x;
            cum.push(total);
        }
        let mut picks = Vec::new();
        for p in 0..self.opt.bursts_per_round {
            let x = self.rng.unit() * total;
            let i = cum.partition_point(|&c| c <= x).min(cum.len() - 1);
            picks.push((p, i as u32, self.rng.next()));
        }
        let round = self.rounds + 1;
        for &(_, i, _) in &picks {
            let e = &mut self.archive.entries_mut()[i as usize];
            e.chosen += 1;
            let cp = e.checkpoint;
            self.archive.touch(cp, round);
        }
        // An entry's picks go to one part, so its state is rebuilt once; the parts' sizes are
        // evened greedily. Results are merged in choice order whatever the split.
        let mut groups: Vec<Vec<Pick>> = Vec::new();
        let mut at: HashMap<u32, usize> = HashMap::new();
        for p in picks {
            let g = *at.entry(p.1).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[g].push(p);
        }
        let mut parts = vec![Part::default(); n.max(1)];
        for g in groups {
            let k = (0..parts.len())
                .min_by_key(|&k| (parts[k].picks.len(), k))
                .unwrap();
            parts[k].picks.extend(g);
        }
        parts.retain(|p| !p.is_empty());
        parts
    }

    /// Plays a part's bursts. Pure: reads the archive as the round started; any thread.
    pub fn run_part(&self, part: &Part) -> PartResult {
        let mut out = PartResult {
            bursts: Vec::new(),
            rebuilt: Vec::new(),
            frames: 0,
        };
        let mut k = 0;
        while k < part.picks.len() {
            let i = part.picks[k].1;
            let g = self
                .archive
                .entry_state(i, &mut out.frames, &mut out.rebuilt);
            while k < part.picks.len() && part.picks[k].1 == i {
                let (pick, _, seed) = part.picks[k];
                out.bursts
                    .push(self.burst(pick, i, &g, seed, &mut out.frames));
                k += 1;
            }
        }
        out
    }

    /// One burst from entry `i`'s state `g` (DESIGN §2.4): 1–16 holds, each 4–32 frames of one
    /// joystick state, kept from the last hold with probability 0.6; the state after each hold is
    /// offered. A caught engine panic ends it.
    fn burst(&self, pick: u32, i: u32, g0: &Game, seed: u64, frames: &mut u64) -> Burst {
        let e = &self.archive.entries()[i as usize];
        let mut b = Burst {
            pick,
            base: e.checkpoint,
            prefix: e.suffix.clone(),
            inputs: Vec::new(),
            held: Vec::new(),
            exit: None,
        };
        let mut rng = Rng::new(seed);
        let mut g = g0.clone();
        let mut w = Walk::new(&self.facts, &self.opt, &g, e.key.via, e.frames);
        let played = catch_unwind(AssertUnwindSafe(|| {
            let holds = 1 + rng.below(MAX_HOLDS);
            let mut a = ACTS[rng.below(ACTS.len())];
            for _ in 0..holds {
                if rng.unit() > KEEP {
                    a = ACTS[rng.below(ACTS.len())];
                }
                let n = HOLD_FRAMES[rng.below(HOLD_FRAMES.len())];
                for _ in 0..n {
                    b.inputs.push(a.0);
                    match w.step(&mut g, a.0) {
                        Step::Go => {}
                        Step::End => return,
                        Step::Goal => {
                            b.exit = Some((b.inputs.len(), g.state().w(HEARTS)));
                            return;
                        }
                    }
                }
                let st = g.state();
                let key = w.key(st);
                let hearts = st.w(HEARTS);
                let len = b.prefix.len() + b.inputs.len();
                let frames = w.frames;
                let game = (self.archive.improves(&key, hearts, frames)
                    && self.archive.wants_state(len))
                .then(|| g.clone());
                b.held.push(Held {
                    len: b.inputs.len(),
                    key,
                    hearts,
                    coins_left: st.w(COINS_LEFT),
                    game,
                });
            }
        }));
        // A panic mid-hold leaves `b` with the holds before it: the burst ends there.
        drop(played);
        *frames += b.inputs.len() as u64 + w.tail_frames;
        b
    }

    /// Takes a round's results, in part order (any order works: bursts are taken in choice order).
    pub fn merge(&mut self, results: Vec<PartResult>) -> Progress {
        self.rounds += 1;
        let round = self.rounds;
        let mut bursts = Vec::new();
        let mut rebuilt = Vec::new();
        for r in results {
            self.frames += r.frames;
            bursts.extend(r.bursts);
            rebuilt.extend(r.rebuilt);
        }
        bursts.sort_by_key(|b| b.pick);
        let mut best: Option<(Vec<u8>, u16)> = None;
        for b in bursts {
            let mut from = b.prefix.clone();
            for h in b.held {
                from.truncate(b.prefix.len());
                from.extend_from_slice(&b.inputs[..h.len]);
                let o = Offer {
                    key: h.key,
                    hearts: h.hearts,
                    coins_left: h.coins_left,
                    base: b.base,
                    inputs: &from,
                    game: h.game,
                };
                if let (Some(i), _) = self.archive.offer(o, round) {
                    self.set_dist(i);
                }
            }
            if let Some((len, hearts)) = b.exit {
                let mut all = self.archive.checkpoint_inputs(b.base);
                all.extend_from_slice(&b.prefix);
                all.extend_from_slice(&b.inputs[..len]);
                if best.as_ref().is_none_or(|(x, _)| all.len() < x.len()) {
                    best = Some((all, hearts));
                }
            }
        }
        for (k, g) in rebuilt {
            self.archive.restore(k, g, round);
        }
        self.archive.evict();
        match best {
            Some((inputs, hearts)) => Progress::Exit(self.route(inputs, hearts)),
            None => Progress::Working,
        }
    }

    /// The furthest entry (DESIGN §2.9): the most `via` passed, the lowest band, then the nearest
    /// its goal, then the fewest frames; and why it is not a route.
    pub fn best_partial(&self) -> (Route, Reason) {
        let es = self.archive.entries();
        let (i, e) = es
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (std::cmp::Reverse(a.key.via), a.key.band)
                    .cmp(&(std::cmp::Reverse(b.key.via), b.key.band))
                    .then(a.dist.total_cmp(&b.dist))
                    .then(a.frames.cmp(&b.frames))
            })
            .expect("the start is archived");
        let met = es
            .iter()
            .filter(|e| e.key.band == 0 && e.key.via as usize == self.opt.via.len())
            .map(|e| e.dist)
            .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))));
        let route = self.route(self.archive.entry_inputs(i as u32), e.hearts);
        let reason = Reason {
            coins_left: e.coins_left,
            exit_distance_with_coins_met: met.map(|d| d.round().min(u16::MAX as f64) as u16),
            cells: es.len() as u32,
        };
        (route, reason)
    }
}
