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
//!
//! `AudioDma`'s slots are the oracle's bus owners' slot for slot (toolkit `contention.rs`,
//! `audio_dma_takes_the_oracles_audio_slots`), so they are taken where the model puts them.

use amiga_paula::{AudioDma, PaulaWrite};

const LINE: u32 = amiga_clock::LINE_SLOTS as u32;

#[derive(Clone, Debug)]
pub(crate) struct Audio {
    /// The channels as the current frame began.
    state: AudioDma,
    /// This frame's Paula writes, at their beam positions, in order.
    writes: Vec<PaulaWrite>,
    /// The slots the channels take this frame, as DMA slots from its start, ascending: the last
    /// run's.
    slots: Vec<u32>,
    /// The first slot a write since the last run may have changed: looking one up from there runs
    /// the frame again.
    stale_from: Option<u32>,
    /// The frame's length in lines; `None` for a frame Paula's model does not run (not PAL).
    lines: Option<u32>,
    /// The next frame's slots, if no write comes before it: for the bus reaching that frame before
    /// the host has started it (a CPU waiting out a blit across the frame's end, T54).
    next: Option<Vec<u32>>,
}

impl Audio {
    /// Power-on: every channel idle, DMA off.
    pub fn new(frame_lines: u16) -> Audio {
        Audio {
            state: AudioDma::new(),
            writes: Vec::new(),
            slots: Vec::new(),
            stale_from: Some(0),
            lines: matches!(frame_lines, 312 | 313).then_some(frame_lines as u32),
            next: None,
        }
    }

    /// The first frame of a machine started part way through the program (a snapshot): the
    /// channels' state is unknown, so those DMACON has on start from idle, as setting their bits
    /// would start them.
    pub fn start(&mut self, dmacon: u16) {
        self.state = AudioDma::booted(0x8000 | dmacon);
        self.writes.clear();
        self.stale_from = Some(0);
        self.next = None;
    }

    /// A Paula write at beam `(vpos, hpos)`.
    pub fn write(&mut self, reg: u16, value: u16, (vpos, hpos): (u16, u16)) {
        self.writes.push(PaulaWrite {
            reg,
            value,
            hpos,
            vpos,
        });
        // It changes the model's fetches on its line at the earliest.
        let from = (vpos as u32 * LINE).min(self.stale_from.unwrap_or(u32::MAX));
        self.stale_from = Some(from);
        self.next = None;
    }

    /// Whether an audio fetch takes DMA slot `slot` of the next frame, no write coming before it.
    pub fn takes_next(&mut self, slot: u32) -> bool {
        let (state, writes, lines) = (&self.state, &self.writes, self.lines);
        let next = self.next.get_or_insert_with(|| match lines {
            Some(lines) => {
                let mut a = state.clone();
                a.frame(writes, lines);
                a.frame(&[], lines).iter().map(|&(slot, _)| slot).collect()
            }
            None => Vec::new(),
        });
        next.binary_search(&slot).is_ok()
    }

    /// Whether an audio fetch takes DMA slot `slot` from the frame's start.
    pub fn takes(&mut self, slot: u32) -> bool {
        if self.stale_from.is_some_and(|s| slot >= s) {
            self.run();
        }
        self.slots.binary_search(&slot).is_ok()
    }

    /// The frame ends: its writes become the channels' state.
    pub fn end_frame(&mut self) {
        if let Some(lines) = self.lines {
            self.state.frame(&self.writes, lines);
        }
        self.writes.clear();
        self.stale_from = Some(0);
        self.next = None;
    }

    fn run(&mut self) {
        self.stale_from = None;
        self.slots.clear();
        if let Some(lines) = self.lines {
            let mut a = self.state.clone();
            let fetched = a.frame(&self.writes, lines);
            self.slots.extend(fetched.iter().map(|&(slot, _)| slot));
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

    /// The next frame's slots, looked up before it starts (T54), are those the frame has once
    /// it does, with no write between.
    #[test]
    fn the_next_frames_slots_are_the_ones_it_takes() {
        let mut a = playing();
        let ahead: Vec<u32> = (0..313 * LINE).filter(|&s| a.takes_next(s)).collect();
        a.end_frame();
        let then: Vec<u32> = (0..313 * LINE).filter(|&s| a.takes(s)).collect();
        assert!(then.len() > 250, "about one fetch a line: {}", then.len());
        assert_eq!(ahead, then);
        // A write drops the lookahead: the next frame starts from the channels as it leaves them.
        let mut a = playing();
        assert!(a.takes_next(ahead[0]));
        a.write(0x96, 0x0001, (300, 0)); // AUD0EN off
        assert!((0..313 * LINE).all(|s| !a.takes_next(s)), "the channel is off next frame");
    }

    #[test]
    fn a_playing_channel_takes_the_models_slots() {
        let mut a = playing();
        let model: Vec<u32> = {
            let mut m = AudioDma::new();
            m.frame(&a.writes, 313).iter().map(|f| f.0).collect()
        };
        assert!(model.len() > 200, "about one fetch a line: {}", model.len());
        assert!(model.iter().all(|s| s % LINE == 0x0D), "channel 0's slot");
        let taken: Vec<u32> = (0..313 * LINE).filter(|&s| a.takes(s)).collect();
        assert_eq!(taken, model, "the model's slots, no other");
    }

    #[test]
    fn a_write_changes_only_the_slots_after_its_line() {
        let mut a = playing();
        let line = |l: u32| l * LINE + 0x0D;
        let before: Vec<bool> = (100..110).map(|l| a.takes(line(l))).collect();
        assert!(before.iter().any(|&t| t));
        // DMACON: AUD0EN off on line 104, past its slot: lines up to 104 keep theirs.
        a.write(0x96, 0x0001, (104, 0x20));
        for l in 100..=104 {
            assert_eq!(a.takes(line(l)), before[(l - 100) as usize], "line {l}");
        }
        assert!((106..200).all(|l| !a.takes(line(l))), "off from there");
    }
}
