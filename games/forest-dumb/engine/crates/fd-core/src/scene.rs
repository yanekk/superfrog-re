//! What the game is showing. Only play exists until the game over (T23) and level end (T26) scenes.

/// The game's scene; its state lives in [`crate::Game`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scene {
    /// Play in a level: one pass of `game_frame` a step.
    Level,
    // T23: GameOver. T26: LevelEnd.
}
