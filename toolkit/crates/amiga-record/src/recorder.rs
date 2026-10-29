//! The recorder and the replay, as state machines the app drives: a recording held in memory from
//! the start's first frame, one input a stepped frame, until the app stops it; and the other way
//! round, a recording's inputs fed one a step in place of the player's, then nothing, so the app
//! holds on the last frame.

use amiga_input::FrameInput;

use crate::recording::{GameRecording, MAX_FRAMES, Recording};

/// Whether a recording is running, and the recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recorder<G: GameRecording> {
    Off,
    Recording { rec: Recording<G> },
}

impl<G: GameRecording> Default for Recorder<G> {
    fn default() -> Self {
        Recorder::Off
    }
}

impl<G: GameRecording> Recorder<G> {
    /// A recording started at `header`'s start from `seed`, no frame stepped yet.
    pub fn start(header: G::Header, seed: Vec<u8>, build: &str) -> Self {
        Recorder::Recording {
            rec: Recording {
                build: build.to_string(),
                seed,
                note: String::new(),
                header,
                inputs: Vec::new(),
            },
        }
    }

    /// One stepped frame's input. Past [`MAX_FRAMES`] nothing more is held ([`Recorder::full`]):
    /// the app stops the recording there.
    pub fn step(&mut self, input: FrameInput) {
        if let Recorder::Recording { rec } = self
            && rec.inputs.len() < MAX_FRAMES as usize
        {
            rec.inputs.push(input);
        }
    }

    /// The running recording, for a game's own marks (a toggle before the next step).
    pub fn recording_mut(&mut self) -> Option<&mut Recording<G>> {
        match self {
            Recorder::Off => None,
            Recorder::Recording { rec } => Some(rec),
        }
    }

    /// How many frames the recording holds; `None` when none runs.
    pub fn frames(&self) -> Option<usize> {
        match self {
            Recorder::Off => None,
            Recorder::Recording { rec } => Some(rec.inputs.len()),
        }
    }

    pub fn is_recording(&self) -> bool {
        matches!(self, Recorder::Recording { .. })
    }

    /// Has the recording reached the longest a recording may be (a day)?
    pub fn full(&self) -> bool {
        matches!(self, Recorder::Recording { rec } if rec.inputs.len() >= MAX_FRAMES as usize)
    }

    /// The recording, stopped at `at` ([`GameRecording::stop`]) with `note`; `None` when none was
    /// running.
    pub fn finish(self, at: G::Stop, note: String) -> Option<Recording<G>> {
        match self {
            Recorder::Off => None,
            Recorder::Recording { mut rec } => {
                G::stop(&mut rec, at);
                rec.note = note;
                Some(rec)
            }
        }
    }
}

/// A recording's inputs, fed to the game one a step in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay<G: GameRecording> {
    rec: Recording<G>,
    played: usize,
    /// [`Replay::just_finished`] has said so.
    told: bool,
}

impl<G: GameRecording> Replay<G> {
    pub fn new(rec: &Recording<G>) -> Self {
        Replay {
            rec: rec.clone(),
            played: 0,
            told: false,
        }
    }

    /// The recording replayed.
    pub fn recording(&self) -> &Recording<G> {
        &self.rec
    }

    /// How many inputs have been fed.
    pub fn played(&self) -> usize {
        self.played
    }

    /// The input for the next step, `None` once every recorded one has been played.
    pub fn next(&mut self) -> Option<FrameInput> {
        let input = self.rec.inputs.get(self.played).copied()?;
        self.played += 1;
        Some(input)
    }

    /// Has every recorded input been played?
    pub fn finished(&self) -> bool {
        self.played == self.rec.inputs.len()
    }

    /// The frame the replay holds on, the first time it is asked once every input is played;
    /// `None` before and after.
    pub fn just_finished(&mut self) -> Option<usize> {
        (self.finished() && !core::mem::replace(&mut self.told, true)).then_some(self.played)
    }
}
