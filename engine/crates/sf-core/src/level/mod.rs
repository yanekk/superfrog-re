//! The level scene (docs/08 "Main loop and interrupt", docs/09): what runs while a level plays,
//! and the object dispatch that items, enemies and bosses register into (T15–T18).
//!
//! The original splits a level's work in two. Every frame the vertical-blank interrupt runs the
//! frog, the camera, the clock, scrolling and sound ([`vbl`]); every second frame the main loop
//! runs one 25 Hz pass over the objects and their drawing ([`pass`]). [`LevelCtx`] is what a
//! ported routine works on: the original's memory as the engine models it, the hardware the frame
//! reads, and the sound driver.

pub mod collision;
pub mod load;

use sf_paula::PaulaWrite;

use crate::audio::{AudioDriver, SoundData};
use crate::input::{FrameInput, Joy};
use crate::layout::h0;
use crate::mem::Bus;

/// `attract_mode` (h0+$18): -1 while the attract demo runs; sound is silent then.
pub const ATTRACT_MODE: u32 = h0(0x18);

/// What the custom chips and CIAs answer this frame: the original reads the joystick and the fire
/// button itself, wherever it needs them (docs/08 "Input"); within a frame they hold still.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hw {
    /// `JOY1DAT` (`$DFF00C`): the joystick in port 2, as Denise encodes it.
    pub joy1dat: u16,
    /// Port 2's fire button: CIA-A PRA (`$BFE001`) bit 7 reads 0 while it is held.
    pub fire: bool,
}

impl Hw {
    pub fn of(input: FrameInput) -> Hw {
        Hw {
            joy1dat: joy1dat(input.joy),
            fire: input.joy.contains(Joy::FIRE),
        }
    }

    /// CIA-A PRA as the game tests it: bit 7 low while port 2's fire is held, bit 6 low for
    /// port 1's (the mouse button).
    pub fn ciaa_pra(&self) -> u8 {
        if self.fire { 0x7f } else { 0xff }
    }
}

/// `JOY1DAT` for a joystick held as `j`: right sets bit 1, left bit 9; bit 0 is bit 1 XOR down and
/// bit 8 is bit 9 XOR up (the quadrature counters a joystick's switches drive).
pub fn joy1dat(j: Joy) -> u16 {
    let (r, l) = (j.contains(Joy::RIGHT) as u16, j.contains(Joy::LEFT) as u16);
    let (d, u) = (j.contains(Joy::DOWN) as u16, j.contains(Joy::UP) as u16);
    r << 1 | (r ^ d) | l << 9 | (l ^ u) << 8
}

/// What a ported level routine works on.
pub struct LevelCtx<'a> {
    pub bus: Bus<'a>,
    pub hw: Hw,
    pub audio: &'a mut AudioDriver,
    pub sound: &'a SoundData,
    /// The frame's Paula writes, in order.
    pub paula: &'a mut Vec<PaulaWrite>,
}

impl LevelCtx<'_> {
    /// `sfx_play` (h0+$12a72): effect `effect` on `channel`, unless the attract demo runs.
    pub fn sfx(&mut self, effect: u8, channel: u8) {
        if self.bus.l(ATTRACT_MODE) == 0 {
            self.audio
                .play_sfx(self.sound, effect, channel, self.paula);
        }
    }

    /// `sample_play` (h0+$148b8): stream sample `k` on channel 3, unless the attract demo runs.
    pub fn sample(&mut self, k: usize) {
        if self.bus.l(ATTRACT_MODE) == 0 {
            self.audio.play_sample(self.sound, k, self.paula);
        }
    }

    /// `rng_range` (h0+$160c0): a number in `0..=n`.
    pub fn rng_range(&mut self, n: u16) -> u16 {
        let mut r = self.bus.st.rng();
        let v = r.range(n);
        self.bus.st.set_rng(r);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joy1dat_encodes_directions_as_denise_does() {
        assert_eq!(joy1dat(Joy::NONE), 0);
        assert_eq!(joy1dat(Joy::RIGHT), 0x0003);
        assert_eq!(joy1dat(Joy::LEFT), 0x0300);
        assert_eq!(joy1dat(Joy::DOWN), 0x0001);
        assert_eq!(joy1dat(Joy::UP), 0x0100);
        assert_eq!(joy1dat(Joy::RIGHT | Joy::DOWN), 0x0002);
        assert_eq!(joy1dat(Joy::LEFT | Joy::UP), 0x0200);
        assert_eq!(Hw::of(FrameInput::joy(Joy::FIRE)).ciaa_pra(), 0x7f);
    }
}
