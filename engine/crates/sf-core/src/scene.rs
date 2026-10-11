//! Which part of the game is running. Each scene is a mechanic task's: the level (T14), the slot
//! machine and bonus level (T19), the front end (T20), the game flow between them (T21).

use sf_paula::PaulaWrite;

use crate::game::{Pacing, Picture, View};
use crate::input::FrameInput;
use crate::state::State;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Scene {
    /// Nothing ported yet: changes no state, writes nothing to Paula and renders black. Until the
    /// mechanic tasks land, every start is here.
    #[default]
    Placeholder,
}

impl Scene {
    /// Run one frame of this scene; returns the Paula writes it made, in order.
    pub fn step(
        &mut self,
        _state: &mut State,
        _input: FrameInput,
        _pacing: Pacing,
    ) -> Vec<PaulaWrite> {
        match self {
            Scene::Placeholder => Vec::new(),
        }
    }

    pub fn render(&self, _state: &State, view: View) -> Picture {
        match self {
            Scene::Placeholder => Picture::black(view),
        }
    }
}
