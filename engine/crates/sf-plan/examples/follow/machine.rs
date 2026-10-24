//! What a route is played on: the engine (with the experiment's perturbations: a late start, lag
//! frames, a difficulty poke) or the original. Executors only read the state and choose inputs.

use std::collections::BTreeSet;
use std::sync::Arc;

use sf_conform::sync;
use sf_core::input::FrameInput;
use sf_core::layout::{AmigaMemory, h0};
use sf_core::{DisplayData, Game, Start, State};
use sf_oracle::Amiga;

/// `opt_game_mode` (h0+$9e6): 0 easy, 1 normal, 2 hard.
pub const OPT_GAME_MODE: u32 = h0(0x9e6);

pub trait Machine {
    /// The current state (after the last frame played).
    fn state(&self) -> &State;
    /// Plays one frame with `x` held.
    fn step(&mut self, x: FrameInput) -> Result<(), String>;
    /// An engine copy of the current state, for look-ahead.
    fn game(&self) -> Game;
    /// Whether the state is part way through a pass (the original lagging): a copy then is not a
    /// pass's end and look-ahead waits.
    fn mid_pass(&self) -> bool;
    /// Frames on which the game did not advance (the engine's injected lag; the original's
    /// mid-pass frames).
    fn lag_frames(&self) -> u32;
}

/// Where the engine injects lag (P4): on these frames the game is not stepped and the input is
/// lost, as when the original's pass runs over two frames and reads the joystick once.
#[derive(Debug, Clone, Default)]
pub enum Lag {
    #[default]
    None,
    /// Frames (counted from the run's frame 0) the original ended mid-pass on.
    At(BTreeSet<u32>),
    /// `len` frames every `period`, from `phase`.
    Every { period: u32, len: u32, phase: u32 },
}

impl Lag {
    fn lags(&self, frame: u32) -> bool {
        match self {
            Lag::None => false,
            Lag::At(s) => s.contains(&frame),
            Lag::Every { period, len, phase } => {
                frame >= *phase && (frame - phase) % period < *len
            }
        }
    }
}

pub struct Engine {
    pub g: Game,
    pub frame: u32,
    pub lag: Lag,
    lagged: u32,
}

impl Engine {
    pub fn new(g: Game, lag: Lag) -> Engine {
        Engine { g, frame: 0, lag, lagged: 0 }
    }
}

impl Machine for Engine {
    fn state(&self) -> &State {
        self.g.state()
    }
    fn step(&mut self, x: FrameInput) -> Result<(), String> {
        if self.lag.lags(self.frame) {
            self.lagged += 1;
        } else {
            self.g.step_unseen(x);
        }
        self.frame += 1;
        Ok(())
    }
    fn game(&self) -> Game {
        self.g.clone()
    }
    fn mid_pass(&self) -> bool {
        false
    }
    fn lag_frames(&self) -> u32 {
        self.lagged
    }
}

pub struct Oracle {
    pub a: Amiga,
    display: Arc<DisplayData>,
    mem: AmigaMemory,
    st: State,
    mid: bool,
    lagged: u32,
}

impl Oracle {
    pub fn new(a: Amiga, display: Arc<DisplayData>) -> Oracle {
        let mem = sync::core_memory(a.memory());
        let st = State::transplant(&mem);
        Oracle { a, display, mem, st, mid: false, lagged: 0 }
    }
    /// After a poke: the state seen is the original's memory again.
    pub fn refresh(&mut self) {
        self.mem = sync::core_memory(self.a.memory());
        self.st = State::transplant(&self.mem);
    }
}

impl Machine for Oracle {
    fn state(&self) -> &State {
        &self.st
    }
    fn step(&mut self, x: FrameInput) -> Result<(), String> {
        sync::step_inserting(&mut self.a, x)?;
        self.mem = sync::core_memory(self.a.memory());
        self.mid = sync::mid_pass(&self.a.cpu(), &self.mem);
        self.lagged += self.mid as u32;
        self.st = State::transplant(&self.mem);
        Ok(())
    }
    fn game(&self) -> Game {
        Game::with_display(self.display.clone(), Start::Transplant(self.mem.clone()))
    }
    fn mid_pass(&self) -> bool {
        self.mid
    }
    fn lag_frames(&self) -> u32 {
        self.lagged
    }
}
