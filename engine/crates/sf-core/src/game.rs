//! The decision function (DESIGN §3.3): `Game` is a function of the [`GameData`], its [`Start`]
//! and the inputs, and nothing else.

use sf_assets::{GameData, LevelId};
use sf_paula::PaulaWrite;

use crate::input::FrameInput;
use crate::layout::{AmigaMemory, VarId};
use crate::rng::Rng;
use crate::scene::Scene;
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

/// Presentation choices that change the picture and nothing else (DESIGN §2.7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct View {
    /// Extra lores columns of the level drawn on each side for widescreen; 0 for 4:3.
    pub extra_columns_each_side: u16,
}

/// A frame as Denise would show it: palette indices inside the display window, and each line's
/// 32-colour palette (the copper may change it per line). `left` and `top` are the window's
/// position in DIWSTRT's units (lores pixel, raster line), as the oracle reports them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Picture {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
    /// Row-major, `width * height` palette indices; 32..=63 are extra-half-brite.
    pub pixels: Vec<u8>,
    /// One palette per line, 12-bit OCS colour `0x0RGB`.
    pub palettes: Vec<[u16; 32]>,
}

/// The PAL display window the original opens on its title screen: DIWSTRT `$2C81`, 320×256.
const DEFAULT_WINDOW: (u16, u16, u16, u16) = (0x81, 0x2C, 320, 256);

impl Picture {
    /// The default window, every pixel colour 0, every colour black.
    pub fn black(view: View) -> Picture {
        let (left, top, width, height) = DEFAULT_WINDOW;
        let width = width + 2 * view.extra_columns_each_side;
        Picture {
            left: left - view.extra_columns_each_side,
            top,
            width,
            height,
            pixels: vec![0; width as usize * height as usize],
            palettes: vec![[0; 32]; height as usize],
        }
    }

    /// The 12-bit colour shown at (`x`, `y`).
    pub fn rgb12(&self, x: u16, y: u16) -> u16 {
        let i = self.pixels[y as usize * self.width as usize + x as usize];
        let pal = &self.palettes[y as usize];
        if i < 32 {
            pal[i as usize]
        } else {
            pal[(i & 31) as usize] >> 1 & 0x777
        }
    }
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
    scene: Scene,
    /// The seed a power-on start is waiting to put in place.
    seed: Option<Rng>,
    frame: u32,
}

impl Game {
    pub fn new(_data: &GameData, start: Start) -> Game {
        let (state, seed) = match start {
            Start::PowerOn { seed } => (State::default(), Some(seed)),
            Start::Transplant(mem) => (State::transplant(&mem), None),
            Start::Level(_) => (State::default(), None),
        };
        Game {
            state,
            scene: Scene::Placeholder,
            seed,
            frame: 0,
        }
    }

    /// One PAL frame of normal play: the main loop is never late.
    pub fn step(&mut self, input: FrameInput) -> FrameOut {
        self.step_paced(input, Pacing::OnTime)
    }

    /// One PAL frame on the original's schedule (conformance runs; see [`Pacing`]).
    pub fn step_paced(&mut self, input: FrameInput, pacing: Pacing) -> FrameOut {
        let paula = self.scene.step(&mut self.state, input, pacing);
        self.frame += 1;
        FrameOut {
            picture: self.render(View::default()),
            paula,
        }
    }

    pub fn render(&self, view: View) -> Picture {
        self.scene.render(&self.state, view)
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

    #[test]
    fn widescreen_widens_the_window_by_the_extra_columns() {
        let p = Picture::black(View {
            extra_columns_each_side: 32,
        });
        assert_eq!((p.left, p.width), (0x81 - 32, 384));
    }

    #[test]
    fn extra_half_brite_halves_the_colour() {
        let mut p = Picture::black(View::default());
        p.palettes[0][3] = 0xFE8;
        p.pixels[0] = 35;
        p.pixels[1] = 3;
        assert_eq!((p.rgb12(0, 0), p.rgb12(1, 0)), (0x774, 0xFE8));
    }
}
