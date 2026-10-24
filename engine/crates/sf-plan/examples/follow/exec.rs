//! The executors: the fixed script (baseline), A (position-triggered script), B1 (rule-based
//! follower) and the look-ahead followers (B2 random bursts, B3 the route's own inputs re-timed).

use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, State};
use sf_plan::core::map::pressed;
use sf_plan::core::{Facts, band};

use crate::machine::Machine;
use crate::play::{COINS_LEFT, HEARTS, LEVEL_STATE, Exec, Obs};
use crate::reference::RefFrame;

// ---------------------------------------------------------------------------------------------
// Baseline: the fixed script.

pub struct Fixed {
    pub inputs: Vec<FrameInput>,
}

impl Exec for Fixed {
    fn decide(&mut self, _m: &dyn Machine, _o: &Obs, t: u32) -> FrameInput {
        self.inputs.get(t as usize).copied().unwrap_or(FrameInput::NONE)
    }
}

// ---------------------------------------------------------------------------------------------
// A: a position-triggered script. Cut from the reference where the input changes; each step is
// "when the frog matches this trigger, hold these inputs until the next trigger".

#[derive(Debug, Clone)]
pub struct Step {
    pub input: FrameInput,
    /// The frog when the reference began this step.
    pub at: Obs,
    /// The reference's frames in this step.
    pub len: u32,
    /// The frog stood still through the reference's step (a wait): its end is timed, not placed.
    pub wait: bool,
}

pub fn cut_steps(r: &[RefFrame]) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for (k, rf) in r.iter().enumerate() {
        if steps.last().is_none_or(|s| s.input != rf.input) {
            steps.push(Step { input: rf.input, at: rf.o, len: 0, wait: false });
        }
        steps.last_mut().unwrap().len += 1;
        let _ = k;
    }
    for i in 0..steps.len() {
        let next = steps.get(i + 1).map(|s| s.at);
        let a = steps[i].at;
        steps[i].wait = match next {
            Some(n) => (n.x - a.x).abs() <= 1 && (n.y - a.y).abs() <= 1,
            None => true,
        };
    }
    steps
}

pub struct Triggers {
    pub steps: Vec<Step>,
    i: usize,
    t_in: u32,
    prev_d: i32,
    /// Pure: every step's end is placed, waits included (no timed waits).
    pure: bool,
    pub fired_by_timeout: u32,
}

/// A trigger matches within this many pixels (|dx| + |dy|) at once.
const TIGHT: i32 = 2;
/// Within this, a trigger fires once the frog has passed its closest approach.
const LOOSE: i32 = 24;

impl Triggers {
    pub fn new(steps: Vec<Step>, pure: bool) -> Triggers {
        Triggers { steps, i: 0, t_in: 0, prev_d: i32::MAX, pure, fired_by_timeout: 0 }
    }
    fn dist(a: &Obs, b: &Obs) -> i32 {
        (a.x as i32 - b.x as i32).abs() + (a.y as i32 - b.y as i32).abs()
    }
    fn advance(&mut self) {
        self.i += 1;
        self.t_in = 0;
        self.prev_d = i32::MAX;
    }
}

impl Exec for Triggers {
    fn decide(&mut self, _m: &dyn Machine, o: &Obs, _t: u32) -> FrameInput {
        if self.i + 1 < self.steps.len() {
            let cur = &self.steps[self.i];
            let next = &self.steps[self.i + 1];
            let fire = if cur.wait && !self.pure {
                self.t_in >= cur.len
            } else {
                // Coins: the frog has at least the reference's coins there (none left to take).
                let coins_ok = o.coins <= next.at.coins && o.switches == next.at.switches;
                let d = Self::dist(o, &next.at);
                let f = coins_ok && (d <= TIGHT || (d <= LOOSE && self.prev_d <= LOOSE && d > self.prev_d));
                self.prev_d = if coins_ok { d } else { i32::MAX };
                f
            };
            if fire {
                self.advance();
            } else if self.t_in > 2 * cur.len + 25 {
                self.fired_by_timeout += 1;
                self.advance();
            }
        }
        self.t_in += 1;
        self.steps.get(self.i).map_or(FrameInput::NONE, |s| s.input)
    }

    fn respawned(&mut self, o: &Obs) {
        // The step whose trigger is nearest the frog with the same coins left.
        let best = (0..self.steps.len()).min_by_key(|&j| {
            let s = &self.steps[j].at;
            (s.coins as i32 - o.coins as i32).abs() * 1000 + Self::dist(o, s)
        });
        self.i = best.unwrap_or(0);
        self.t_in = 0;
        self.prev_d = i32::MAX;
    }
}

// ---------------------------------------------------------------------------------------------
// Progress along B's guide (shared by B1, B2, B3).

/// Guide points: (x, y, coins left, switches), every 8th reference frame while playing.
pub type Guide = Vec<(i16, i16, u16, u32)>;

/// A point is passed when the frog's cell is within this many cells of the point's.
const NEAR_CELLS: i32 = 2;

fn near(g: &(i16, i16, u16, u32), x: i16, y: i16, coins: u16, sw: u32) -> bool {
    let (gc, gr) = crate::reference::cell_of(g.0, g.1);
    let (c, r) = crate::reference::cell_of(x, y);
    (gc as i32 - c as i32).abs() <= NEAR_CELLS
        && (gr as i32 - r as i32).abs() <= NEAR_CELLS
        && band(coins) <= band(g.2)
        && sw == g.3
}

/// The furthest point in `from..from+window` the frog is near, if any.
fn furthest(guide: &Guide, from: usize, window: usize, x: i16, y: i16, coins: u16, sw: u32) -> Option<usize> {
    (from..guide.len().min(from + window)).rev().find(|&j| near(&guide[j], x, y, coins, sw))
}

/// After a respawn: the point nearest the frog with its coins band.
fn relocate(guide: &Guide, o: &Obs) -> usize {
    (0..guide.len())
        .min_by_key(|&j| {
            let g = &guide[j];
            let bands = (band(g.2) as i32 - band(o.coins) as i32).abs();
            let sw = (g.3 != o.switches) as i32;
            bands * 100_000 + sw * 50_000 + (g.0 as i32 - o.x as i32).abs() + (g.1 as i32 - o.y as i32).abs()
        })
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// B1: rule-based. Steer toward a point a little ahead; jump when it is above or a wall is ahead.

pub struct Rules {
    pub guide: Guide,
    cur: usize,
}

const B1_LOOK: usize = 3;
const B1_WINDOW: usize = 12;

impl Rules {
    pub fn new(guide: Guide) -> Rules {
        Rules { guide, cur: 0 }
    }
}

impl Exec for Rules {
    fn decide(&mut self, m: &dyn Machine, o: &Obs, _t: u32) -> FrameInput {
        if let Some(j) = furthest(&self.guide, self.cur, B1_WINDOW, o.x, o.y, o.coins, o.switches) {
            self.cur = j;
        }
        let Some(&(tx, ty, _, _)) = self.guide.get((self.cur + B1_LOOK).min(self.guide.len().saturating_sub(1))) else {
            return FrameInput::NONE;
        };
        let st = m.state();
        let (dx, dy) = (tx as i32 - o.x as i32, ty as i32 - o.y as i32);
        let mut j = 0u8;
        if dx > 6 {
            j |= Joy::RIGHT.0;
        } else if dx < -6 {
            j |= Joy::LEFT.0;
        }
        let wall = (dx > 0 && st.w(FROG + f::WALL_RIGHT) != 0) || (dx < 0 && st.w(FROG + f::WALL_LEFT) != 0);
        let rising = (st.w(FROG + f::VY) as i16) < 0;
        if (o.landed && (dy < -20 || wall)) || (!o.landed && rising && dy < -8) {
            j |= Joy::UP.0;
        }
        FrameInput { joy: Joy(j), ..FrameInput::NONE }
    }
    fn respawned(&mut self, o: &Obs) {
        self.cur = relocate(&self.guide, o);
    }
}

// ---------------------------------------------------------------------------------------------
// B2/B3: short look-ahead. Every `k` frames, copy the live state into an engine `Game`, play a
// handful of `h`-frame candidates, commit the best one's first `k` frames. Lazy: the current plan
// (B3: the route's inputs at the current alignment) is tried first and kept when it stays on the
// path; only then are the others tried.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// B2: random sticky bursts (the explorer's joystick states and holds).
    Bursts,
    /// B3: the route's own inputs, waiting, skipping or rewinding a few frames.
    Shifts,
    /// Both candidate sets.
    Both,
}

pub struct Mpc {
    pub kind: Kind,
    pub facts: Facts,
    pub guide: Guide,
    /// The reference frame each guide point was sampled at.
    pub guide_frame: Vec<u32>,
    pub ref_inputs: Vec<FrameInput>,
    ref_landed: Vec<bool>,
    pub k: u32,
    pub h: u32,
    pub bursts: u32,
    pub threads: usize,
    pub lazy: bool,
    cur: usize,
    /// The reference frame the plan's next input was taken from (B3).
    align: i64,
    plan: Vec<(FrameInput, i64)>,
    pos: usize,
    since: u32,
    rng: u64,
    frames: u64,
    pub decisions: u32,
    pub replans: u32,
    /// Decisions since `cur` last moved; past `STUCK` the look-ahead doubles.
    stalled: u32,
    last_cur: usize,
    last_obs: Option<Obs>,
}

const STUCK: u32 = 6;

const ACTS: [u8; 10] = [0, 8, 4, 1, 9, 5, 2, 16, 24, 20];
const HOLDS: [u32; 3] = [4, 8, 16];

impl Mpc {
    pub fn new(kind: Kind, facts: Facts, r: &[RefFrame], seed: u64) -> Mpc {
        let mut guide = Vec::new();
        let mut guide_frame = Vec::new();
        for (k, rf) in r.iter().enumerate().step_by(8) {
            if rf.o.playing() {
                guide.push((rf.o.x, rf.o.y, rf.o.coins, rf.o.switches));
                guide_frame.push(k as u32);
            }
        }
        Mpc {
            kind,
            facts,
            guide,
            guide_frame,
            ref_inputs: r.iter().map(|f| f.input).collect(),
            ref_landed: r.iter().map(|f| f.o.landed).collect(),
            k: 8,
            h: 64,
            bursts: 12,
            threads: 2,
            lazy: true,
            cur: 0,
            align: 0,
            plan: Vec::new(),
            pos: 0,
            since: u32::MAX,
            rng: seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
            frames: 0,
            decisions: 0,
            replans: 0,
            stalled: 0,
            last_cur: 0,
            last_obs: None,
        }
    }

    fn window(&self) -> usize {
        (self.h / 8) as usize + 8
    }

    fn next_rand(&mut self) -> u64 {
        // splitmix64
        self.rng = self.rng.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn ref_at(&self, k: i64) -> FrameInput {
        if k < 0 {
            return FrameInput::NONE;
        }
        self.ref_inputs.get(k as usize).copied().unwrap_or(FrameInput::NONE)
    }

    /// The route's inputs from reference frame `from`, `h` long, each with its reference frame.
    fn ref_seq(&self, from: i64, wait: u32) -> Vec<(FrameInput, i64)> {
        let mut v: Vec<(FrameInput, i64)> = (0..wait).map(|_| (FrameInput::NONE, from)).collect();
        let mut k = from;
        while v.len() < self.h as usize {
            v.push((self.ref_at(k), k + 1));
            k += 1;
        }
        v
    }

    fn candidates(&mut self) -> Vec<Vec<(FrameInput, i64)>> {
        let mut c = Vec::new();
        // The current plan first: the rest of the last choice, extended (B2: its last input held;
        // B3: the route's inputs on from where it had got to).
        let rest: Vec<(FrameInput, i64)> = self.plan[self.pos.min(self.plan.len())..].to_vec();
        let warm = if self.kind == Kind::Bursts {
            let mut w = rest;
            let last = w.last().copied().unwrap_or((FrameInput::NONE, 0));
            w.resize(self.h as usize, last);
            w
        } else {
            self.ref_seq(self.align, 0)
        };
        c.push(warm);
        if matches!(self.kind, Kind::Shifts | Kind::Both) {
            for w in [2u32, 4, 8, 16, 32] {
                c.push(self.ref_seq(self.align, w));
            }
            for d in [2i64, 4, 8, 16, -4, -8, -16] {
                c.push(self.ref_seq(self.align + d, 0));
            }
            // Re-anchored: the route from where the frog is on the path, not where the clock
            // says it should be.
            if let Some(&gf) = self.guide_frame.get(self.cur) {
                for d in [0i64, 8, 16, 32, -8] {
                    c.push(self.ref_seq(gf as i64 + d, 0));
                }
            }
            // Re-anchored by place and coins: where the route was here with as many coins left
            // (a coin missed: the route's own way to it).
            if let Some(o) = self.last_obs {
                let j = (0..self.guide.len())
                    .min_by_key(|&j| {
                        let g = &self.guide[j];
                        (g.2 as i32 - o.coins as i32).abs() * 1000
                            + (g.3 != o.switches) as i32 * 500
                            + (g.0 as i32 - o.x as i32).abs()
                            + (g.1 as i32 - o.y as i32).abs()
                    })
                    .unwrap_or(0);
                if let Some(&gf) = self.guide_frame.get(j) {
                    // From the last frame before it the route's frog stood on ground: a jump
                    // replayed from its take-off, not from mid-air.
                    let lf = (0..=gf as usize).rev().find(|&k| self.ref_landed[k]).unwrap_or(0) as i64;
                    for a in [lf, lf - 8, lf - 16, gf as i64 - 8, gf as i64] {
                        c.push(self.ref_seq(a, 0));
                    }
                }
            }
        }
        if matches!(self.kind, Kind::Bursts | Kind::Both) {
            for _ in 0..self.bursts {
                let mut v = Vec::with_capacity(self.h as usize);
                let mut act = ACTS[(self.next_rand() % 10) as usize];
                while v.len() < self.h as usize {
                    if self.next_rand() % 10 >= 6 {
                        act = ACTS[(self.next_rand() % 10) as usize];
                    }
                    let n = HOLDS[(self.next_rand() % 3) as usize];
                    for _ in 0..n {
                        v.push((FrameInput { joy: Joy(act), ..FrameInput::NONE }, self.align));
                    }
                }
                v.truncate(self.h as usize);
                c.push(v);
            }
        }
        c
    }

    fn decide_now(&mut self, m: &dyn Machine, o: &Obs) {
        self.decisions += 1;
        self.last_obs = Some(*o);
        if self.cur > self.last_cur {
            (self.stalled, self.last_cur) = (0, self.cur);
        } else {
            self.stalled += 1;
        }
        let base_h = self.h;
        if self.stalled > STUCK {
            self.h = base_h * 2;
        }
        self.decide_inner(m, o);
        self.h = base_h;
    }

    fn decide_inner(&mut self, m: &dyn Machine, o: &Obs) {
        let g = m.game();
        let cands = self.candidates();
        let ctx = Ctx { facts: &self.facts, guide: &self.guide, cur: self.cur, window: self.window(), hearts: o.hearts, lives: o.lives };
        let want = self.cur + ((self.h / 8) as usize) / 2;
        let first = score(&g, &cands[0], &ctx);
        self.frames += self.h as u64;
        let pick = if self.lazy && first.on_track(want) {
            0
        } else {
            self.replans += 1;
            let rest = &cands[1..];
            let scores = eval_all(&g, rest, &ctx, self.threads);
            self.frames += rest.len() as u64 * self.h as u64;
            if std::env::var_os("FOLLOW_LOG2").is_some() {
                for (i, s) in scores.iter().enumerate() {
                    eprintln!("   cand {} {:?}", i + 1, s);
                }
            }
            let mut best = (first, 0usize);
            for (i, s) in scores.into_iter().enumerate() {
                if s.better(&best.0) {
                    best = (s, i + 1);
                }
            }
            best.1
        };
        if std::env::var_os("FOLLOW_LOG").is_some() {
            eprintln!(
                "dec {} cur {}/{} frog {},{} coins {} sw {} pick {pick} first {:?} align {}",
                self.decisions, self.cur, self.guide.len(), o.x, o.y, o.coins, o.switches, first, self.align
            );
        }
        self.plan = cands[pick].clone();
        self.pos = 0;
        self.since = 0;
    }
}

struct Ctx<'a> {
    facts: &'a Facts,
    guide: &'a Guide,
    cur: usize,
    window: usize,
    hearts: u16,
    lives: u16,
}

#[derive(Debug, Clone, Copy)]
struct Score {
    dead: bool,
    survived: u32,
    exit: Option<u32>,
    hearts_lost: u16,
    progress: usize,
    /// Coins left at the end (a coin missed off the route is worth going back for).
    coins: u16,
    /// Pixels from the next point after `progress`, at the end.
    gap: i32,
}

impl Score {
    fn key(&self) -> (i64, i64, i64, i64, i64) {
        if self.dead {
            return (0, self.survived as i64, 0, 0, 0);
        }
        if let Some(e) = self.exit {
            return (3, -(self.hearts_lost as i64), -(e as i64), 0, 0);
        }
        (
            1 + (self.hearts_lost == 0) as i64,
            self.progress as i64,
            -(self.coins as i64),
            -(self.hearts_lost as i64),
            -(self.gap as i64),
        )
    }
    fn better(&self, o: &Score) -> bool {
        self.key() > o.key()
    }
    fn on_track(&self, want: usize) -> bool {
        !self.dead && self.hearts_lost == 0 && (self.exit.is_some() || self.progress >= want)
    }
}

fn score(start: &Game, seq: &[(FrameInput, i64)], c: &Ctx) -> Score {
    let mut g = start.clone();
    let mut s = Score { dead: false, survived: 0, exit: None, hearts_lost: 0, progress: c.cur, coins: 0, gap: 0 };
    for (k, &(x, _)) in seq.iter().enumerate() {
        g.step_unseen(x);
        let st: &State = g.state();
        let ls = st.l(LEVEL_STATE);
        if st.w(FROG + f::DYING) != 0 || st.w(FROG + f::LIVES) < c.lives || ls == 0x190 {
            s.dead = true;
            s.survived = k as u32;
            return s;
        }
        if ls == 1 {
            s.exit = Some(k as u32);
            s.hearts_lost = c.hearts.saturating_sub(st.w(HEARTS));
            return s;
        }
        if k % 2 == 1 {
            let (x, y) = (st.w(FROG + f::X) as i16, st.w(FROG + f::Y) as i16);
            let sw = pressed(c.facts, st);
            if let Some(j) = furthest(c.guide, s.progress, c.window, x, y, st.w(COINS_LEFT), sw) {
                s.progress = s.progress.max(j);
            }
        }
    }
    let st = g.state();
    s.hearts_lost = c.hearts.saturating_sub(st.w(HEARTS));
    s.coins = st.w(COINS_LEFT);
    if let Some(n) = c.guide.get(s.progress + 1) {
        s.gap = (n.0 as i32 - st.w(FROG + f::X) as i16 as i32).abs() + (n.1 as i32 - st.w(FROG + f::Y) as i16 as i32).abs();
    }
    s
}

fn eval_all(g: &Game, cands: &[Vec<(FrameInput, i64)>], c: &Ctx, threads: usize) -> Vec<Score> {
    if threads <= 1 || cands.len() < 2 {
        return cands.iter().map(|s| score(g, s, c)).collect();
    }
    let chunk = cands.len().div_ceil(threads);
    std::thread::scope(|sc| {
        let hs: Vec<_> = cands
            .chunks(chunk)
            .map(|part| sc.spawn(move || part.iter().map(|s| score(g, s, c)).collect::<Vec<_>>()))
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

impl Exec for Mpc {
    fn decide(&mut self, m: &dyn Machine, o: &Obs, _t: u32) -> FrameInput {
        if let Some(j) = furthest(&self.guide, self.cur, self.window(), o.x, o.y, o.coins, o.switches) {
            self.cur = self.cur.max(j);
        }
        let due = self.since >= self.k || self.pos >= self.plan.len();
        if due && !m.mid_pass() && o.playing() {
            self.decide_now(m, o);
        }
        self.since = self.since.saturating_add(1);
        let (x, next) = match self.plan.get(self.pos) {
            Some(&p) => p,
            // Past the plan's end (the original mid-pass at a decision): B2 holds its last
            // input, B3 goes on with the route from where it had got to.
            None if self.kind == Kind::Bursts => (self.plan.last().map_or(FrameInput::NONE, |p| p.0), self.align),
            None => (self.ref_at(self.align), self.align + 1),
        };
        self.pos += 1;
        self.align = next;
        x
    }
    fn respawned(&mut self, o: &Obs) {
        self.cur = relocate(&self.guide, o);
        self.align = self.guide_frame.get(self.cur).copied().unwrap_or(0) as i64;
        self.plan.clear();
        self.pos = 0;
        self.since = u32::MAX;
    }
    fn engine_frames(&self) -> u64 {
        self.frames
    }
}
