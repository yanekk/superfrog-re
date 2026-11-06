//! Forest Dumb Forever's engine core: the game's measured timing, the idle title's placeholder
//! scene (forest-dumb-start), and the level engine, `Game`, which enters W1-1 at the level sync
//! point and steps it a pass at a time (plans/forest-dumb-level-1 DESIGN §2.3, §3.3).
//!
//! Pure: everything arrives per step as a parameter, and the disks' data through a closure. No
//! clock, file or network (`fd-conform/tests/purity.rs`).

pub mod character;
pub mod game;
pub mod game_over;
pub mod layout;
pub mod level;
pub mod level_end;
pub mod ported;
pub mod program;
pub mod recording;
pub mod render;
pub mod scene;
pub mod state;
pub mod timing;
pub mod title;

pub use game::{Game, GameData, Start};
pub use layout::{LAYOUT, Mem, Transplant, Var, Width, layout_var};
pub use level::{Bob, Dispatch, DrawList, Event, LevelCtx, Object, ObjectKind, register};
pub use ported::{PORTED, Ported};
pub use program::{HUNK_BASES, Program, Scratch};
pub use scene::Scene;
pub use state::State;
pub use timing::FdTiming;
pub use title::TitleScene;
