//! Which part of the game is running. Each scene is a mechanic task's: the level (T14), the slot
//! machine and bonus level (T19), the front end (T20), the game flow between them (T21).

use sf_paula::PaulaWrite;

use crate::game::Pacing;
use crate::input::FrameInput;
use crate::layout::{Ram, h0};
use crate::render::{self, DisplayData, Picture, View};
use crate::state::State;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Scene {
    /// Nothing ported yet: changes no state, writes nothing to Paula and renders black. Until the
    /// mechanic tasks land, every start is here.
    #[default]
    Placeholder,
    /// In a level. T13 draws it; T14 runs it (until then a step changes nothing).
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

    /// Run one frame of this scene; returns the Paula writes it made, in order.
    pub fn step(
        &mut self,
        _state: &mut State,
        _input: FrameInput,
        _pacing: Pacing,
    ) -> Vec<PaulaWrite> {
        match self {
            Scene::Placeholder | Scene::Level => Vec::new(),
        }
    }

    pub fn render(&self, state: &State, display: &DisplayData, view: View) -> Picture {
        match self {
            Scene::Placeholder => Picture::black(view),
            Scene::Level => render::level(state, display, view),
        }
    }
}
