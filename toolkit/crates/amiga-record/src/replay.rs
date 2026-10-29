//! The headless replay loop: a recording's inputs stepped through a game unpaced, one a frame, as
//! the app plays them, every step's picture handed on.

use amiga_chips::Picture;
use amiga_input::FrameInput;

/// A game a recording is replayed on.
pub trait Replayable {
    /// The picture before any step: what a recording of no frames stopped on.
    fn picture(&self) -> Picture;
    /// Steps frame `frame` (0 the first input's) with `input`, and returns the picture the step
    /// showed. A game with marks of its own applies frame `frame`'s first.
    fn step(&mut self, frame: u32, input: FrameInput) -> Picture;
}

/// `inputs` stepped through `game` in order; `each` sees every step's frame number, the game after
/// it and the picture it showed. Returns the last picture: what the app showed when the recording
/// stopped.
pub fn replay<R: Replayable>(
    game: &mut R,
    inputs: &[FrameInput],
    mut each: impl FnMut(u32, &R, &Picture),
) -> Picture {
    let mut last = game.picture();
    for (k, &input) in inputs.iter().enumerate() {
        let pic = game.step(k as u32, input);
        each(k as u32, game, &pic);
        last = pic;
    }
    last
}
