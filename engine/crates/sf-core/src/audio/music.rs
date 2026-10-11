//! The Player 4.1A replayer as Superfrog modified it (docs/11-sound.md): `music_start`
//! (`h0+$13126`), `music_stop` (`$132a0`), `music_tick` (`$13424`: `p41_play` `$1343c`, then the
//! fade and shadow copy `$145b6`) and the DMA stage `$13cc4`.
//!
//! The replayer writes a channel's pointer, length, period and volume into a shadow block, not
//! Paula; `$145b6` copies the shadow to Paula for the channels no effect holds, after applying the
//! fade. Only DMACON and INTENA are written directly.

use super::{AudioDriver, DMACON, HEADER_LEN, INTENA, PaulaWrite, SoundData, wr};

/// A channel record in the replayer block: 44 bytes from `$144dc`.
const fn ch(n: usize) -> usize {
    n * 0x2c
}

// Channel record fields.
/// The row: byte 0 note×2 (bit 0 the instrument's top bit), word 0 bits 4..8 instrument×16,
/// bits 0..3 effect; byte 2 the parameter; byte 3 empty rows after it (negative: repeats).
const ROW: usize = 0;
const PARAM: usize = 2;
const COUNT: usize = 3;
const INST: usize = 4;
const PERIOD: usize = 0xc;
const VOLUME: usize = 0xe;
const FINETUNE: usize = 0x10;
const PORTA_TARGET: usize = 0x12;
const PORTA_SPEED: usize = 0x14;
const VIBRATO: usize = 0x16;
const TREMOLO: usize = 0x18;
const RETRIG: usize = 0x1a;
const NOTE: usize = 0x1c;
const LIVE: usize = 0x1e;
const STREAM: usize = 0x20;
const BLOCK: usize = 0x24;
const BLOCK_COUNT: usize = 0x28;
const DMA_BIT: usize = 0x2a;

/// The globals, `$1458c`.
const G: usize = 0xb0;
const TICK: usize = G;
const DMA_ON: usize = G + 2;
const DMA_PENDING: usize = G + 4;
const ROWS_LEFT: usize = G + 6;
const SPEED: usize = G + 8;
const FETCH_TICK: usize = G + 0xa;
const FETCH_AT_ROW: usize = G + 0xc;
const POSITION: usize = G + 0xe;
const FETCH_DELAY: usize = G + 0x12;
const ROW_DELAY: usize = G + 0x14;
const POSITIONS: usize = G + 0x16;
const PATTERNS: usize = G + 0x1a;
const MODULE: usize = G + 0x22;
/// `$145b2..$145b5`: `$ff` while music may use the channel; channel 3's byte comes first.
const FREE: usize = 0xd6;

// The fade and shadow block, `$146e0`.
const FADE: usize = 0;
const FADE_SPEED: usize = 1;
const FADE_COUNT: usize = 2;
const fn sh(n: usize) -> usize {
    4 + n * 16
}

// Tables in hunk 0.
const ARP_STEPS: u32 = 0x13548;
const PERIODS: u32 = 0x135a2;
const FINETUNES: u32 = 0x13f5e;
const VIBRATO_TABLE: u32 = 0x140bc;

fn sext(w: u16) -> u32 {
    w as i16 as i32 as u32
}

impl AudioDriver {
    pub(crate) fn free(&self, n: usize) -> bool {
        self.p41.b(FREE + 3 - n) == 0xff
    }

    fn period(&self, data: &SoundData, index: u16) -> u16 {
        data.h0_word(PERIODS.wrapping_add(sext(index)))
    }

    /// `music_start`: nothing if music is on; else, with interrupts off, `p41_init`.
    pub fn music_start(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        if self.music_on.w(0) != 0 {
            return;
        }
        wr(out, INTENA, 0x4000);
        self.p41_init(data, out);
        wr(out, INTENA, 0xc000);
    }

    /// `p41_init` (`$13144`).
    fn p41_init(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        // bset #1,$bfe001 (the LED filter off) is a CIA write, not Paula's.
        self.music_on.set_w(0, 0);
        self.shadow.set_w(FADE, 0);
        let base = self.module_addr;
        if self.mem.l(data, base) != u32::from_be_bytes(*b"P41A") {
            return;
        }
        for i in 0..HEADER_LEN {
            let v = self.mem.b(data, base + i);
            self.backup.set_b(i as usize, v);
        }
        self.music_on.set_w(0, 1);
        let h = base + 4;
        self.p41.set_l(MODULE, h);
        for f in [4, 8, 0xc] {
            let v = self.mem.l(data, h + f).wrapping_add(h);
            self.mem.set_l(h + f, v);
        }
        let samples = self.mem.l(data, h + 0xc);
        // dbra over the count byte minus one: a count of 0 relocates 65 536 entries.
        let count = (self.mem.b(data, h + 2) as u16).wrapping_sub(1) as u32 + 1;
        for i in 0..count {
            let e = h + 0x10 + i * 0x10;
            let v = self.mem.l(data, e).wrapping_add(samples);
            self.mem.set_l(e, v);
            let v = self.mem.l(data, e + 6).wrapping_add(samples);
            self.mem.set_l(e + 6, v);
        }
        for n in 0..4 {
            for o in 0..42 {
                self.p41.set_b(ch(n) + o, 0);
            }
        }
        let patterns = self.mem.l(data, h + 4);
        self.p41.set_l(PATTERNS, patterns);
        let mut pos = self.mem.l(data, h + 8);
        self.p41.set_l(POSITIONS, pos);
        let mut at = patterns;
        for n in 0..4 {
            let off = sext(self.mem.w(data, pos));
            pos += 2;
            if n < 3 {
                self.p41.set_l(ch(n) + STREAM, patterns.wrapping_add(off));
            } else {
                at = at.wrapping_add(off);
                self.p41.set_l(ch(n) + STREAM, at);
            }
        }
        self.p41.set_l(POSITION, pos);
        self.p41.set_w(ROWS_LEFT, 0x3f);
        self.p41.set_w(FETCH_AT_ROW, 0);
        self.p41.set_l(FETCH_DELAY, 0);
        self.p41.set_w(TICK, 0);
        wr(out, DMACON, 0x000f);
        for n in 0..4u16 {
            wr(out, 0xa8 + n * 16, 0);
        }
        self.p41.set_w(SPEED, 6);
        self.p41.set_w(FETCH_TICK, 5);
    }

    /// `music_stop` (`$132a0`): music and every effect stopped, Paula silenced, the module header
    /// put back as it was before `music_start` relocated it.
    pub fn music_stop(&mut self, _data: &SoundData, out: &mut Vec<PaulaWrite>) {
        wr(out, INTENA, 0x4000);
        self.music_on.set_w(0, 0);
        self.shadow.set_w(FADE, 0);
        // Thirteen longs from each record's start: channel 0's run reaches into channel 1's.
        for start in [0, 0x26, 0x5a, 0x8e] {
            for o in 0..0x34 {
                self.sfx.set_b(start + o, 0);
            }
        }
        wr(out, 0xd4, 2);
        wr(out, 0xd8, 0);
        self.stream.set_w(4, 0xffff);
        for i in 0..4 {
            self.p41.set_b(FREE + i, 0xff);
        }
        wr(out, DMACON, 0x000f);
        let to = self.module_addr;
        for i in 0..HEADER_LEN {
            self.mem.set_b(to + i, self.backup.b(i as usize));
        }
        wr(out, INTENA, 0xc000);
    }

    /// `$146ae`: start a fade, `word` = ceiling (high byte) and frames per step (low byte); every
    /// channel is handed back to music.
    pub fn start_fade(&mut self, word: u16) {
        self.shadow.set_w(FADE, word);
        for i in 0..4 {
            self.p41.set_b(FREE + i, 0xff);
        }
    }

    /// The fade `music_fade_wait` starts (`$2001`) before it waits up to 100 frames for the music
    /// to stop.
    pub fn fade_out(&mut self) {
        self.shadow.set_w(FADE, 0x2001);
    }

    /// `music_tick` (`$13424`): one replayer tick, then the fade and the shadow copy to Paula.
    pub fn music_tick(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        if self.music_on.w(0) == 0 {
            return;
        }
        self.p41_play(data, out);
        self.shadow_to_paula(data, out);
    }

    /// `p41_play` (`$1343c`).
    fn p41_play(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        let tick = self.p41.w(TICK).wrapping_add(1);
        if tick == self.p41.w(SPEED) {
            self.play_row(data, out);
            return;
        }
        self.p41.set_w(TICK, tick);
        for n in 0..4 {
            if self.p41.w(ch(n) + LIVE) == 0 || !self.free(n) {
                continue;
            }
            self.tick_effect(data, n, out);
        }
        if tick == self.p41.w(FETCH_TICK) {
            self.fetch_rows(data);
        }
    }

    /// The effects that run on the ticks between rows (`p41_fx_tab`, `$13492`).
    fn tick_effect(&mut self, data: &SoundData, n: usize, out: &mut Vec<PaulaWrite>) {
        let c = ch(n);
        let s = sh(n);
        let param = self.p41.b(c + PARAM);
        match self.p41.w(c + ROW) & 0xf {
            1 => {
                let p = self.p41.w(c + PERIOD).wrapping_sub(param as u16);
                self.p41.set_w(c + PERIOD, p);
                if 0x71 > p as i16 {
                    self.p41.set_w(c + PERIOD, 0x71);
                }
                self.shadow.set_w(s + 6, self.p41.w(c + PERIOD));
            }
            2 => {
                let p = self.p41.w(c + PERIOD).wrapping_add(param as u16);
                self.p41.set_w(c + PERIOD, p);
                if p as i16 > 0x358 {
                    self.p41.set_w(c + PERIOD, 0x358);
                }
                self.shadow.set_w(s + 6, self.p41.w(c + PERIOD));
            }
            3 => self.tone_portamento(n),
            4 => self.vibrato(data, n),
            5 => {
                self.volume_slide(n);
                self.tone_portamento(n);
            }
            6 => {
                self.volume_slide(n);
                self.vibrato(data, n);
            }
            7 => self.tremolo(data, n),
            8 => self.arpeggio(data, n),
            0xa => self.volume_slide(n),
            0xe => self.tick_extended(data, n, out),
            _ => {}
        }
    }

    /// `A`, and the slide half of `5` and `6`: the parameter is a signed byte taken off the volume's
    /// low byte; a result with bit 7 set is silence.
    fn volume_slide(&mut self, n: usize) {
        let c = ch(n);
        let v = self
            .p41
            .b(c + VOLUME + 1)
            .wrapping_sub(self.p41.b(c + PARAM));
        self.p41.set_b(c + VOLUME + 1, v);
        if (v as i8) < 0 {
            self.p41.set_w(c + VOLUME, 0);
        } else if self.p41.w(c + VOLUME) as i16 > 0x40 {
            self.p41.set_w(c + VOLUME, 0x40);
        }
        self.shadow.set_w(sh(n) + 8, self.p41.w(c + VOLUME));
    }

    /// `3` (`$13a98`).
    fn tone_portamento(&mut self, n: usize) {
        let c = ch(n);
        let target = self.p41.w(c + PORTA_TARGET);
        if target == 0 {
            return;
        }
        let speed = self.p41.w(c + PORTA_SPEED);
        let p = self.p41.w(c + PERIOD);
        let reached = if (target as i16) < (p as i16) {
            let p = p.wrapping_sub(speed);
            self.p41.set_w(c + PERIOD, p);
            (target as i16) >= (p as i16)
        } else {
            let p = p.wrapping_add(speed);
            self.p41.set_w(c + PERIOD, p);
            (target as i16) <= (p as i16)
        };
        if reached {
            self.p41.set_w(c + PERIOD, target);
            self.p41.set_w(c + PORTA_TARGET, 0);
        }
        self.shadow.set_w(sh(n) + 6, self.p41.w(c + PERIOD));
    }

    /// The index into the vibrato table for the command and position at `field` (`$16` or `$18`).
    fn wave_index(&self, field: usize) -> u32 {
        let w = self.p41.w(field);
        (((w & 0xf00) >> 2) + ((w >> 1) & 0x3e)) as u32
    }

    /// Advance the position byte after `field` by the command's speed.
    fn wave_step(&mut self, field: usize) {
        let step = (self.p41.b(field) >> 2) & 0x3c;
        let pos = self.p41.b(field + 1).wrapping_add(step);
        self.p41.set_b(field + 1, pos);
    }

    /// `4` (`$14030`).
    fn vibrato(&mut self, data: &SoundData, n: usize) {
        let c = ch(n);
        let delta = data.h0_word(VIBRATO_TABLE + self.wave_index(c + VIBRATO));
        let p = self.p41.w(c + PERIOD);
        let p = if (self.p41.b(c + VIBRATO + 1) as i8) < 0 {
            p.wrapping_sub(delta)
        } else {
            p.wrapping_add(delta)
        };
        self.shadow.set_w(sh(n) + 6, p);
        self.wave_step(c + VIBRATO);
    }

    /// `7` (`$1406e`).
    fn tremolo(&mut self, data: &SoundData, n: usize) {
        let c = ch(n);
        let delta = data.h0_word(VIBRATO_TABLE + self.wave_index(c + TREMOLO));
        let v = self.p41.w(c + VOLUME);
        let v = if (self.p41.b(c + TREMOLO + 1) as i8) < 0 {
            let v = v.wrapping_sub(delta);
            if (v as i16) < 0 { 0 } else { v }
        } else {
            let v = v.wrapping_add(delta);
            if v as i16 > 0x40 { 0x40 } else { v }
        };
        self.shadow.set_w(sh(n) + 8, v);
        self.wave_step(c + TREMOLO);
    }

    /// `8` (`$13568`): this replayer's arpeggio. Step 1 adds the low nibble, step 2 the high one.
    fn arpeggio(&mut self, data: &SoundData, n: usize) {
        let c = ch(n);
        let step = data.h0_byte(ARP_STEPS.wrapping_add(sext(self.p41.w(TICK))));
        let note = self.p41.w(c + NOTE);
        let index = if step == 0 {
            note
        } else {
            let param = self.p41.b(c + PARAM);
            let x = if step & 1 != 0 {
                param & 0xf
            } else {
                param >> 4
            };
            ((x as u16) * 2).wrapping_add(note)
        };
        let p = self.period(data, index);
        self.shadow.set_w(sh(n) + 6, p);
    }

    /// `E9`, `EC`, `ED` between rows (`$134b2`).
    fn tick_extended(&mut self, data: &SoundData, n: usize, out: &mut Vec<PaulaWrite>) {
        let c = ch(n);
        let param = self.p41.b(c + PARAM);
        let x = (param & 0xf) as u16;
        match param & 0xf0 {
            0x90 => {
                let r = self.p41.w(c + RETRIG).wrapping_sub(1);
                self.p41.set_w(c + RETRIG, r);
                if r != 0 {
                    return;
                }
                let d = self.p41.w(c + DMA_BIT) & 0x800f;
                wr(out, DMACON, d);
                let on = self.p41.w(DMA_ON) | d | 0x8000;
                self.p41.set_w(DMA_ON, on);
                self.restart_sample(data, n);
                self.p41.set_w(c + RETRIG, x);
                self.p41.set_b(DMA_PENDING, 0xff);
            }
            0xc0 => {
                if x == self.p41.w(TICK) {
                    self.shadow.set_w(sh(n) + 8, 0);
                    self.p41.set_w(c + VOLUME, 0);
                }
            }
            0xd0 => {
                if x == self.p41.w(TICK) {
                    let on = self.p41.w(DMA_ON) | self.p41.w(c + DMA_BIT) | 0x8000;
                    self.p41.set_w(DMA_ON, on);
                    self.restart_sample(data, n);
                    self.p41.set_b(DMA_PENDING, 0xff);
                }
            }
            _ => {}
        }
    }

    /// The instrument's start and length into the channel's shadow.
    fn restart_sample(&mut self, data: &SoundData, n: usize) {
        let inst = self.p41.l(ch(n) + INST);
        self.shadow.set_l(sh(n), self.mem.l(data, inst));
        self.shadow
            .set_w(sh(n) + 4, self.mem.w(data, inst.wrapping_add(4)));
    }

    /// `$13b34`: read the next row of every channel from its pattern stream, one tick before it
    /// plays.
    fn fetch_rows(&mut self, data: &SoundData) {
        let delay = self.p41.w(FETCH_DELAY);
        if delay != 0 {
            self.p41.set_w(FETCH_DELAY, delay - 1);
            return;
        }
        let module = self.p41.l(MODULE);
        let patterns = self.p41.l(PATTERNS);
        for n in 0..4 {
            let c = ch(n);
            let count = self.p41.b(c + COUNT);
            if count != 0 {
                if (count as i8) > 0 {
                    // An empty row: no new data, and no effect runs on it.
                    self.p41.set_b(c + COUNT, count - 1);
                    self.p41.set_w(c + LIVE, 0);
                    continue;
                }
                // The same row again.
                self.p41.set_b(c + COUNT, count.wrapping_add(1));
            } else if self.p41.b(c + BLOCK_COUNT + 1) != 0 {
                let k = self.p41.b(c + BLOCK_COUNT + 1) - 1;
                self.p41.set_b(c + BLOCK_COUNT + 1, k);
                let at = self.p41.l(c + BLOCK);
                self.p41.set_l(c + ROW, self.mem.l(data, at));
                self.p41.set_l(c + BLOCK, at.wrapping_add(4));
            } else {
                let at = self.p41.l(c + STREAM);
                if (self.mem.b(data, at) as i8) >= 0 {
                    self.p41.set_l(c + ROW, self.mem.l(data, at));
                    self.p41.set_l(c + STREAM, at.wrapping_add(4));
                } else {
                    // A reference: a count word and an offset to rows played earlier.
                    self.p41.set_w(c + BLOCK_COUNT, self.mem.w(data, at));
                    let off = sext(self.mem.w(data, at.wrapping_add(2)));
                    self.p41.set_l(c + STREAM, at.wrapping_add(4));
                    let from = patterns.wrapping_add(off);
                    self.p41.set_l(c + ROW, self.mem.l(data, from));
                    self.p41.set_l(c + BLOCK, from.wrapping_add(4));
                }
            }
            self.p41.set_b(c + LIVE, 0xff);
            if !self.free(n) {
                let r = self.p41.w(c + ROW) & 0xf;
                self.p41.set_w(c + ROW, r);
            }
            let inst = (self.p41.w(c + ROW) & 0x1f0) as u32;
            if inst != 0 {
                let a = module.wrapping_add(inst);
                self.p41.set_l(c + INST, a);
                self.p41
                    .set_l(c + VOLUME, self.mem.l(data, a.wrapping_add(0xc)));
            }
        }
    }

    /// `$13bdc`: tick 0, the row plays.
    fn play_row(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        self.p41.set_w(TICK, 0);
        let delay = self.p41.w(ROW_DELAY);
        if delay != 0 {
            self.p41.set_w(ROW_DELAY, delay - 1);
            return;
        }
        if self.p41.w(FETCH_AT_ROW) != 0 {
            self.fetch_rows(data);
        }
        self.p41.set_b(DMA_PENDING, 0xff);
        let mut dma: u16 = 0;
        for n in 0..4 {
            let c = ch(n);
            if self.p41.w(c + LIVE) != 0 {
                self.row_note(data, n, &mut dma, out);
            }
            if self.free(n) {
                self.shadow.set_l(sh(n) + 6, self.p41.l(c + PERIOD));
            }
        }
        let dma = dma & 0x800f;
        wr(out, DMACON, dma);
        self.p41.set_b(DMA_ON + 1, dma as u8);
    }

    /// One channel's row at tick 0 (`$13c0e`).
    fn row_note(&mut self, data: &SoundData, n: usize, dma: &mut u16, out: &mut Vec<PaulaWrite>) {
        let c = ch(n);
        let effect = self.p41.w(c + ROW) & 0xf;
        let param = self.p41.b(c + PARAM);
        if effect == 3 || effect == 5 {
            if effect == 3 && param != 0 {
                self.p41.set_b(c + PORTA_SPEED + 1, param);
            }
            let note = (self.p41.b(c + ROW) & 0x7e) as u16;
            if note != 0 {
                let index = note.wrapping_add(self.p41.w(c + FINETUNE));
                self.p41.set_w(c + NOTE, index);
                let p = self.period(data, index);
                self.p41.set_w(c + PORTA_TARGET, p);
            }
            return;
        }
        if !self.free(n) {
            self.p41.set_b(c + ROW, 0);
        }
        let note = (self.p41.b(c + ROW) & 0x7e) as u16;
        if note != 0 {
            self.p41.set_b(c + VIBRATO + 1, 0);
            let index = note.wrapping_add(self.p41.w(c + FINETUNE));
            self.p41.set_w(c + NOTE, index);
            let p = self.period(data, index);
            self.p41.set_w(c + PERIOD, p);
            let d = self.p41.w(c + DMA_BIT) & 0x800f;
            wr(out, DMACON, d);
            *dma |= d;
            if effect == 9 {
                self.sample_offset(data, n);
                return;
            }
            self.restart_sample(data, n);
        }
        self.row_effect(data, n, dma);
    }

    /// `9` with a note (`$13d18`): start `param` × 256 bytes into the sample, or at its start with
    /// length 1 when that is past its end.
    fn sample_offset(&mut self, data: &SoundData, n: usize) {
        let c = ch(n);
        let off = self.p41.w(c + PARAM) & 0xff00;
        let inst = self.p41.l(c + INST);
        let start = self.mem.l(data, inst);
        self.shadow.set_l(sh(n), start.wrapping_add(off as u32));
        let mut len = self
            .mem
            .w(data, inst.wrapping_add(4))
            .wrapping_sub(off >> 1);
        if (len as i16) < 0 {
            self.shadow.set_l(sh(n), start);
            len = 1;
        }
        self.shadow.set_w(sh(n) + 4, len);
    }

    /// The effects that act when their row plays (`$144bc`).
    fn row_effect(&mut self, data: &SoundData, n: usize, dma: &mut u16) {
        let c = ch(n);
        let param = self.p41.b(c + PARAM);
        let x = (param & 0xf) as u16;
        match self.p41.w(c + ROW) & 0xf {
            4 => self.set_wave(c + VIBRATO, param),
            7 => self.set_wave(c + TREMOLO, param),
            0xb => {
                let pos = self.p41.l(POSITIONS).wrapping_add((param as u32) << 2);
                self.p41.set_l(POSITION, pos);
                self.next_position(data, 0x40);
            }
            0xc => self.p41.set_b(c + VOLUME + 1, param),
            0xd => self.next_position(data, 0x40),
            0xe => match param >> 4 {
                // 0, the LED filter, writes the CIA, not Paula.
                1 => {
                    let p = self.p41.w(c + PERIOD).wrapping_sub(x);
                    self.p41
                        .set_w(c + PERIOD, if 0x71 > p as i16 { 0x71 } else { p });
                }
                2 => {
                    let p = self.p41.w(c + PERIOD).wrapping_add(x);
                    self.p41
                        .set_w(c + PERIOD, if p as i16 > 0x358 { 0x358 } else { p });
                }
                5 => self
                    .p41
                    .set_w(c + FINETUNE, data.h0_word(FINETUNES + x as u32 * 2)),
                9 => self.p41.set_w(c + RETRIG, x),
                0xa => {
                    let v = self.p41.w(c + VOLUME).wrapping_add(x);
                    self.p41
                        .set_w(c + VOLUME, if v as i16 > 0x40 { 0x40 } else { v });
                }
                0xb => {
                    let v = self.p41.w(c + VOLUME).wrapping_sub(x);
                    self.p41
                        .set_w(c + VOLUME, if (v as i16) < 0 { 0 } else { v });
                }
                0xd => *dma ^= self.p41.w(c + DMA_BIT),
                0xe => {
                    self.p41.set_w(FETCH_DELAY, x);
                    self.p41.set_w(ROW_DELAY, x);
                }
                _ => {}
            },
            0xf => {
                let slow = self.music_on.b(3) != 0;
                let speed = if slow { param.wrapping_add(1) } else { param };
                self.p41.set_b(SPEED + 1, speed);
                if param == 1 {
                    self.p41.set_b(FETCH_TICK + 1, speed);
                    self.p41.set_b(FETCH_AT_ROW, 0xff);
                } else {
                    self.p41.set_b(FETCH_TICK + 1, speed.wrapping_sub(1));
                    self.p41.set_w(FETCH_AT_ROW, 0);
                }
            }
            _ => {}
        }
    }

    /// `4`/`7` at their row: a nonzero nibble of the parameter replaces that nibble of the command.
    fn set_wave(&mut self, field: usize, param: u8) {
        if param == 0 {
            return;
        }
        let mut v = self.p41.b(field);
        if param & 0xf != 0 {
            v = v & 0xf0 | param & 0xf;
        }
        if param & 0xf0 != 0 {
            v = v & 0xf | param & 0xf0;
        }
        self.p41.set_b(field, v);
    }

    /// Point every channel at the patterns of the position `POSITION` holds (the list restarts at
    /// a negative word), and set the rows left.
    fn next_position(&mut self, data: &SoundData, rows: u16) {
        self.p41.set_w(ROWS_LEFT, rows);
        let patterns = self.p41.l(PATTERNS);
        let mut at = self.p41.l(POSITION);
        let mut first = self.mem.w(data, at);
        at = at.wrapping_add(2);
        if (first as i16) < 0 {
            at = self.p41.l(POSITIONS);
            first = self.mem.w(data, at);
            at = at.wrapping_add(2);
        }
        let mut offs = [first, 0, 0, 0];
        for o in offs.iter_mut().skip(1) {
            *o = self.mem.w(data, at);
            at = at.wrapping_add(2);
        }
        self.p41.set_l(POSITION, at);
        for (n, &o) in offs.iter().enumerate() {
            self.p41
                .set_l(ch(n) + STREAM, patterns.wrapping_add(sext(o)));
        }
    }

    /// `$145b6`: the fade, then each channel music holds goes from the shadow to Paula.
    fn shadow_to_paula(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        if self.shadow.w(FADE) != 0 {
            let k = self.shadow.b(FADE_COUNT).wrapping_sub(1);
            self.shadow.set_b(FADE_COUNT, k);
            let mut apply = true;
            if (k as i8) < 0 {
                self.shadow.set_b(FADE_COUNT, self.shadow.b(FADE_SPEED));
                let ceiling = self.shadow.b(FADE).wrapping_sub(1);
                self.shadow.set_b(FADE, ceiling);
                if ceiling == 0 {
                    self.music_stop(data, out);
                    self.shadow.set_w(FADE, 0);
                    apply = false;
                }
            }
            if apply {
                let ceiling = (self.shadow.b(FADE) & 0x3f) as u16;
                for n in 0..4 {
                    if ceiling < self.shadow.w(sh(n) + 8) {
                        self.shadow.set_w(sh(n) + 8, ceiling);
                    }
                }
            }
        }
        for n in 0..4 {
            if self.free(n) {
                let s = sh(n);
                let reg = 0xa0 + n as u16 * 16;
                wr(out, reg, self.shadow.w(s));
                wr(out, reg + 2, self.shadow.w(s + 2));
                wr(out, reg + 4, self.shadow.w(s + 4));
                wr(out, reg + 6, self.shadow.w(s + 6));
                wr(out, reg + 8, self.shadow.w(s + 8));
            }
        }
    }

    /// `$13cc4`, later in the same interrupt: the channels whose notes started this tick are
    /// switched on, the next position is taken when a pattern ends, and every music channel's
    /// shadow is pointed at its instrument's loop for the next tick.
    pub fn music_dma_tick(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>) {
        if self.music_on.w(0) == 0 || self.p41.w(DMA_PENDING) == 0 {
            return;
        }
        self.p41.set_w(DMA_PENDING, 0);
        wr(out, DMACON, self.p41.w(DMA_ON));
        wr(out, INTENA, 0x4000);
        wr(out, INTENA, 0xc000);
        if self.p41.w(TICK) == 0 {
            let rows = self.p41.w(ROWS_LEFT).wrapping_sub(1);
            self.p41.set_w(ROWS_LEFT, rows);
            if (rows as i16) < 0 {
                self.next_position(data, 0x3f);
            }
        }
        for n in 0..4 {
            if self.free(n) {
                let inst = self.p41.l(ch(n) + INST);
                self.shadow
                    .set_l(sh(n), self.mem.l(data, inst.wrapping_add(6)));
                self.shadow
                    .set_w(sh(n) + 4, self.mem.w(data, inst.wrapping_add(0xa)));
            }
        }
    }
}
