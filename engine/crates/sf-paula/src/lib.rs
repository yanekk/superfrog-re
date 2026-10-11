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
