//! The audio channels' DMA slots on the machine (translator T52): which of `$0D`, `$0F`, `$11`
//! and `$13` the channels take, line by line, so the CPU and the blitter find them taken. Odd
//! slots, about one a line while the music plays: the CPU rarely wants them, but a blit with
//! BLTPRI set wants every free slot, so each costs it one.
//!
//! The channels are the toolkit's `amiga_paula::AudioDma` (vAmiga's audio state machine), run on
//! the frame's Paula writes at their beam positions. Its run is a prediction, like the copper's:
//! from the frame's start with the writes made so far, as if no more came. A write changes only
//! the slots after its line, so a slot is looked up in the last run unless a write since then
//! could have changed it; then the frame is run again (a frame of Paula's clocks, about 0.3 ms;
//! the music player's writes come in a burst on one line, so about one rerun a frame).

use amiga_paula::{AudioDma, PaulaWrite};

/// Where the oracle's bus has an audio fetch, after `AudioDma`'s position for it: a line later.
/// Measured on frame 6777 of `camera-bottom-right`: 231 of 232 lines' audio slots equal the
/// oracle's DMA debugger's so shifted, 7 unshifted. `AudioDma`'s line is the one its DAC timing
/// was matched on, and the first game's bus timing is fitted on it as it is, so the toolkit's
/// model is left alone.
pub const AUDIO_LAG: u32 = amiga_clock::LINE_SLOTS as u32;

const LINE: u32 = amiga_clock::LINE_SLOTS as u32;

#[derive(Clone, Debug)]
pub(crate) struct Audio {
    /// The channels as the current frame began.
    state: AudioDma,
    /// This frame's Paula writes, at their beam positions, in order.
    writes: Vec<PaulaWrite>,
    /// The slots the channels take this frame, as DMA slots from its start, ascending: the last
    /// run's, with `carried` first.
    slots: Vec<u32>,
    /// The last frame's fetches that `AUDIO_LAG` moves into this frame's first line.
    carried: Vec<u32>,
    /// The first slot a write since the last run may have changed: looking one up from there runs
    /// the frame again.
    stale_from: Option<u32>,
    /// The frame's length in lines; `None` for a frame Paula's model does not run (not PAL).
    lines: Option<u32>,
}

impl Audio {
    /// Power-on: every channel idle, DMA off.
    pub fn new(frame_lines: u16) -> Audio {
        Audio {
            state: AudioDma::new(),
            writes: Vec::new(),
            slots: Vec::new(),
            carried: Vec::new(),
            stale_from: Some(0),
            lines: matches!(frame_lines, 312 | 313).then_some(frame_lines as u32),
        }
    }

    /// The first frame of a machine started part way through the program (a snapshot): the
    /// channels' state is unknown, so those DMACON has on start from idle, as setting their bits
    /// would start them.
    pub fn start(&mut self, dmacon: u16) {
        self.state = AudioDma::booted(0x8000 | dmacon);
        self.writes.clear();
        self.carried.clear();
        self.stale_from = Some(0);
    }

    /// A Paula write at beam `(vpos, hpos)`.
    pub fn write(&mut self, reg: u16, value: u16, (vpos, hpos): (u16, u16)) {
        self.writes.push(PaulaWrite {
            reg,
            value,
            hpos,
            vpos,
        });
        // It changes the model's fetches on its line at the earliest, which the bus has a line on.
        let from = (vpos as u32 * LINE + AUDIO_LAG).min(self.stale_from.unwrap_or(u32::MAX));
        self.stale_from = Some(from);
    }

    /// Whether an audio fetch takes DMA slot `slot` from the frame's start.
    pub fn takes(&mut self, slot: u32) -> bool {
        if self.stale_from.is_some_and(|s| slot >= s) {
            self.run();
        }
        self.slots.binary_search(&slot).is_ok()
    }

    /// The frame ends: its writes become the channels' state, and the fetches past its end are
    /// the next frame's first.
    pub fn end_frame(&mut self) {
        if self.stale_from.is_some() {
            self.run();
        }
        if let Some(lines) = self.lines {
            let end = lines * LINE;
            self.carried = self
                .slots
                .iter()
                .filter(|&&s| s >= end)
                .map(|&s| s - end)
                .collect();
            self.state.frame(&self.writes, lines);
        }
        self.writes.clear();
        self.stale_from = Some(0);
    }

    fn run(&mut self) {
        self.stale_from = None;
        self.slots.clear();
        self.slots.extend_from_slice(&self.carried);
        if let Some(lines) = self.lines {
            let mut a = self.state.clone();
            let fetched = a.frame(&self.writes, lines);
            self.slots
                .extend(fetched.iter().map(|&(slot, _)| slot + AUDIO_LAG));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Channel 0 on with a period of 124: a word every 248 clocks, so about one fetch a line.
    fn playing() -> Audio {
        let mut a = Audio::new(313);
        a.write(0xA4, 100, (0, 2)); // AUD0LEN
        a.write(0xA6, 124, (0, 4)); // AUD0PER
        a.write(0x96, 0x8201, (0, 6)); // DMACON: DMAEN, AUD0EN
        a
    }

    #[test]
    fn a_playing_channel_takes_its_slot_a_line_after_the_models_fetch() {
        let mut a = playing();
        let model: Vec<u32> = {
            let mut m = AudioDma::new();
            m.frame(&a.writes, 313).iter().map(|f| f.0).collect()
        };
        assert!(model.len() > 200, "about one fetch a line: {}", model.len());
        for &s in &model {
            assert_eq!(s % LINE, 0x0D, "channel 0's slot");
            assert!(a.takes(s + AUDIO_LAG), "slot {s} a line on");
        }
        assert!(!a.takes(model[0]), "not where the model has it");
    }

    #[test]
    fn a_write_changes_only_the_slots_after_its_line() {
        let mut a = playing();
        let line = |l: u32| l * LINE + 0x0D;
        let before: Vec<bool> = (100..110).map(|l| a.takes(line(l))).collect();
        assert!(before.iter().any(|&t| t));
        // DMACON: AUD0EN off on line 104; lines up to 105 (the model's 104 a line on) keep theirs.
        a.write(0x96, 0x0001, (104, 0x20));
        for l in 100..=105 {
            assert_eq!(a.takes(line(l)), before[(l - 100) as usize], "line {l}");
        }
        assert!((107..200).all(|l| !a.takes(line(l))), "off from there");
    }

    #[test]
    fn the_last_lines_fetches_carry_into_the_next_frame() {
        let mut a = playing();
        let last = 312 * LINE + 0x0D;
        let carried = a.takes(last + AUDIO_LAG) as usize;
        a.end_frame();
        assert_eq!(a.takes(0x0D) as usize, carried);
    }
}
