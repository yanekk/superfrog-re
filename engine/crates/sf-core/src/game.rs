//! The decision function (DESIGN §3.3): `Game` is a function of the [`GameData`], its [`Start`]
//! and the inputs, and nothing else.

use std::sync::Arc;

use sf_assets::{GameData, LevelId};
use sf_paula::PaulaWrite;

use crate::audio::AudioDriver;
use crate::input::{FrameInput, KeySet};
use crate::layout::{AmigaMemory, Ram, VarId, h0};
use crate::level::Hw;
use crate::render::{DisplayData, Picture, View};
use crate::rng::Rng;
use crate::scene::{Machine, Scene};
use crate::state::State;

/// Where a game starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// Switch on. The original seeds its RNG once, from a clock, while it boots (docs/08 "RNG");
    /// the seed is part of the start (owner, 2026-10-11): normal play passes one from the host
    /// clock, a power-on conformance run the oracle's. The boot scene (T20/T21) puts it in place
    /// at the point the original seeds; until then the state stays as power-on leaves it.
    PowerOn { seed: Rng },
    /// Continue from the original's memory at a sync point: every [`crate::LAYOUT`] variable,
    /// `rng_state` included, is copied in.
    Transplant(AmigaMemory),
    /// Enter a level as the original does (T14).
    Level(LevelId),
}

/// How the original's main loop kept time on this frame (DESIGN §2.1; owner, 2026-10-11).
///
/// In a level the original's main loop runs one 25 Hz pass every second vertical blank, and the
/// 50 Hz work (frog, camera, clock) runs in the vertical-blank interrupt. In busy stretches the
/// pass overruns, so a vertical blank arrives while it is still running (a late pass; docs/08
/// "Lag"). Normal play never lags; conformance runs feed the original's schedule from the oracle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Pacing {
    /// The main loop was waiting when this frame's vertical blank came: normal play, always.
    #[default]
    OnTime,
    /// The original's main loop was still in its pass at this frame's vertical blank. The engine
    /// then runs the interrupt's work at one fixed point in the late pass.
    Late,
}

/// What one frame produced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FrameOut {
    pub picture: Picture,
    /// The frame's Paula register writes, in order (DESIGN §2.1 (2)).
    pub paula: Vec<PaulaWrite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    state: State,
    /// What the display reads from the disks (shared: it never changes once built).
    display: Arc<DisplayData>,
    scene: Scene,
    /// The seed a power-on start is waiting to put in place.
    seed: Option<Rng>,
    frame: u32,
    /// The sound driver (T11): its variables are mirrored into the state's after each frame.
    audio: AudioDriver,
    /// The keyboard: the keys held last frame, and the byte it last sent, which CIA-A's serial
    /// register holds (the code shifted up one, bit 0 set for a key going up, all inverted).
    keys: KeySet,
    sdr: u8,
}

/// `key_down`, `key_up` (h0+$16078, h0+$16079).
const KEY_DOWN: u32 = h0(0x16078);
const KEY_UP: u32 = h0(0x16079);

/// The byte the keyboard sends for `code` going down or up.
fn key_byte(code: u8, up: bool) -> u8 {
    !(code << 1 | up as u8)
}

impl Game {
    pub fn new(data: &GameData, start: Start) -> Game {
        Game::with_display(Arc::new(DisplayData::new(data)), start)
    }

    /// [`Game::new`] with the disks' display content already built, as a caller making many
    /// games from one [`GameData`] (conformance runs) keeps it.
    pub fn with_display(display: Arc<DisplayData>, start: Start) -> Game {
        let mut audio = AudioDriver::new(display.sound());
        let mut sdr = 0xff;
        let (state, scene, seed) = match start {
            Start::PowerOn { seed } => (State::default(), Scene::Placeholder, Some(seed)),
            Start::Transplant(mem) => {
                let word = |a: u32| (mem.byte(a) as u32) << 8 | mem.byte(a + 1) as u32;
                let long = |a: u32| word(a) << 16 | word(a + 2);
                let files = if display.sound().is_complete() {
                    crate::audio::files_for(long(h0(0x40da)), long(h0(0x9ec)))
                } else {
                    Vec::new()
                };
                audio = AudioDriver::transplant(display.sound(), &|a| mem.byte(a), &files);
                // The keyboard's last byte, as the last poll decoded it.
                sdr = match (mem.byte(KEY_DOWN), mem.byte(KEY_UP)) {
                    (0, up) if up != 0 => key_byte(up, true),
                    (down, _) => key_byte(down, false),
                };
                (State::transplant(&mem), Scene::of(&mem), None)
            }
            Start::Level(_) => (State::default(), Scene::Placeholder, None),
        };
        Game {
            state,
            display,
            scene,
            seed,
            frame: 0,
            audio,
            keys: KeySet::NONE,
            sdr,
        }
    }

    /// One PAL frame of normal play: the main loop is never late.
    pub fn step(&mut self, input: FrameInput) -> FrameOut {
        self.step_paced(input, Pacing::OnTime)
    }

    /// One PAL frame on the original's schedule (conformance runs; see [`Pacing`]).
    pub fn step_paced(&mut self, input: FrameInput, pacing: Pacing) -> FrameOut {
        // A key going down or up this frame is the byte the keyboard sends (the last one wins).
        for code in input.keys.codes() {
            if !self.keys.contains(code) {
                self.sdr = key_byte(code, false);
            }
        }
        for code in self.keys.codes() {
            if !input.keys.contains(code) {
                self.sdr = key_byte(code, true);
            }
        }
        self.keys = input.keys;
        let mut hw = Hw::of(input);
        hw.sdr = self.sdr;
        let (paula, shown) = {
            let mut m = Machine {
                state: &mut self.state,
                display: &self.display,
                audio: &mut self.audio,
                hw,
            };
            self.scene.step(&mut m, pacing, View::default())
        };
        for (at, bytes) in self.audio.vars(self.display.sound()) {
            if at >= crate::layout::H0 {
                self.state.poke(at, &bytes);
            }
        }
        self.frame += 1;
        FrameOut {
            picture: shown.unwrap_or_else(|| self.render(View::default())),
            paula,
        }
    }

    /// The frame the original shows for this state (DESIGN §3.3), widened by `view`.
    pub fn render(&self, view: View) -> Picture {
        self.scene.render(&self.state, &self.display, view)
    }

    /// Every modelled variable's value, as the original would hold it at its [`crate::LAYOUT`]
    /// address.
    pub fn probe(&self) -> Vec<(VarId, u32)> {
        self.state.probe()
    }

    /// Write `bytes` at `addr` into the modelled variables, as [`State::poke`] does. For
    /// conformance tooling, which plants a known divergence to prove the runner finds it.
    pub fn poke(&mut self, addr: u32, bytes: &[u8]) {
        self.state.poke(addr, bytes);
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn scene(&self) -> Scene {
        self.scene
    }

    /// The seed a power-on start has not yet put in place.
    pub fn pending_seed(&self) -> Option<Rng> {
        self.seed
    }

    /// Frames stepped since the start.
    pub fn frame(&self) -> u32 {
        self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placeholder_game_steps_black_and_silent_and_keeps_its_state() {
        let mut mem = AmigaMemory {
            chip: vec![0; 16],
            slow: vec![0; 512 * 1024],
        };
        let at = (crate::layout::def(VarId::RngState).addr - crate::layout::SLOW_BASE) as usize;
        mem.slow[at + 3] = 9;
        let mut g = Game::new(&GameData::default(), Start::Transplant(mem));
        let before = g.probe();
        let out = g.step(FrameInput::NONE);
        assert!(out.paula.is_empty());
        assert_eq!((out.picture.width, out.picture.height), (320, 256));
        assert!((0..256).all(|y| (0..320).all(|x| out.picture.rgb12(x, y) == 0)));
        assert_eq!(g.probe(), before);
        assert_eq!(g.frame(), 1);
    }

    #[test]
    fn power_on_holds_its_seed_for_the_boot_scene() {
        let seed = Rng::seeded(1, 2);
        let g = Game::new(&GameData::default(), Start::PowerOn { seed });
        assert_eq!(g.pending_seed(), Some(seed));
        assert_eq!(g.state(), &State::default());
    }
}
