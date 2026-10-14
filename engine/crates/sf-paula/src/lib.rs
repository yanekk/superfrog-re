//! Pure: Paula register writes -> PCM (DESIGN §3.2).
//!
//! [`Paula::frame`] runs the four channels for one PAL frame at the Paula clock, from the frame's
//! register writes and the chip memory the channels' DMA reads ([`channel`], integer only), then
//! mixes them with the hardware's pairing (channels 0 and 3 left, 1 and 2 right) and resamples to
//! the host rate ([`resample`], band-limited).
//!
//! Each write lands at its beam position (`vpos`, `hpos`). The oracle's writes carry the original's
//! positions, which makes the channels' DAC values identical to the reference's clock for clock
//! (`sf-conform/tests/paula_vs_oracle.rs`); writes made with no position (0, 0) all land at the start of the frame.

mod channel;
mod resample;
pub mod write;

pub use channel::{CLOCKS_PER_FRAME, CLOCKS_PER_LINE, LINES_PER_FRAME};
pub use resample::HALF_WIDTH as RESAMPLER_DELAY;
pub use write::PaulaWrite;

/// The PAL colour clock, which Paula's period counters count (Hz).
pub const PAULA_CLOCK_PAL: u32 = 3_546_895;

/// Where the channels' DMA reads sample data: the engine's view of chip RAM (`sf-core` provides it
/// from `GameData`), so the mixer never owns game data.
pub trait ChipMemory {
    /// The big-endian word at the even chip address `addr` (below 512 KB).
    fn chip_word(&self, addr: u32) -> u16;
}

/// The channels' DMA alone: which of their slots (`$0D`, `$0F`, `$11`, `$13`) Agnus serves, frame by
/// frame, from the frames' register writes (the CPU's and the blitter's bus timing needs them, T96).
///
/// The same state machine as [`Paula`]'s, run on zeroed chip memory: the slots do not depend on the
/// samples, except with ADKCON's attach modes (a channel's words then set the next one's period or
/// volume), which the timing this serves does not meet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioDma {
    chip: channel::Chip,
    taps: [Vec<i16>; 4],
}

impl Default for AudioDma {
    fn default() -> AudioDma {
        AudioDma::new()
    }
}

impl AudioDma {
    /// Power-on: every channel idle, DMA off.
    pub fn new() -> AudioDma {
        AudioDma {
            chip: channel::Chip::new(),
            taps: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        }
    }

    /// As the game's boot leaves Agnus: DMA's master switch on (the original's DMACON `$8200` in
    /// its frame 58, its trackloader's `$8210` after; the engine loads without them), every channel
    /// idle. The sound driver then turns channels on with their bits alone (`$8001`).
    pub fn booted() -> AudioDma {
        let mut a = AudioDma::new();
        a.chip.set_dmacon(0x8200);
        a
    }

    /// Run one frame of `lines` lines on its writes, as [`Paula::frame`]: the slots the channels'
    /// DMA took in it, as (clock in the frame, `vpos * 227 + hpos`; channel), in order.
    pub fn frame(&mut self, writes: &[PaulaWrite], lines: u32) -> &[(u32, u8)] {
        struct Zero;
        impl ChipMemory for Zero {
            fn chip_word(&self, _: u32) -> u16 {
                0
            }
        }
        self.chip.run_frame(writes, lines, &Zero, &mut self.taps);
        // Only the slots are wanted: no state is kept in the DAC values, and an empty list clones
        // and hashes for nothing.
        for t in &mut self.taps {
            t.clear();
        }
        &self.chip.fetched
    }
}

/// The four audio channels, their DMA, and the resampler to the host rate.
pub struct Paula {
    chip: channel::Chip,
    resampler: resample::Resampler,
    taps: [Vec<i16>; 4],
    lines: u32,
}

impl Paula {
    /// Power-on: silent, DMA off. `host_rate` is the output's sample rate in Hz.
    pub fn new(host_rate: u32) -> Paula {
        Paula {
            chip: channel::Chip::new(),
            resampler: resample::Resampler::new(host_rate),
            taps: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            lines: LINES_PER_FRAME,
        }
    }

    /// The length of the frames from now on: [`LINES_PER_FRAME`] (a long frame, the default) or one
    /// fewer (short). Agnus's LOF bit decides it: it alternates while BPLCON0's LACE bit is set and
    /// keeps the last value when LACE is cleared, so Superfrog runs whole stretches of short frames.
    pub fn set_frame_lines(&mut self, lines: u32) {
        assert!(
            lines == LINES_PER_FRAME || lines == LINES_PER_FRAME - 1,
            "a PAL frame of {lines} lines"
        );
        self.lines = lines;
    }

    /// Run one PAL frame (`lines * 227` Paula clocks, [`Paula::set_frame_lines`]) and append its host-rate stereo
    /// samples, `[left, right]`, to `out`: `host_rate / 49.92` of them for a long frame, give or take one, running
    /// [`RESAMPLER_DELAY`] samples late. Full scale is two channels at sample -128, volume 64.
    pub fn frame(&mut self, writes: &[PaulaWrite], chip: &dyn ChipMemory, out: &mut Vec<[i16; 2]>) {
        self.chip
            .run_frame(writes, self.lines, chip, &mut self.taps);
        let [t0, t1, t2, t3] = &self.taps;
        let levels =
            (0..t0.len()).map(|i| [t0[i] as i32 + t3[i] as i32, t1[i] as i32 + t2[i] as i32]);
        self.resampler.push(levels, out);
    }

    /// Each channel's DAC value at every Paula clock of the last frame: the sample byte times the
    /// volume (0..=64), one per clock of the frame, clock `vpos * 227 + hpos`.
    pub fn channel_taps(&self) -> [&[i16]; 4] {
        [&self.taps[0], &self.taps[1], &self.taps[2], &self.taps[3]]
    }
}
