//! Which part of the game is running. Each scene is a mechanic task's: the level (T14), the slot
//! machine and bonus level (T19), the front end (T20), the game flow between them (T21).

use sf_paula::PaulaWrite;

use crate::audio::AudioDriver;
use crate::game::Pacing;
use crate::layout::{Ram, h0};
use crate::level::{Hw, LevelCtx};
use crate::mem::Bus;
use crate::render::{self, DisplayData, Picture, View};
use crate::state::State;

/// What a scene's frame works on: the state, what the disks decide, the sound driver and the
/// hardware the frame reads.
pub struct Machine<'a> {
    pub state: &'a mut State,
    pub display: &'a DisplayData,
    pub audio: &'a mut AudioDriver,
    pub hw: Hw,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Scene {
    /// Nothing ported yet: changes no state, writes nothing to Paula and renders black. Until the
    /// mechanic tasks land, every start is here.
    #[default]
    Placeholder,
    /// In a level: the frog, the camera, scrolling, the clock and sound (T14); objects come with
    /// T15–T18.
    Level,
}

/// `logic_running` (h0+$9d6): set by `level_frame_loop` while a level plays.
const LOGIC_RUNNING: u32 = h0(0x9d6);
/// `level_state` (h0+$a06): negative while a level plays.
const LEVEL_STATE: u32 = h0(0xa06);

impl Scene {
    /// The scene the original's memory is in, for a transplant: a level while
    /// `level_frame_loop` runs it, otherwise none ported yet.
    pub fn of(ram: &dyn Ram) -> Scene {
        let word = |a: u32| (ram.byte(a) as u16) << 8 | ram.byte(a + 1) as u16;
        if word(LOGIC_RUNNING) != 0 && ram.byte(LEVEL_STATE) & 0x80 != 0 {
            Scene::Level
        } else {
            Scene::Placeholder
        }
    }

    /// Run one frame of this scene; returns the Paula writes it made, in order, and the picture
    /// the frame shows when it is not the state's at the frame's end: a level's is drawn after
    /// the vertical blank, before the pass, whose drawing (the panel's digits, a redrawn tile)
    /// the beam has passed by the time the pass gets to it (docs/12 "Late redraws").
    pub fn step(
        &mut self,
        m: &mut Machine,
        pacing: Pacing,
        view: View,
    ) -> (Vec<PaulaWrite>, Option<Picture>) {
        let mut out = Vec::new();
        let mut picture = None;
        match self {
            Scene::Placeholder => {}
            Scene::Level => {
                let mut c = LevelCtx {
                    bus: Bus::new(m.state, m.display.rom()),
                    dispatch: crate::level::standard_dispatch(),
                    hw: m.hw,
                    audio: m.audio,
                    sound: m.display.sound(),
                    paula: &mut out,
                };
                crate::level::frame::vbl(&mut c);
                picture = Some(render::level(c.bus.st, m.display, view));
                // A late pass (DESIGN §2.1) runs here too: what is left over is the runner's lag.
                let _ = pacing;
                if crate::level::frame::pass_due(&c) {
                    crate::level::frame::pass(&mut c);
                }
            }
        }
        (out, picture)
    }

    pub fn render(&self, state: &State, display: &DisplayData, view: View) -> Picture {
        match self {
            Scene::Placeholder => Picture::black(view),
            Scene::Level => render::level(state, display, view),
        }
    }
}
