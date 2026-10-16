//! Sound effects and the streamed sample (docs/11-sound.md): `sfx_bank_init` (`h0+$12a26`),
//! `sfx_play` (`$12a72`), `sfx_tick` (`$12c60`), `sample_play` (`$148b8`) and `sample_stream`
//! (`$147e4`).
//!
//! An effect is a priority word and a list of 18-byte steps; each step names a sample, a duration,
//! and a period and a volume ramp, and says what follows it. A channel an effect holds is marked in
//! the replayer's "free" bytes, so music leaves it alone until the effect ends.

use super::{
    AudioDriver, BANK_ADDR, DMACON, INTENA, PaulaWrite, STREAM_BUFFER, SoundData, clr, wr,
};

/// Where channel `n`'s record starts in the effect block (`$12f62`): channel 0's is 38 bytes,
/// the others 52.
const fn rec(n: usize) -> usize {
    [0, 0x26, 0x5a, 0x8e][n]
}
const STEAL: usize = 0xc2;

// Record fields.
const ACTIVE: usize = 0;
/// Byte 1: bit 0 active, bit 1 period ramp on, bit 2 volume ramp on, bit 4 period ramp done,
/// bit 5 volume ramp done, bit 7 the next tick starts the step's sample.
const FLAGS: usize = 1;
const PRIORITY: usize = 2;
const PERIOD: usize = 4;
const VOLUME: usize = 6;
const EFFECT: usize = 8;
const DURATION: usize = 0xc;
const ID: usize = 0xe;
/// A ramp: delta, steps, steps left, sign (period at `$10`, volume at `$18`).
const PER_RAMP: usize = 0x10;
const VOL_RAMP: usize = 0x18;
const PER_STEP: usize = 0x20;
const VOL_STEP: usize = 0x22;
const STEP: usize = 0x24;

// A step: flags, sample, duration, then period from/to/steps, volume from/to/steps, sample offset.
const STEP_LEN: u32 = 18;

/// The replayer's "free" byte for channel `n` (`$145b2 + 3 - n`).
const fn free_byte(n: usize) -> usize {
    0xd6 + 3 - n
}

const SAMPLES: usize = 0;
const EFFECTS: usize = 0x400;

/// `mulu.w` then `divu.w` of the interpolation, exactly as the 68000 leaves the low word.
fn ramp_value(delta: u16, steps: u16, left: u16) -> u16 {
    let p1 = steps as u32 * delta as u32;
    let p2 = left as u32 * delta as u32;
    let d1 = (p1 & 0xffff_0000) | (p1 as u16).wrapping_sub(p2 as u16) as u32;
    let q = d1 / steps as u32;
    // On overflow divu leaves the dividend.
    if q > 0xffff { d1 as u16 } else { q as u16 }
}

impl AudioDriver {
    /// `sfx_bank_init`, with the bank already at [`BANK_ADDR`]: a pointer to each sample (up to the
    /// `$ffffffff` after the last; entries past it keep what they held) and to each of 256 effects;
    /// the module goes after the last effect.
    pub fn sfx_bank_init(&mut self, data: &SoundData) {
        let mut a = BANK_ADDR + 0x400;
        for i in 0..256 {
            self.tables.set_l(SAMPLES + i * 4, a);
            let len = self.mem.l(data, a);
            a = a.wrapping_add(len.wrapping_add(len)).wrapping_add(4);
            if self.mem.l(data, a) == 0xffff_ffff {
                break;
            }
        }
        a = a.wrapping_add(4);
        for i in 0..256 {
            self.tables.set_l(EFFECTS + i * 4, a);
            a = a.wrapping_add(self.mem.l(data, BANK_ADDR + i as u32 * 4));
        }
        self.module_addr = a;
    }

    fn sample_ptr(&self, i: u8) -> u32 {
        self.tables.l(SAMPLES + i as usize * 4)
    }

    fn step_addr(&self, effect: u32, index: u16) -> u32 {
        effect
            .wrapping_add(4)
            .wrapping_add((index.wrapping_mul(STEP_LEN as u16)) as i16 as i32 as u32)
    }

    /// Load a step's ramp (`$4`/`$a` of the step) into the record: start value, then delta, steps
    /// and sign when the ramp moves; steps left is always the step count minus one.
    fn load_ramp(
        &mut self,
        data: &SoundData,
        r: usize,
        step: u32,
        from: u32,
        value: usize,
        ramp: usize,
        on: u8,
    ) {
        let start = self.mem.w(data, step + from);
        let end = self.mem.w(data, step + from + 2);
        let steps = self.mem.w(data, step + from + 4);
        self.sfx.set_w(r + value, start);
        self.sfx.set_b(r + FLAGS, self.sfx.b(r + FLAGS) | on);
        let mut delta = end.wrapping_sub(start);
        if delta != 0 {
            let mut sign = 0;
            if (delta as i16) < 0 {
                delta = delta.wrapping_neg();
                sign = 1;
            }
            self.sfx.set_w(r + ramp, delta);
            self.sfx.set_w(r + ramp + 2, steps);
            self.sfx.set_w(r + ramp + 6, sign);
        }
        self.sfx.set_w(r + ramp + 4, steps.wrapping_sub(1));
    }

    /// `sfx_play` (attract mode is the caller's check).
    pub(crate) fn sfx_play(
        &mut self,
        data: &SoundData,
        id: u8,
        chan: u8,
        out: &mut Vec<PaulaWrite>,
    ) {
        if let Some(rest) = self.sfx_play_begin(data, id, chan, out) {
            self.sfx_play_rest(data, rest, out);
        }
    }

    /// `sfx_play` to its raster wait (h0+$12b20..$12b38): interrupts off (INTENA `$4000`), the
    /// beam waited on for three lines, interrupts on (`$c000`). What is left of it, if anything
    /// ([`AudioDriver::sfx_play_rest`]): a vertical blank the wait held off comes in between.
    pub(crate) fn sfx_play_begin(
        &mut self,
        data: &SoundData,
        id: u8,
        chan: u8,
        out: &mut Vec<PaulaWrite>,
    ) -> Option<SfxRest> {
        let n = chan as usize;
        if chan == 3 && self.stream.w(4) != 0xffff {
            self.sfx.set_b(STEAL, 0);
            return None;
        }
        let effect = self.tables.l(EFFECTS + id as usize * 4);
        let r = rec(n);
        if self.sfx.w(r + ACTIVE) != 0 {
            let playing = self.mem.w(data, self.sfx.l(r + EFFECT).wrapping_add(2));
            if (playing as i16) < (self.mem.w(data, effect.wrapping_add(2)) as i16) {
                self.sfx.set_b(STEAL, 0);
                return None;
            }
            if self.mem.w(data, effect) != 0 {
                self.sfx.set_b(STEAL, 1);
            }
        }
        self.p41.set_b(free_byte(n), id);
        let steal = self.sfx.b(STEAL) != 0;
        let reg = 0xa0 + chan as u16 * 16;
        if !steal {
            wr(out, DMACON, 1 << chan);
            wr(out, reg + 6, 1);
        }
        wr(out, INTENA, 0x4000);
        wr(out, INTENA, 0xc000);
        Some(SfxRest {
            id,
            chan,
            effect,
            steal,
        })
    }

    /// The rest of `sfx_play`, after its raster wait (h0+$12b40): the channel's record set up and
    /// the effect started.
    pub(crate) fn sfx_play_rest(&mut self, data: &SoundData, rest: SfxRest, out: &mut Vec<PaulaWrite>) {
        self.sfx_play_setup(data, rest);
        self.sfx_play_start(data, rest, out);
    }

    /// The rest's record set up (h0+$12b40..$12c12), up to its duration: a vertical blank coming
    /// here has `sfx_tick` see the new effect with no duration, so it ends it at once.
    pub(crate) fn sfx_play_setup(&mut self, data: &SoundData, rest: SfxRest) {
        self.sfx_play_wait_over(rest);
        self.sfx_play_record(data, rest);
    }

    /// The rest's first instruction (h0+$12b40, `clr.l $20(a2)`): the ramps' step indices
    /// cleared. The 68000 runs it before it takes the vertical blank the raster wait held off,
    /// so that blank's `sfx_tick` steps the old effect from index 0, and an index it advances
    /// there is the new effect's (T194: `enemies-w6-2-complete` 6658, the new effect's period
    /// starting at its second step).
    pub(crate) fn sfx_play_wait_over(&mut self, rest: SfxRest) {
        self.sfx.set_l(rec(rest.chan as usize) + PER_STEP, 0);
    }

    /// The rest's record set up after its first instruction (h0+$12b44..$12c12).
    pub(crate) fn sfx_play_record(&mut self, data: &SoundData, rest: SfxRest) {
        let SfxRest {
            id, chan, effect, ..
        } = rest;
        let r = rec(chan as usize);
        self.sfx.set_w(r + STEP, 0);
        self.sfx.set_w(r + ACTIVE, 1);
        self.sfx
            .set_w(r + PRIORITY, self.mem.w(data, effect.wrapping_add(2)));
        self.sfx.set_l(r + EFFECT, effect);
        self.sfx.set_w(r + DURATION, 0);
        self.sfx.set_w(r + ID, id as u16);
        let step = effect.wrapping_add(4);
        self.sfx.set_w(r + PER_RAMP + 2, 0);
        self.load_ramp(data, r, step, 4, PERIOD, PER_RAMP, 2);
        self.sfx.set_w(r + VOL_RAMP + 2, 0);
        self.load_ramp(data, r, step, 0xa, VOLUME, VOL_RAMP, 4);
    }

    /// The rest's end (h0+$12c12): the duration, then the effect started on Paula (the period and
    /// volume as the record holds them).
    pub(crate) fn sfx_play_start(&mut self, data: &SoundData, rest: SfxRest, out: &mut Vec<PaulaWrite>) {
        let SfxRest {
            chan,
            effect,
            steal,
            ..
        } = rest;
        let r = rec(chan as usize);
        let reg = 0xa0 + chan as u16 * 16;
        let step = effect.wrapping_add(4);
        let sample = self.sample_ptr(self.mem.b(data, step + 1));
        let off = (self.mem.w(data, step + 0x10) & 0xfffe) as u32;
        let start = sample.wrapping_add(4).wrapping_add(off);
        let len = self
            .mem
            .w(data, sample.wrapping_add(2))
            .wrapping_sub((off >> 1) as u16);
        self.sfx.set_w(r + DURATION, self.mem.w(data, step + 2));
        wr(out, reg + 6, self.sfx.w(r + PERIOD));
        wr(out, reg + 8, self.sfx.w(r + VOLUME));
        if !steal {
            wr(out, reg + 4, len);
            wr(out, reg, (start >> 16) as u16);
            wr(out, reg + 2, start as u16);
            wr(out, DMACON, 0x8000 | 1 << chan);
        }
        self.sfx.set_b(STEAL, 0);
    }

    /// The effects playing: channels whose record is active (what `sfx_tick` steps each frame).
    pub fn sfx_active(&self) -> u32 {
        (0..4).filter(|&n| self.sfx.w(rec(n) + ACTIVE) != 0).count() as u32
    }

    /// A sample streams on channel 3 (`sample_stream` steps it at the handler's start).
    pub fn streaming(&self) -> bool {
        self.stream.w(4) != 0xffff
    }

    /// `sfx_tick` (`$12c60`): each effect's step, channels 3 down to 0.
    pub fn sfx_tick(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        for n in (0..4).rev() {
            let r = rec(n);
            if self.sfx.w(r + ACTIVE) == 0 {
                continue;
            }
            let reg = 0xa0 + n as u16 * 16;
            let bit = 1u16 << n;
            let effect = self.sfx.l(r + EFFECT);
            let step = self.step_addr(effect, self.sfx.w(r + STEP));
            if self.sfx.b(r + FLAGS) & 0x80 != 0 {
                // The step's sample starts.
                self.sfx.set_b(r + FLAGS, self.sfx.b(r + FLAGS) & !0x80);
                let s = self.mem.b(data, step + 1);
                if s == 0xff {
                    continue;
                }
                let sample = self.sample_ptr(s);
                let off = self.mem.w(data, step + 0x10) & 0xfffe;
                let start = sample
                    .wrapping_add(4)
                    .wrapping_add(off as i16 as i32 as u32);
                let len = self
                    .mem
                    .w(data, sample.wrapping_add(2))
                    .wrapping_sub(off >> 1);
                self.sfx.set_w(r + DURATION, self.mem.w(data, step + 2));
                wr(out, reg + 4, len);
                wr(out, reg, (start >> 16) as u16);
                wr(out, reg + 2, start as u16);
                wr(out, reg + 8, self.sfx.w(r + VOLUME));
                wr(out, reg + 6, self.sfx.w(r + PERIOD));
                wr(out, DMACON, 0x8000 | bit);
            } else if (self.sfx.w(r + DURATION) as i16) > 0 {
                self.ramps(data, n, out);
            } else {
                let flags = self.mem.b(data, step);
                match flags & 3 {
                    0 => {
                        // The effect ends: the channel goes back to music.
                        wr(out, DMACON, bit);
                        wr(out, reg + 6, 1);
                        wr(out, reg + 8, 0);
                        self.sfx.set_l(r, 0);
                        self.sfx.set_l(r + PER_STEP, 0);
                        self.sfx.set_w(r + STEP, 0);
                        self.p41.set_b(free_byte(n), 0xff);
                        continue;
                    }
                    1 => self
                        .sfx
                        .set_w(r + STEP, self.sfx.w(r + STEP).wrapping_add(1)),
                    3 => self.sfx.set_w(r + STEP, 0),
                    _ => {}
                }
                if flags & 0x80 == 0 {
                    wr(out, DMACON, bit);
                    wr(out, reg + 6, 1);
                }
                self.sfx.set_b(r + FLAGS, self.sfx.b(r + FLAGS) | 0x80);
            }
        }
    }

    /// `$12d6a`: the period and volume ramps of one channel, then its duration counts down.
    fn ramps(&mut self, data: &SoundData, n: usize, out: &mut Vec<PaulaWrite>) {
        let r = rec(n);
        let reg = 0xa0 + n as u16 * 16;
        let effect = self.sfx.l(r + EFFECT);
        for (value, ramp, index, from, on, done, to) in [
            (PERIOD, PER_RAMP, PER_STEP, 4u32, 2u8, 0x10u8, reg + 6),
            (VOLUME, VOL_RAMP, VOL_STEP, 0xa, 4, 0x20, reg + 8),
        ] {
            let flags = self.sfx.b(r + FLAGS);
            if flags & done != 0 || flags & on == 0 {
                continue;
            }
            let step = self.step_addr(effect, self.sfx.w(r + index));
            if self.sfx.w(r + ramp + 2) != 0 {
                let mut v = ramp_value(
                    self.sfx.w(r + ramp),
                    self.sfx.w(r + ramp + 2),
                    self.sfx.w(r + ramp + 4),
                );
                if self.sfx.w(r + ramp + 6) != 0 {
                    v = v.wrapping_neg();
                }
                self.sfx
                    .set_w(r + value, v.wrapping_add(self.mem.w(data, step + from)));
            }
            wr(out, to, self.sfx.w(r + value));
            let left = self.sfx.w(r + ramp + 4).wrapping_sub(1);
            self.sfx.set_w(r + ramp + 4, left);
            if (left as i16) >= 0 {
                continue;
            }
            let mask = if value == PERIOD { 0xc } else { 0x30 };
            match self.mem.b(data, step) & mask {
                0 => {
                    self.sfx.set_b(r + FLAGS, self.sfx.b(r + FLAGS) | done);
                    continue;
                }
                0x4 | 0x10 => self
                    .sfx
                    .set_w(r + index, self.sfx.w(r + index).wrapping_add(1)),
                0x8 | 0x20 => {}
                _ => self.sfx.set_w(r + index, 0),
            }
            let next = self.step_addr(effect, self.sfx.w(r + index));
            self.sfx.set_w(r + ramp + 2, 0);
            self.load_ramp(data, r, next, from, value, ramp, on);
        }
        self.sfx
            .set_w(r + DURATION, self.sfx.w(r + DURATION).wrapping_sub(1));
    }

    /// `sample_play`: stream sample `k` (0..8) on channel 3 from the next frame, ending any effect
    /// there. Attract mode is the caller's check, as for [`AudioDriver::play_sfx`].
    pub fn play_sample(&mut self, data: &SoundData, k: usize, out: &mut Vec<PaulaWrite>) {
        let s = &data.stream[k];
        wr(out, INTENA, 0x4000);
        for o in 0..0x34 {
            self.sfx.set_b(rec(3) + o, 0);
        }
        self.p41.set_b(free_byte(3), 1);
        self.stream.set_l(0, s.addr);
        self.stream.set_w(4, (s.len as u16) >> 1);
        self.stream.set_w(6, s.volume);
        wr(out, INTENA, 0xc000);
    }

    /// `sample_stream` (`$147e4`): the next 160 bytes go to [`STREAM_BUFFER`] and play once on
    /// channel 3; when the sample runs out the channel goes back to music.
    pub fn stream_tick(&mut self, _data: &SoundData, out: &mut Vec<PaulaWrite>) {
        if self.stream.w(4) == 0xffff {
            return;
        }
        wr(out, 0xd6, 1);
        wr(out, DMACON, 8);
        self.stream.set_l(0, self.stream.l(0).wrapping_add(0xa0));
        let left = self.stream.w(4).wrapping_sub(0x50);
        self.stream.set_w(4, left);
        if (left as i16) < 0 {
            wr(out, DMACON, 8);
            wr(out, 0xd4, 2);
            // `clr.w $dff0d8` (h0+$14890), then `move.w #$ffff,...` ($33fc).
            clr(out, 0xd8, 0x33fc);
            self.stream.set_w(4, 0xffff);
            self.p41.set_b(free_byte(3), 0xff);
            return;
        }
        wr(out, 0xd0, (STREAM_BUFFER >> 16) as u16);
        wr(out, 0xd2, STREAM_BUFFER as u16);
        wr(out, 0xd8, self.stream.w(6));
        wr(out, 0xd4, 0x50);
        wr(out, DMACON, 0x8008);
        wr(out, 0xd6, 0x1bc);
    }
}

/// What is left of an `sfx_play` after its raster wait ([`AudioDriver::sfx_play_begin`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SfxRest {
    id: u8,
    chan: u8,
    effect: u32,
    steal: bool,
}
