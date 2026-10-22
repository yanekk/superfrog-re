//! Route shortening (DESIGN §2.12, D23): the shortening budget and the shortcut pass.
//!
//! After the first exit the shell keeps searching for `budget` more rounds and keeps the shortest
//! exit (`better`); then the shortcut pass: for states i < j on the route, an explorer run from i
//! targets j's key within (j − i) − 1 frames, and a hit is spliced in if the whole route still
//! replays to the exit with no more hearts lost. All of it counted in rounds and frames, never
//! timed, so the shortened route is the same for any thread count (M8).

use std::collections::HashSet;

use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_core::layout::h0;
use sf_core::{Game, State};

use super::archive::{CellKey, HEARTS, cell_key};
use super::explore::{Goal, Route};
use super::facts::Facts;

/// `level_state` (h0+$a06): −2 while the level is played (as `explore`).
const LEVEL_STATE: u32 = h0(0xa06);
const PLAYING: u32 = 0xffff_fffe;
/// The idle frames the frog must survive after the level ends (as `explore`'s exit).
const EXIT_TAIL: u32 = 40;
/// The route's states are sampled this often (DESIGN §2.12's i and j).
pub const SAMPLE: usize = 8;
/// A full state is kept this often along the route, so a pair's start replays at most this many
/// frames (a state is ~0.8 MB; one every 8 frames of a 10 000-frame route would be 1 GB).
const CHECKPOINT_EVERY: usize = 128;
/// The longest stretch a pair spans: a short explorer run rarely finds a target further away.
pub const MAX_SPAN: usize = 640;
/// A pair is tried only if it could save this many frames: (j − i) less the straight distance
/// between the two keys at `FRAMES_PER_KEY` a key (a key is 2×2 cells, the frog runs a cell in ~8
/// frames).
pub const MIN_SAVE: usize = 24;
const FRAMES_PER_KEY: usize = 16;
/// The rounds each pair's explorer run may take.
pub const PAIR_ROUNDS: u64 = 8;

/// The rounds of search after the first exit: `share` of the rounds used to it, at least `min`
/// (DESIGN §2.12). With both 0 there is no shortening.
pub fn budget(rounds_to_exit: u64, share: f32, min: u64) -> u64 {
    ((rounds_to_exit as f64 * share as f64).round() as u64).max(min)
}

/// Whether exit `new` replaces `best`: shorter, and no more hearts lost.
pub fn better(new: &Route, best: &Route) -> bool {
    new.hearts_lost <= best.hearts_lost && new.inputs.len() < best.inputs.len()
}

/// Replays `inputs` from `start` to the exit, as the explorer judges one: no life lost and not
/// dying on any frame, the level ending, and the frog alive through the 40-frame idle tail. The
/// frames to the level's end and the hearts lost, or `None`. `frames` counts what was simulated.
pub fn replay_exit(start: &Game, inputs: &[FrameInput], frames: &mut u64) -> Option<(usize, u8)> {
    exit_from(start.clone(), start.state().w(HEARTS), inputs, frames)
}

/// `replay_exit` from `g`, a state on the way, the hearts counted from `hearts0`; the frames to
/// the level's end are counted from `g`.
fn exit_from(
    mut g: Game,
    hearts0: u16,
    inputs: &[FrameInput],
    frames: &mut u64,
) -> Option<(usize, u8)> {
    let mut lives = g.state().w(FROG + f::LIVES);
    let dead = |st: &State, lives: u16| {
        (st.w(FROG + f::LIVES) as i16) < lives as i16 || st.w(FROG + f::DYING) != 0
    };
    for (k, x) in inputs.iter().enumerate() {
        g.step_unseen(FrameInput::joy(x.joy));
        *frames += 1;
        let st = g.state();
        if dead(st, lives) {
            return None;
        }
        // A 1-up gained is not a loss (rust-engine FINDINGS T205).
        lives = st.w(FROG + f::LIVES);
        if st.l(LEVEL_STATE) != PLAYING {
            let hearts = st.w(HEARTS);
            for _ in 0..EXIT_TAIL {
                g.step_unseen(FrameInput::NONE);
                *frames += 1;
                if dead(g.state(), lives) {
                    return None;
                }
            }
            return Some((
                k + 1,
                hearts0.saturating_sub(hearts).min(u8::MAX as u16) as u8,
            ));
        }
    }
    None
}

/// The shortcut pass's bookkeeping (DESIGN §2.12): which pair of route states to try next, and
/// whether a stretch found replaces the old one. The shell runs the explorer for each pair.
pub struct Shortcut {
    start: Game,
    facts: Facts,
    route: Vec<FrameInput>,
    hearts_lost: u8,
    rounds_left: u64,
    /// The state after k × `CHECKPOINT_EVERY` frames.
    checkpoints: Vec<Game>,
    /// The key after k × `SAMPLE` frames.
    keys: Vec<CellKey>,
    /// This pass's pairs (frames i, j), best first, taken from the back.
    queue: Vec<(usize, usize)>,
    pending: Option<(usize, usize)>,
    /// Pairs found wanting on the route as it now stands.
    failed: HashSet<(usize, usize)>,
    tried: u64,
    accepted: u64,
    frames: u64,
}

impl Shortcut {
    /// The pass over `route`, an exit from `start`, with `rounds` of explorer to spend. A route
    /// that does not replay to the exit gives no pairs.
    pub fn new(start: &Game, facts: &Facts, route: Vec<FrameInput>, rounds: u64) -> Shortcut {
        let mut s = Shortcut {
            start: start.clone(),
            facts: facts.clone(),
            route,
            hearts_lost: 0,
            rounds_left: rounds,
            checkpoints: Vec::new(),
            keys: Vec::new(),
            queue: Vec::new(),
            pending: None,
            failed: HashSet::new(),
            tried: 0,
            accepted: 0,
            frames: 0,
        };
        match replay_exit(&s.start, &s.route, &mut s.frames) {
            Some((len, lost)) => {
                s.route.truncate(len);
                s.hearts_lost = lost;
                s.pass();
            }
            None => s.rounds_left = 0,
        }
        s
    }

    /// Samples the route as it now stands and orders this pass's pairs: for each i the j with the
    /// most frames to save, the pairs by that saving, most first (then the earlier i).
    fn pass(&mut self) {
        let mut g = self.start.clone();
        self.checkpoints.clear();
        self.keys.clear();
        for k in 0..self.route.len() {
            if k % CHECKPOINT_EVERY == 0 {
                self.checkpoints.push(g.clone());
            }
            if k % SAMPLE == 0 {
                self.keys.push(cell_key(&self.facts, g.state(), 0));
            }
            g.step_unseen(FrameInput::joy(self.route[k].joy));
            self.frames += 1;
        }
        let n = self.keys.len();
        // Every pair that could save `MIN_SAVE`, those taking no coin first (a stretch that must
        // take the same coins as the route rarely leaves the frog where the rest of the route
        // expects it), then by the frames to save, then the earlier i.
        let mut all = Vec::new();
        for i in 0..n {
            for j in i + 1..n.min(i + MAX_SPAN / SAMPLE + 1) {
                let (a, b) = (&self.keys[i], &self.keys[j]);
                let dist = (a.col.abs_diff(b.col) + a.row.abs_diff(b.row)) as usize;
                let save = ((j - i) * SAMPLE).saturating_sub(dist * FRAMES_PER_KEY);
                let pair = (i * SAMPLE, j * SAMPLE);
                if save >= MIN_SAVE && !self.failed.contains(&pair) {
                    all.push((a.band == b.band, save, pair));
                }
            }
        }
        all.sort_by_key(|&(same, save, (i, j))| (std::cmp::Reverse((same, save)), i, j));
        // Then greedily one pair per i and per j: pairs i, i + 8, … to one j are mostly one
        // stretch, and when one fails to splice the others do.
        let (mut used_i, mut used_j) = (HashSet::new(), HashSet::new());
        let mut pairs = Vec::new();
        for (_, _, (i, j)) in all {
            if !used_i.contains(&i) && !used_j.contains(&j) {
                used_i.insert(i);
                used_j.insert(j);
                pairs.push((i, j));
            }
        }
        // Best last, so `pop` takes it.
        pairs.reverse();
        self.queue = pairs;
    }

    /// The next pair: the route's state at i and the goal j (its key, within (j − i) − 1 frames).
    /// `None` once the rounds are spent or a pass has found nothing.
    pub fn next(&mut self) -> Option<(Game, Goal)> {
        if let Some(p) = self.pending.take() {
            // Not offered: counted as nothing found.
            self.failed.insert(p);
        }
        if self.rounds_left == 0 {
            return None;
        }
        let (i, j) = self.queue.pop()?;
        let mut g = self.checkpoints[i / CHECKPOINT_EVERY].clone();
        for x in &self.route[i / CHECKPOINT_EVERY * CHECKPOINT_EVERY..i] {
            g.step_unseen(FrameInput::joy(x.joy));
            self.frames += 1;
        }
        self.pending = Some((i, j));
        self.tried += 1;
        let goal = Goal::Key {
            key: self.keys[j / SAMPLE],
            within_frames: (j - i - 1) as u32,
        };
        Some((g, goal))
    }

    /// The rounds the next pair's explorer may take.
    pub fn pair_rounds(&self) -> u64 {
        PAIR_ROUNDS.min(self.rounds_left)
    }

    /// Rounds the shell spent on a pair's explorer run.
    pub fn spend(&mut self, rounds: u64) {
        self.rounds_left = self.rounds_left.saturating_sub(rounds);
    }

    /// The explorer's result for the pair `next` gave: the inputs from i to j's key, or `None`. A
    /// hit is spliced in and kept only if the whole route replays to the exit with no more hearts
    /// lost; then a new pass starts on the shorter route.
    pub fn offer(&mut self, found: Option<Vec<FrameInput>>) {
        let Some((i, j)) = self.pending.take() else {
            return;
        };
        if let Some(found) = found.filter(|x| x.len() < j - i) {
            let mut r = self.route[..i].to_vec();
            r.extend(found);
            r.extend_from_slice(&self.route[j..]);
            // The route to i is unchanged: replay from the checkpoint before it.
            let c = i / CHECKPOINT_EVERY;
            let from = c * CHECKPOINT_EVERY;
            let hearts0 = self.start.state().w(HEARTS);
            let g = self.checkpoints[c].clone();
            if let Some((len, lost)) = exit_from(g, hearts0, &r[from..], &mut self.frames)
                .map(|(len, lost)| (from + len, lost))
                && lost <= self.hearts_lost
                && len < self.route.len()
            {
                r.truncate(len);
                self.route = r;
                self.hearts_lost = lost;
                self.accepted += 1;
                // The route before i is unchanged, so are its states: its failures stand.
                self.failed.retain(|&(_, b)| b <= i);
                self.pass();
                return;
            }
        }
        self.failed.insert((i, j));
    }

    pub fn route(&self) -> &[FrameInput] {
        &self.route
    }

    pub fn hearts_lost(&self) -> u8 {
        self.hearts_lost
    }

    /// Pairs tried and stretches kept.
    pub fn counts(&self) -> (u64, u64) {
        (self.tried, self.accepted)
    }

    /// Engine frames the pass itself simulated (samples, pair starts, replays).
    pub fn engine_frames(&self) -> u64 {
        self.frames
    }
}
