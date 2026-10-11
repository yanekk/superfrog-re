//! Pure: the game — State, Game::step, renderer, audio driver, layout table (DESIGN §3.2).
//!
//! [`Game`] is the decision function (DESIGN §3.3): a function of the [`GameData`], its [`Start`]
//! and the inputs, one PAL frame per [`Game::step`]. Every original variable the engine models is
//! in [`layout::LAYOUT`] with its address, so conformance can transplant the original's memory in
//! and probe the engine's values back out.

pub mod game;
pub mod input;
pub mod layout;
pub mod rng;
pub mod scene;
pub mod state;

pub use game::{FrameOut, Game, Pacing, Picture, Start, View};
pub use layout::{AmigaMemory, LAYOUT, Ram, VarDef, VarId, Width};
pub use rng::Rng;
pub use scene::Scene;
pub use sf_assets::{GameData, LevelId};
pub use sf_paula::PaulaWrite;
pub use state::State;
