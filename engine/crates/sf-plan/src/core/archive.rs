//! The explorer's archive (DESIGN §2.4, D22): one entry per cell key, holding the best state that
//! reached it (more hearts first, then fewer frames), the inputs to it, and its counters.
//!
//! States are kept for checkpoints only. A checkpoint is an immutable node of a tree rooted at the
//! explorer's start: its parent and the inputs from the parent to it. An entry's inputs are a
//! checkpoint and the suffix after it; an entry within `CHECKPOINT_MIN` frames of its checkpoint
//! keeps no state of its own and is rebuilt from it by replay. A checkpoint's state may be dropped
//! under the memory cap and rebuilt from its nearest ancestor that still has one. An entry replaced
//! by a better state takes a new checkpoint; the old one stays, so nothing reached through it breaks.
//!
//! The cap is the archive's own count of stored bytes (states at a size measured once from the
//! start, inputs at one byte a frame), never a reading of the OS, so which states are kept is a
//! function of seed and options only, and since a rebuilt state equals the stored one, the search
//! does not depend on the cap (M8).

use std::collections::HashMap;

use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, State};

use super::facts::{CoinKind, Facts, frog_cell, live};
use super::map::pressed;

/// `coins_left_for_exit` (h0+$832).
pub(crate) const COINS_LEFT: u32 = h0(0x832);
/// The hearts left (h0+$830).
pub(crate) const HEARTS: u32 = h0(0x830);

/// An entry this many frames or fewer past its checkpoint keeps no state: rebuilding it costs at
/// most that many frames each time it is chosen.
pub const CHECKPOINT_MIN: usize = 64;
/// A checkpoint at least this many frames past its nearest anchor ancestor is an anchor: anchors
/// lose their states only after every other checkpoint has, so a rebuild rarely replays further.
pub const ANCHOR_FRAMES: u32 = 1024;
/// What the archive counts for an entry's or a checkpoint's bookkeeping, beside its inputs.
const ENTRY_OVERHEAD: u64 = 96;
const CHECKPOINT_OVERHEAD: u64 = 96;

/// An archive cell (DESIGN §2.4): the frog's coarse cell, the coins band, the switches pressed and
/// the wings held; and the number of `via` hints passed (§2.10), 0 when there are none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellKey {
    /// Frog column / 2, row / 2.
    pub col: u16,
    pub row: u16,
    pub band: u16,
    /// `Facts::switches` indices pressed, as bits.
    pub switches: u32,
    pub wings: bool,
    pub via: u8,
}

/// The coins band: 0 with no coins left, each of the last 8 coins its own band, then 4 coins a
/// band (9 + (left − 9) / 4). Deviates from DESIGN §2.4's 1 + (left − 1) / 4 (owner, 2026-10-22):
/// with 4 coins a band, taking one of the last coins changed no key and nothing pulled toward the
/// rest, and W6-1 stalled 2–4 coins short (0 of 5 runs exited).
pub fn band(coins_left: u16) -> u16 {
    match coins_left {
        0 => 0,
        n @ 1..=8 => n,
        n => 9 + (n - 9) / 4,
    }
}

/// Straight distance in cells from the frog to the nearest object coin still in the map (its
/// live marker 8 or 9): the pull on the last 8 coins.
pub fn coin_dist(facts: &Facts, st: &State) -> f64 {
    let (c, r) = frog_cell(st);
    facts
        .coins
        .iter()
        .filter(|k| matches!(k.kind, CoinKind::Object(_)))
        .filter(|k| matches!(live(st, k.cell.0, k.cell.1), 8 | 9))
        .map(|k| (k.cell.0.abs_diff(c) + k.cell.1.abs_diff(r)) as f64)
        .fold(f64::MAX, f64::min)
}

/// `st`'s key, `via` hints passed.
pub fn cell_key(facts: &Facts, st: &State, via: u8) -> CellKey {
    let (c, r) = frog_cell(st);
    CellKey {
        col: c / 2,
        row: r / 2,
        band: band(st.w(COINS_LEFT)),
        switches: pressed(facts, st),
        wings: st.w(FROG + f::FLIGHT) != 0,
        via,
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub key: CellKey,
    pub hearts: u16,
    pub coins_left: u16,
    /// Frames from the explorer's start.
    pub frames: u32,
    pub checkpoint: u32,
    /// Joystick bytes after the checkpoint.
    pub suffix: Vec<u8>,
    pub chosen: u32,
    pub reached: u32,
    /// The compass distance to the goal, in cells (filled by the explorer).
    pub dist: f64,
    /// `coin_dist` of its state.
    pub coin_dist: f64,
}

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub parent: Option<u32>,
    /// Joystick bytes from the parent to here.
    pub suffix: Vec<u8>,
    /// Frames from the explorer's start.
    pub frames: u32,
    pub anchor: bool,
    state: Option<Game>,
    /// The last round this state was stored, rebuilt or chosen from (eviction order).
    last_used: u64,
}

/// What a state offered to the archive is and how it was reached: from `base` (a checkpoint) by
/// `inputs`.
pub struct Offer<'a> {
    pub key: CellKey,
    pub hearts: u16,
    pub coins_left: u16,
    pub coin_dist: f64,
    pub base: u32,
    pub inputs: &'a [u8],
    /// The state, when the archive may need it (`Archive::wants_state`).
    pub game: Option<Game>,
}

pub struct Archive {
    entries: Vec<Entry>,
    index: HashMap<CellKey, u32>,
    checkpoints: Vec<Checkpoint>,
    state_bytes: u64,
    cap: u64,
    stored_states: u64,
    input_bytes: u64,
}

/// A state's bytes as the archive counts them: its serialized size and an eighth, which is what a
/// clone holds on the heap (W5-2, measured 2026-10-22: 620 KB a clone, 565 KB serialized).
pub fn state_size(g: &Game) -> u64 {
    bincode::serialized_size(g).unwrap_or(1 << 20) * 9 / 8
}

impl Archive {
    /// An archive holding `start` as checkpoint 0 and its entry, under `cap` stored bytes.
    pub fn new(start: &Game, key: CellKey, cap: u64) -> Archive {
        let st = start.state();
        let mut a = Archive {
            entries: Vec::new(),
            index: HashMap::new(),
            checkpoints: vec![Checkpoint {
                parent: None,
                suffix: Vec::new(),
                frames: 0,
                anchor: true,
                state: Some(start.clone()),
                last_used: 0,
            }],
            state_bytes: state_size(start),
            cap,
            stored_states: 1,
            input_bytes: CHECKPOINT_OVERHEAD,
        };
        a.index.insert(key, 0);
        a.entries.push(Entry {
            key,
            hearts: st.w(HEARTS),
            coins_left: st.w(COINS_LEFT),
            frames: 0,
            checkpoint: 0,
            suffix: Vec::new(),
            chosen: 0,
            reached: 1,
            dist: 0.0,
            coin_dist: 0.0,
        });
        a.input_bytes += ENTRY_OVERHEAD;
        a
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [Entry] {
        &mut self.entries
    }

    pub fn get(&self, key: &CellKey) -> Option<&Entry> {
        self.index.get(key).map(|&i| &self.entries[i as usize])
    }

    pub fn checkpoint(&self, k: u32) -> &Checkpoint {
        &self.checkpoints[k as usize]
    }

    pub fn checkpoints(&self) -> usize {
        self.checkpoints.len()
    }

    /// Bytes the archive holds by its own count: states and inputs.
    pub fn stored_bytes(&self) -> u64 {
        self.stored_states * self.state_bytes + self.input_bytes
    }

    pub fn stored_states(&self) -> u64 {
        self.stored_states
    }

    /// Whether a state reaching `key` with `hearts` in `frames` would be kept: its key is new, or
    /// it beats the entry there (more hearts, then fewer frames).
    pub fn improves(&self, key: &CellKey, hearts: u16, frames: u32) -> bool {
        self.get(key).is_none_or(|e| {
            (hearts, std::cmp::Reverse(frames)) > (e.hearts, std::cmp::Reverse(e.frames))
        })
    }

    /// Whether an offer from `base` by `len` frames that `improves` would need its state stored
    /// (it becomes a checkpoint).
    pub fn wants_state(&self, len: usize) -> bool {
        len > CHECKPOINT_MIN
    }

    /// Offers a state; returns the entry's index if it was kept, and the checkpoint it took if it
    /// took one. `round` orders evictions.
    pub fn offer(&mut self, o: Offer, round: u64) -> (Option<u32>, Option<u32>) {
        let base = &self.checkpoints[o.base as usize];
        let frames = base.frames + o.inputs.len() as u32;
        if let Some(&i) = self.index.get(&o.key) {
            self.entries[i as usize].reached += 1;
        }
        if !self.improves(&o.key, o.hearts, frames) {
            return (None, None);
        }
        let (checkpoint, suffix, cp) = if self.wants_state(o.inputs.len()) {
            let game = o
                .game
                .expect("an offer the archive keeps past CHECKPOINT_MIN carries its state");
            let k = self.push_checkpoint(o.base, o.inputs.to_vec(), game, round);
            (k, Vec::new(), Some(k))
        } else {
            (o.base, o.inputs.to_vec(), None)
        };
        let fresh = Entry {
            key: o.key,
            hearts: o.hearts,
            coins_left: o.coins_left,
            frames,
            checkpoint,
            suffix,
            chosen: 0,
            reached: 1,
            dist: 0.0,
            coin_dist: o.coin_dist,
        };
        self.input_bytes += fresh.suffix.len() as u64;
        let i = match self.index.get(&o.key) {
            Some(&i) => {
                let e = &mut self.entries[i as usize];
                self.input_bytes -= e.suffix.len() as u64;
                // A better state is a new start for novelty, as the experiment had it.
                *e = Entry {
                    reached: e.reached,
                    ..fresh
                };
                i
            }
            None => {
                let i = self.entries.len() as u32;
                self.index.insert(o.key, i);
                self.entries.push(fresh);
                self.input_bytes += ENTRY_OVERHEAD;
                i
            }
        };
        (Some(i), cp)
    }

    fn push_checkpoint(&mut self, parent: u32, suffix: Vec<u8>, game: Game, round: u64) -> u32 {
        let p = &self.checkpoints[parent as usize];
        let frames = p.frames + suffix.len() as u32;
        let anchor = frames - self.anchor_frames(parent) >= ANCHOR_FRAMES;
        self.input_bytes += suffix.len() as u64 + CHECKPOINT_OVERHEAD;
        self.stored_states += 1;
        self.checkpoints.push(Checkpoint {
            parent: Some(parent),
            suffix,
            frames,
            anchor,
            state: Some(game),
            last_used: round,
        });
        self.checkpoints.len() as u32 - 1
    }

    /// The frames of `k`'s nearest anchor (itself included).
    fn anchor_frames(&self, mut k: u32) -> u32 {
        loop {
            let c = &self.checkpoints[k as usize];
            if c.anchor {
                return c.frames;
            }
            k = c.parent.expect("checkpoint 0 is an anchor");
        }
    }

    /// Checkpoint `k`'s state if it is stored.
    pub fn stored(&self, k: u32) -> Option<&Game> {
        self.checkpoints[k as usize].state.as_ref()
    }

    /// Checkpoint `k`'s state, rebuilt from its nearest stored ancestor if dropped; the frames
    /// replayed are added to `frames`, and the anchors rebuilt on the way are pushed to `anchors`,
    /// for `restore`, so the next rebuild near them starts there.
    pub fn rebuild(&self, k: u32, frames: &mut u64, anchors: &mut Vec<(u32, Game)>) -> Game {
        let mut chain = Vec::new();
        let mut at = k;
        let mut g = loop {
            let c = &self.checkpoints[at as usize];
            if let Some(g) = &c.state {
                break g.clone();
            }
            chain.push(at);
            at = c.parent.expect("checkpoint 0 keeps its state");
        };
        for &c in chain.iter().rev() {
            let cp = &self.checkpoints[c as usize];
            replay(&mut g, &cp.suffix);
            *frames += cp.suffix.len() as u64;
            if cp.anchor && c != k {
                anchors.push((c, g.clone()));
            }
        }
        g
    }

    /// Checkpoint `k`'s state, rebuilt if dropped (`rebuild`).
    pub fn state_of(&self, k: u32, frames: &mut u64) -> Game {
        self.rebuild(k, frames, &mut Vec::new())
    }

    /// Entry `i`'s state, rebuilt from its checkpoint; the checkpoint states that had to be rebuilt
    /// on the way (its own and anchors) are pushed to `rebuilt`, for `restore`.
    pub fn entry_state(&self, i: u32, frames: &mut u64, rebuilt: &mut Vec<(u32, Game)>) -> Game {
        let e = &self.entries[i as usize];
        let cp = self.rebuild(e.checkpoint, frames, rebuilt);
        if self.stored(e.checkpoint).is_none() {
            rebuilt.push((e.checkpoint, cp.clone()));
        }
        let mut g = cp;
        replay(&mut g, &e.suffix);
        *frames += e.suffix.len() as u64;
        g
    }

    /// Marks checkpoint `k` used in `round`.
    pub fn touch(&mut self, k: u32, round: u64) {
        let c = &mut self.checkpoints[k as usize];
        c.last_used = c.last_used.max(round);
    }

    /// Stores a rebuilt state again (it equals the dropped one).
    pub fn restore(&mut self, k: u32, game: Game, round: u64) {
        let c = &mut self.checkpoints[k as usize];
        c.last_used = c.last_used.max(round);
        if c.state.is_none() {
            c.state = Some(game);
            self.stored_states += 1;
        }
    }

    /// Drops states until the count is under the cap: non-anchors before anchors, checkpoints no
    /// entry starts from before those one does, least recently used first, then lowest index;
    /// checkpoint 0 never.
    pub fn evict(&mut self) {
        if self.stored_bytes() <= self.cap {
            return;
        }
        let mut used = vec![false; self.checkpoints.len()];
        for e in &self.entries {
            used[e.checkpoint as usize] = true;
        }
        let mut order: Vec<(bool, bool, u64, u32)> = self
            .checkpoints
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, c)| c.state.is_some())
            .map(|(k, c)| (c.anchor, used[k], c.last_used, k as u32))
            .collect();
        order.sort_unstable();
        for (_, _, _, k) in order {
            if self.stored_bytes() <= self.cap {
                break;
            }
            self.checkpoints[k as usize].state = None;
            self.stored_states -= 1;
        }
    }

    /// Drops every state but the start's (tests: rebuilt states must equal the stored ones).
    pub fn drop_states(&mut self) {
        for c in self.checkpoints.iter_mut().skip(1) {
            if c.state.take().is_some() {
                self.stored_states -= 1;
            }
        }
    }

    /// The joystick bytes from the start to checkpoint `k`.
    pub fn checkpoint_inputs(&self, mut k: u32) -> Vec<u8> {
        let mut parts = Vec::new();
        loop {
            let c = &self.checkpoints[k as usize];
            parts.push(&c.suffix);
            match c.parent {
                Some(p) => k = p,
                None => break,
            }
        }
        parts.into_iter().rev().flatten().copied().collect()
    }

    /// Entry `i`'s joystick bytes from the start.
    pub fn entry_inputs(&self, i: u32) -> Vec<u8> {
        let e = &self.entries[i as usize];
        let mut v = self.checkpoint_inputs(e.checkpoint);
        v.extend_from_slice(&e.suffix);
        v
    }
}

/// Plays joystick bytes on `g`. A replay never panics where the play it repeats did not: the
/// engine is deterministic.
pub fn replay(g: &mut Game, joys: &[u8]) {
    for &j in joys {
        g.step_unseen(FrameInput::joy(Joy(j)));
    }
}
