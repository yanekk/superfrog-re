//! What an executor sees each frame, and the loop that plays one on a machine and records what it
//! pressed and how the run went.

use std::time::Instant;

use sf_core::State;
use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_core::layout::h0;
use sf_plan::core::map::pressed;
use sf_plan::core::{Facts, frog_cell, landed};

use crate::machine::Machine;

/// `level_state` (h0+$a06): negative while playing; 1 exit, 2 bonus end, $64 boss, $190 death.
pub const LEVEL_STATE: u32 = h0(0xa06);
pub const COINS_LEFT: u32 = h0(0x832);
pub const HEARTS: u32 = h0(0x830);

/// The frog as an executor reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Obs {
    pub x: i16,
    pub y: i16,
    pub cell: (u16, u16),
    pub landed: bool,
    pub facing: u16,
    pub coins: u16,
    pub hearts: u16,
    pub lives: u16,
    pub dying: bool,
    pub level_state: u32,
    pub switches: u32,
}

impl Obs {
    pub fn of(st: &State, facts: &Facts) -> Obs {
        Obs {
            x: st.w(FROG + f::X) as i16,
            y: st.w(FROG + f::Y) as i16,
            cell: frog_cell(st),
            landed: landed(st),
            facing: st.w(FROG + f::FACING),
            coins: st.w(COINS_LEFT),
            hearts: st.w(HEARTS),
            lives: st.w(FROG + f::LIVES),
            dying: st.w(FROG + f::DYING) != 0,
            level_state: st.l(LEVEL_STATE),
            switches: pressed(facts, st),
        }
    }
    pub fn playing(&self) -> bool {
        self.level_state & 0x8000_0000 != 0
    }
    /// The level left by its exit (not a death, not a boss or bonus end).
    pub fn exited(&self) -> bool {
        self.level_state == 1
    }
}

pub trait Exec {
    /// The input for local frame `t` (frames since the executor started), the machine's state
    /// before it.
    fn decide(&mut self, m: &dyn Machine, o: &Obs, t: u32) -> FrameInput;
    /// Called after a life is lost and the frog is back in play: re-find its place on the route.
    fn respawned(&mut self, _o: &Obs) {}
    /// Engine frames simulated for look-ahead so far.
    fn engine_frames(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Frames from the run's frame 0 to the end of the frame `level_state` became 1.
    pub exit: Option<u32>,
    pub lives_lost: u32,
    pub hearts_lost: u32,
    pub frames: u32,
    /// Every frame's input, the delay's idle frames and the exit's tail included.
    pub rec: Vec<FrameInput>,
    /// (cell, coins left, switches) before every 8th frame while playing.
    pub trace: Vec<((u16, u16), u16, u32)>,
    pub last: Obs,
    pub first_life_lost: Option<u32>,
    pub game_over: bool,
    pub secs: f64,
    pub engine_frames: u64,
    pub lag_frames: u32,
    pub err: Option<String>,
}

/// Idle frames after the exit, so a recording judged by `verify` shows the exit's milestone.
pub const TAIL: u32 = 30;
/// The level's `level_state` is not −2 on its first frames.
const SETTLE: u32 = 10;

/// Plays `exec` on `m` for at most `max` frames, `delay` idle frames first (P3).
pub fn play(m: &mut dyn Machine, exec: &mut dyn Exec, facts: &Facts, max: u32, delay: u32) -> Outcome {
    let t0 = Instant::now();
    let mut out = Outcome::default();
    let mut o = Obs::of(m.state(), facts);
    let start_lives = o.lives;
    let mut was_playing = o.playing();
    let mut awaiting_respawn = false;
    let mut tail: Option<u32> = None;
    for k in 0..max {
        if k % 8 == 0 && o.playing() {
            out.trace.push((o.cell, o.coins, o.switches));
        }
        let x = if k < delay || tail.is_some() {
            FrameInput::NONE
        } else {
            exec.decide(m, &o, k - delay)
        };
        out.rec.push(x);
        if let Err(e) = m.step(x) {
            out.err = Some(e);
            break;
        }
        let n = Obs::of(m.state(), facts);
        out.hearts_lost += o.hearts.saturating_sub(n.hearts) as u32;
        if n.lives < o.lives {
            out.lives_lost += (o.lives - n.lives) as u32;
            out.first_life_lost.get_or_insert(k);
            awaiting_respawn = true;
        }
        o = n;
        if let Some(t) = tail.as_mut() {
            *t -= 1;
            if *t == 0 {
                break;
            }
            continue;
        }
        if k >= SETTLE && was_playing && !o.playing() && o.exited() && !o.dying {
            out.exit = Some(k + 1);
            tail = Some(TAIL);
        }
        if awaiting_respawn && o.playing() && !o.dying {
            awaiting_respawn = false;
            exec.respawned(&o);
        }
        // Game over: the lives ran out and the level is not played again.
        if o.lives == 0 && o.lives < start_lives && !o.playing() && k > SETTLE && o.level_state == 0x190 {
            out.game_over = true;
        }
        was_playing = was_playing || o.playing();
    }
    out.frames = out.rec.len() as u32;
    out.last = o;
    out.secs = t0.elapsed().as_secs_f64();
    out.engine_frames = exec.engine_frames();
    out.lag_frames = m.lag_frames();
    out
}

/// The route's guide points (every 8th frame of a reference run) covered in order by `trace`, as a
/// share: a point is covered when a trace sample lies in its region (`GuidePoint::covers`, the
/// planner's own "on the path") at or after the last covered point's sample.
pub fn followed_share(guide: &[sf_plan::core::GuidePoint], trace: &[((u16, u16), u16, u32)]) -> f64 {
    use sf_plan::core::{CellKey, band};
    if guide.is_empty() {
        return 0.0;
    }
    let keys: Vec<CellKey> = trace
        .iter()
        .map(|&(c, coins, sw)| CellKey {
            col: c.0 / 2,
            row: c.1 / 2,
            band: band(coins),
            switches: sw,
            wings: false,
            via: 0,
        })
        .collect();
    let mut at = 0usize;
    let mut covered = 0usize;
    for p in guide {
        // A point is looked for from the last covered point's sample on, within the next 400.
        if let Some(j) = (at..keys.len().min(at + 400)).find(|&j| p.covers(&keys[j])) {
            covered += 1;
            at = j;
        }
    }
    covered as f64 / guide.len() as f64
}

/// A recording as a script file's text.
pub fn script_text(header: &str, rec: &[FrameInput]) -> String {
    let mut s = String::new();
    for l in header.lines() {
        s += &format!("# {l}\n");
    }
    let mut last = None;
    for (k, x) in rec.iter().enumerate() {
        if Some(*x) != last {
            s += &format!("{k}: {}\n", sf_conform::InputScript::format_input(*x));
            last = Some(*x);
        }
    }
    s += &format!("{}: -\n", rec.len());
    s
}
