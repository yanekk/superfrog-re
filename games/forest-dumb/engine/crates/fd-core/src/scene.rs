//! What the game is showing: play, or the game over screen (T23). Level end is T26's.

use crate::game_over::GameOver;

/// The game's scene; its state lives in [`crate::Game`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scene {
    /// Play in a level: one pass of `game_frame` a step.
    Level,
    /// The original's game over screen, waiting for fire (T23). Its end restarts the level.
    GameOver(GameOver),
    // T26: LevelEnd.
}
