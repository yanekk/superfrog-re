//! The game-free half of a bug recorder: a game played from a start the game names, recorded as
//! every engine step's input with the start's seed, the build and the owner's note; written as
//! readable text and zipped with a screenshot; read back and fed to the engine again.
//!
//! A game supplies its header through [`GameRecording`]: where play started, where it stopped, and
//! any toggles of its own, with their text and the first line's magic. Everything else here is the
//! same for every game. Pure: no clock, file or device is read here. Writing the zip to a file is
//! the app's; checking the screenshot decodes a PNG, which is the comparison harness's.

mod recorder;
mod recording;
mod replay;
mod zip;

pub use recorder::{Recorder, Replay};
pub use recording::{
    GameRecording, Head, HeaderReader, HeaderWriter, MAX_FRAMES, Recording, RecordingError,
};
pub use replay::{Replayable, replay};
pub use zip::{ZIP_RECORDING, ZIP_SCREENSHOT, read_zip, zip_bytes};
