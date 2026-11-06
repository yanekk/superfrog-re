//! What the game is showing: play, the game over screen (T23), or the level's end (T26).

use crate::game_over::GameOver;
use crate::level_end::{Complete, LevelEnd};

/// The game's scene; its state lives in [`crate::Game`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scene {
    /// Play in a level: one pass of `game_frame` a step.
    Level,
    /// The original's game over screen, waiting for fire (T23). Its end restarts the level.
    GameOver(GameOver),
    /// The step `main_loop` found the level's end in and ran `next_world` (T26), where the
    /// comparison with the original ends.
    LevelEnd(LevelEnd),
    /// "Level 1 complete" until fire, which restarts the level (DESIGN §2.5). Temporary: it stands
    /// where the original loads world 2, and goes when world 2 lands.
    Complete(Complete),
}
