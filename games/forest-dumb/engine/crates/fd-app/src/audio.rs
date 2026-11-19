//! The translated program's sound (translator DESIGN §2.10, T38): each frame's Paula writes,
//! at the beam positions the machine made them at, played by the toolkit's mixer
//! (`amiga_paula::Paula`) on the frame's chip RAM, as Superfrog's app plays its engine's.
//!
//! In the library, not the binary, so a test makes the same samples from a headless run that the
//! app makes from its own: the app's capture is then compared sample for sample.

use amiga_paula::{ChipMemory, LINES_PER_FRAME, Paula, PaulaWrite};
use fd_translated::FrameOut;

/// The mixer's rate when no audio device plays (a scripted run, or none present): what the app's
/// capture and the wall clock count in.
pub const SILENT_RATE: u32 = 48_000;

/// Chip RAM as the channels' DMA reads it: the frame's end, the only moment the app has it. The
/// samples a channel plays are data the program loaded long before, so reading them at the frame's
/// end rather than at each fetch's slot changes nothing the program does in play.
struct Chip<'a>(&'a [u8]);

impl ChipMemory for Chip<'_> {
    fn chip_word(&self, addr: u32) -> u16 {
        let a = addr as usize & !1;
        match self.0.get(a..a + 2) {
            Some(w) => u16::from_be_bytes([w[0], w[1]]),
            None => 0,
        }
    }
}

/// The mixer of one run of the translated program, from its first frame.
pub struct Sound {
    paula: Paula,
}

impl Sound {
    /// At `rate` host samples a second, DMACON as the machine starts with it (`dmacon`, its
    /// register value: `Machine::dmacon`). From power-on Kickstart leaves no channel on (T47's
    /// `$03f0`), so every channel starts idle and silent.
    pub fn new(rate: u32, dmacon: u16) -> Sound {
        // A set/clear word: bit 15 sets the bits given.
        Sound {
            paula: Paula::booted(rate, 0x8000 | dmacon, LINES_PER_FRAME),
        }
    }

    /// One frame: `writes`, `(register, word)` each at `beam` `(vpos, hpos)` in the same order,
    /// on chip RAM `chip`. Its stereo samples are appended to `out`.
    pub fn frame(
        &mut self,
        writes: &[(u16, u16)],
        beam: &[(u16, u16)],
        chip: &[u8],
        out: &mut Vec<[i16; 2]>,
    ) {
        assert_eq!(writes.len(), beam.len(), "a beam position a write");
        let writes: Vec<PaulaWrite> = writes
            .iter()
            .zip(beam)
            .map(|(&(reg, value), &(vpos, hpos))| PaulaWrite {
                reg,
                value,
                hpos,
                vpos,
            })
            .collect();
        self.paula.frame(&writes, &Chip(chip), out);
    }

    /// One frame of the translated program, as it handed it to `run`'s callback.
    pub fn frame_out(&mut self, f: &FrameOut, out: &mut Vec<[i16; 2]>) {
        let writes: Vec<(u16, u16)> = f.paula.iter().map(|&(_, r, v)| (r, v)).collect();
        self.frame(&writes, &f.paula_beam, f.chip, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square wave in chip RAM at `$1000`, and the writes that play it on channel 0 at full
    /// volume from line 10, then stop it at line 200.
    fn tune() -> (Vec<u8>, Vec<PaulaWrite>) {
        let mut chip = vec![0u8; 512 << 10];
        for i in 0..16 {
            chip[0x1000 + i] = if i < 8 { 0x7f } else { 0x80 };
        }
        let at = |vpos, hpos, reg, value| PaulaWrite {
            reg,
            value,
            hpos,
            vpos,
        };
        let w = vec![
            at(10, 20, 0xA0, 0x0000), // AUD0LCH
            at(10, 24, 0xA2, 0x1000), // AUD0LCL
            at(10, 28, 0xA4, 8),      // AUD0LEN, words
            at(10, 32, 0xA6, 400),    // AUD0PER
            at(10, 36, 0xA8, 64),     // AUD0VOL
            at(10, 40, 0x96, 0x8201), // DMACON: master and channel 0 on
            at(200, 0, 0x96, 0x0001), // channel 0 off
        ];
        (chip, w)
    }

    #[test]
    fn a_frames_writes_make_the_samples_the_mixer_makes_given_them_directly() {
        let (chip, w) = tune();
        let mut direct = Paula::booted(SILENT_RATE, 0x8000 | 0x03f0, LINES_PER_FRAME);
        let mut sound = Sound::new(SILENT_RATE, 0x03f0);
        let regs: Vec<(u16, u16)> = w.iter().map(|w| (w.reg, w.value)).collect();
        let beam: Vec<(u16, u16)> = w.iter().map(|w| (w.vpos, w.hpos)).collect();
        // Two frames: the writes, then a frame with none, so the state carried over counts too.
        for writes in [&w[..], &[]] {
            let (mut ours, mut theirs) = (Vec::new(), Vec::new());
            let (r, b) = if writes.is_empty() {
                (&[][..], &[][..])
            } else {
                (&regs[..], &beam[..])
            };
            sound.frame(r, b, &chip, &mut ours);
            direct.frame(writes, &Chip(&chip), &mut theirs);
            assert_eq!(ours, theirs);
        }
    }

    #[test]
    fn the_tune_is_heard_and_where_the_writes_put_it() {
        let (chip, w) = tune();
        let regs: Vec<(u16, u16)> = w.iter().map(|w| (w.reg, w.value)).collect();
        let beam: Vec<(u16, u16)> = w.iter().map(|w| (w.vpos, w.hpos)).collect();
        let mut sound = Sound::new(SILENT_RATE, 0x03f0);
        let mut out = Vec::new();
        sound.frame(&regs, &beam, &chip, &mut out);
        // About host_rate / 50 samples a frame.
        assert!((950..=970).contains(&out.len()), "{}", out.len());
        let loud = |s: &[[i16; 2]]| s.iter().any(|[l, r]| *l != 0 && *r == 0);
        // Channel 0 is left: heard from line 10 to line 200, so in the middle of the frame and
        // not (beyond the resampler's tail) at its very start or end.
        let line = |l: usize| l * out.len() / LINES_PER_FRAME as usize;
        assert!(loud(&out[line(30)..line(180)]), "the tune plays mid-frame");
        assert!(!loud(&out[..line(5)]), "nothing before the writes");
        // Off, a channel holds its last level (Paula's DAC keeps it): the wave stops, flat.
        let after = &out[line(240)..];
        assert!(after.iter().all(|s| *s == after[0]), "the wave stops when channel 0 is off");
    }

    #[test]
    fn chip_reads_past_the_end_are_zero_and_odd_addresses_read_their_word() {
        let c = Chip(&[0x12, 0x34, 0x56]);
        assert_eq!(c.chip_word(0), 0x1234);
        assert_eq!(c.chip_word(1), 0x1234);
        assert_eq!(c.chip_word(2), 0);
    }
}
