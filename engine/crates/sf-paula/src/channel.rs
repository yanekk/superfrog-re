//! Paula's four audio channels and the part of Agnus that feeds them, one colour clock at a time.
//!
//! Integer only (DESIGN §3.1). The model is the audio state machine of the Amiga Hardware Reference
//! Manual (states 000, 001, 101, 010, 011) as the reference emulator implements it (vAmiga
//! `StateMachine`, `Agnus::serviceDASEvent`), quirks included, because the per-channel DAC values
//! are compared with the reference cycle for cycle (T10):
//!
//! - A channel asks Agnus for a word (`AUDxDR`); the request reaches Agnus at the end of the line
//!   (handled at the very start of the next line's clock 0) and the word is fetched in the
//!   channel's DMA slot of that line (hpos 0x0D, 0x0F, 0x11, 0x13), then written to AUDxDAT.
//! - The period counter counts colour clocks; 0 means 65 536. Nothing clamps a small period: the
//!   channel then runs out of data, and the DAC takes a new value only when a new word has
//!   arrived since the last one (the reference's `enablePenhi`/`enablePenlo`).
//! - DMACON takes effect 2 clocks after the write, INTREQ 1 clock after (68000), the rest at once.
//! - The DAC value is the sample byte times the volume (0..=64) latched with it.
//! - Within one clock: the end of the last line, delayed register changes, the DMA slot, the
//!   period counters, audio interrupts, then the CPU's or Copper's register write.
//! - A frame's writes are in beam order. The reference records a frame from just after its start to
//!   just after the next one's, so a write whose position goes back belongs, with all after it, to
//!   the next frame's first clocks; it is held and lands there.

use crate::ChipMemory;
use crate::write::PaulaWrite;

/// Colour clocks in a PAL line (hpos 0..=0xE2).
pub const CLOCKS_PER_LINE: u32 = 227;
/// Lines in a long PAL frame. A short frame has one fewer.
pub const LINES_PER_FRAME: u32 = 313;
/// Colour clocks (Paula clocks) in a long PAL frame: 3 546 895 / 71 051 = 49.92 Hz.
pub const CLOCKS_PER_FRAME: u32 = CLOCKS_PER_LINE * LINES_PER_FRAME;

/// Channel n's DMA slot is at hpos `FIRST_SLOT + 2 n`.
const FIRST_SLOT: u32 = 0x0D;
/// OCS Agnus addresses 512 KB of chip RAM.
const PTR_MASK: u32 = 0x7_FFFE;

const DMAEN: u16 = 0x0200;
const DMACON: u16 = 0x96;
const INTREQ: u16 = 0x9C;
const ADKCON: u16 = 0x9E;

/// The state machine's states, named by their HRM numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    S000,
    S001,
    S101,
    S010,
    S011,
}

#[derive(Debug, Clone)]
struct Channel {
    state: State,
    /// The word being played: high byte first, then low.
    buffer: u16,
    len_latch: u16,
    len: u16,
    per_latch: u16,
    vol_latch: u16,
    vol: u16,
    dat: u16,
    /// The channel wants a word; Agnus hears it at the end of the line.
    dma_request: bool,
    /// Raise the interrupt at the next 011->010 transition (a block ended).
    intreq2: bool,
    /// A new word arrived since the DAC last took a byte (the reference's pen locks).
    pen_hi: bool,
    pen_lo: bool,
    /// When the period counter runs out, in clocks since power-on.
    period_end: Option<u64>,
    /// What the DAC outputs: sample byte times volume.
    dac: i16,
}

impl Channel {
    fn new() -> Channel {
        Channel {
            state: State::S000,
            buffer: 0,
            len_latch: 0,
            len: 0,
            per_latch: 0,
            vol_latch: 0,
            vol: 0,
            dat: 0,
            dma_request: false,
            intreq2: false,
            pen_hi: false,
            pen_lo: false,
            period_end: None,
            dac: 0,
        }
    }
}

/// A register change that lands a few clocks after its write.
#[derive(Debug, Clone, Copy)]
struct Delayed {
    at: u64,
    reg: u16,
    value: u16,
}

/// The four channels, Agnus's audio pointers and requests, and the bits of DMACON, ADKCON and
/// INTREQ the channels read.
#[derive(Debug, Clone)]
pub struct Chip {
    ch: [Channel; 4],
    /// AUDxLC: where each block starts.
    lc: [u32; 4],
    /// Agnus's running DMA pointers.
    pt: [u32; 4],
    /// Requests Agnus has taken and will serve in the channel's slot.
    agnus_request: [bool; 4],
    dmacon: u16,
    adkcon: u16,
    intreq: u16,
    delayed: Vec<Delayed>,
    /// Audio interrupts raised, landing in INTREQ one clock later: (clock, channel).
    irqs: Vec<(u64, usize)>,
    /// Writes from the end of the last frame's list that belong to this frame's start.
    carried: Vec<PaulaWrite>,
    /// Clocks since power-on at the start of the current frame.
    frame_start: u64,
    /// The clock being run.
    now: u64,
}

impl Default for Chip {
    fn default() -> Chip {
        Chip::new()
    }
}

impl Chip {
    /// Power-on: every channel idle and silent, DMA off.
    pub fn new() -> Chip {
        Chip {
            ch: [
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
            ],
            lc: [0; 4],
            pt: [0; 4],
            agnus_request: [false; 4],
            dmacon: 0,
            adkcon: 0,
            intreq: 0,
            delayed: Vec::new(),
            irqs: Vec::new(),
            carried: Vec::new(),
            frame_start: 0,
            now: 0,
        }
    }

    /// Run one PAL frame of `lines` lines (313 long, 312 short). Each write lands at its beam
    /// position (`vpos`, `hpos`); one behind the clock already run lands at once, one past the
    /// frame's end at its last clock. `taps[n]` is cleared and filled with channel n's DAC value at
    /// every clock of the frame.
    pub fn run_frame(
        &mut self,
        writes: &[PaulaWrite],
        lines: u32,
        chip: &dyn ChipMemory,
        taps: &mut [Vec<i16>; 4],
    ) {
        let clocks = lines * CLOCKS_PER_LINE;
        let wrap = (1..writes.len())
            .find(|&i| at(&writes[i]) < at(&writes[i - 1]))
            .unwrap_or(writes.len());
        let mut list = std::mem::take(&mut self.carried);
        list.extend_from_slice(&writes[..wrap]);
        self.carried = writes[wrap..].to_vec();

        for t in taps.iter_mut() {
            t.clear();
            t.reserve(clocks as usize);
        }
        let mut next = 0;
        for clock in 0..clocks {
            self.now = self.frame_start + clock as u64;
            let h = clock % CLOCKS_PER_LINE;
            // The end of the last line is handled first thing in this one: Agnus takes the
            // channels' requests.
            if h == 0 {
                for n in 0..4 {
                    if self.ch[n].dma_request {
                        self.agnus_request[n] = true;
                        self.ch[n].dma_request = false;
                    }
                }
            }
            self.register_changes();
            if (FIRST_SLOT..FIRST_SLOT + 8).contains(&h) && (h - FIRST_SLOT) % 2 == 0 {
                self.fetch(((h - FIRST_SLOT) / 2) as usize, chip);
            }
            for n in 0..4 {
                if self.ch[n].period_end == Some(self.now) {
                    self.period_ends(n);
                }
            }
            self.interrupts();
            while next < list.len() && at(&list[next]) <= clock {
                self.write(list[next].reg, list[next].value);
                next += 1;
            }
            for n in 0..4 {
                taps[n].push(self.ch[n].dac);
            }
        }
        self.now = self.frame_start + clocks as u64 - 1;
        for w in &list[next..] {
            self.write(w.reg, w.value);
        }
        self.frame_start += clocks as u64;
    }

    fn register_changes(&mut self) {
        let now = self.now;
        if self.delayed.iter().all(|d| d.at != now) {
            return;
        }
        let due: Vec<Delayed> = self
            .delayed
            .iter()
            .filter(|d| d.at == now)
            .copied()
            .collect();
        self.delayed.retain(|d| d.at != now);
        for d in due {
            match d.reg {
                DMACON => self.set_dmacon(d.value),
                _ => self.set_intreq(d.value),
            }
        }
    }

    fn interrupts(&mut self) {
        let now = self.now;
        let mut i = 0;
        while i < self.irqs.len() {
            if self.irqs[i].0 <= now {
                self.intreq |= 0x80 << self.irqs[i].1;
                self.irqs.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// A write to a Paula-related register as the CPU or Copper makes it.
    pub fn write(&mut self, reg: u16, value: u16) {
        match reg {
            DMACON => self.delayed.push(Delayed {
                at: self.now + 2,
                reg,
                value,
            }),
            INTREQ => self.delayed.push(Delayed {
                at: self.now + 1,
                reg,
                value,
            }),
            ADKCON => {
                if value & 0x8000 != 0 {
                    self.adkcon |= value & 0x7FFF;
                } else {
                    self.adkcon &= !value;
                }
            }
            0xA0..=0xDF => {
                let n = ((reg - 0xA0) >> 4) as usize;
                match reg & 0xF {
                    0x0 => self.lc[n] = (self.lc[n] & 0xFFFF) | (value as u32) << 16,
                    0x2 => self.lc[n] = (self.lc[n] & 0xFFFF_0000) | (value & 0xFFFE) as u32,
                    0x4 => self.ch[n].len_latch = value,
                    0x6 => self.ch[n].per_latch = value,
                    0x8 => self.poke_vol(n, value),
                    0xA => self.poke_dat(n, value),
                    _ => {}
                }
            }
            _ => {} // INTENA and anything else: the channels do not read them
        }
    }

    fn set_dmacon(&mut self, value: u16) {
        let old = self.dmacon;
        let new = if value & 0x8000 != 0 {
            (old | value) & 0x07FF
        } else {
            old & !value & 0x07FF
        };
        self.dmacon = new;
        let old_dma = if old & DMAEN != 0 { old } else { 0 };
        let new_dma = if new & DMAEN != 0 { new } else { 0 };
        for n in 0..4 {
            if (old_dma ^ new_dma) & (1 << n) != 0 {
                if new_dma & (1 << n) != 0 {
                    if self.ch[n].state == State::S000 {
                        self.move_000_001(n);
                    }
                } else if matches!(self.ch[n].state, State::S001 | State::S101) {
                    self.ch[n].state = State::S000;
                }
            }
        }
    }

    fn set_intreq(&mut self, value: u16) {
        if value & 0x8000 != 0 {
            self.intreq |= value & 0x7FFF;
        } else {
            self.intreq &= !value;
        }
    }

    fn fetch(&mut self, n: usize, chip: &dyn ChipMemory) {
        if !self.agnus_request[n] {
            return;
        }
        self.agnus_request[n] = false;
        let word = chip.chip_word(self.pt[n] & PTR_MASK);
        self.pt[n] = self.pt[n].wrapping_add(2);
        self.poke_dat(n, word);
    }

    fn dma_on(&self, n: usize) -> bool {
        self.dmacon & DMAEN != 0 && self.dmacon & (1 << n) != 0
    }

    fn irq_pending(&self, n: usize) -> bool {
        self.intreq & (0x80 << n) != 0
    }

    /// AUDxAV: this channel's data is the next channel's volume.
    fn attach_volume(&self, n: usize) -> bool {
        self.adkcon & (1 << n) != 0
    }

    /// AUDxAP: this channel's data is the next channel's period.
    fn attach_period(&self, n: usize) -> bool {
        self.adkcon & (0x10 << n) != 0
    }

    fn napnav(&self, n: usize) -> bool {
        !self.attach_period(n) || self.attach_volume(n)
    }

    fn raise(&mut self, n: usize) {
        self.irqs.push((self.now + 1, n));
    }

    fn poke_vol(&mut self, n: usize, value: u16) {
        self.ch[n].vol_latch = (value & 0x7F).min(64);
    }

    fn poke_dat(&mut self, n: usize, value: u16) {
        let c = &mut self.ch[n];
        c.dat = value;
        c.pen_hi = true;
        c.pen_lo = true;
        if self.dma_on(n) {
            match self.ch[n].state {
                State::S000 => self.move_000_001(n),
                State::S001 => self.move_001_101(n),
                State::S101 => self.move_101_010(n),
                State::S010 | State::S011 => {
                    let c = &mut self.ch[n];
                    if c.len != 1 {
                        c.len = c.len.wrapping_sub(1);
                    } else {
                        c.len = c.len_latch;
                        self.pt[n] = self.lc[n];
                        self.ch[n].intreq2 = true;
                    }
                }
            }
        } else if self.ch[n].state == State::S000 && !self.irq_pending(n) {
            self.move_000_010(n);
        }
    }

    fn reload_period(&mut self, n: usize) {
        let p = self.ch[n].per_latch;
        let p = if p == 0 { 0x1_0000 } else { p as u64 };
        self.ch[n].period_end = Some(self.now + p);
    }

    /// pbufld1: the held word goes to the output buffer, or with attached volume to the next
    /// channel's volume.
    fn load_buffer(&mut self, n: usize) {
        if !self.attach_volume(n) {
            self.ch[n].buffer = self.ch[n].dat;
        } else if n < 3 {
            let dat = self.ch[n].dat;
            self.poke_vol(n + 1, dat);
        }
    }

    fn pen_hi(&mut self, n: usize) {
        let c = &mut self.ch[n];
        if c.pen_hi {
            c.dac = ((c.buffer >> 8) as u8 as i8) as i16 * c.vol as i16;
            c.pen_hi = false;
        }
    }

    fn pen_lo(&mut self, n: usize) {
        let c = &mut self.ch[n];
        if c.pen_lo {
            c.dac = (c.buffer as u8 as i8) as i16 * c.vol as i16;
            c.pen_lo = false;
        }
    }

    fn move_000_001(&mut self, n: usize) {
        let c = &mut self.ch[n];
        c.len = c.len_latch;
        c.dma_request = true;
        c.state = State::S001;
    }

    fn move_001_101(&mut self, n: usize) {
        self.raise(n);
        self.ch[n].dma_request = true;
        self.pt[n] = self.lc[n];
        let c = &mut self.ch[n];
        if c.len != 1 {
            c.len = c.len.wrapping_sub(1);
        }
        c.state = State::S101;
    }

    fn move_101_010(&mut self, n: usize) {
        self.reload_period(n);
        self.ch[n].vol = self.ch[n].vol_latch;
        self.load_buffer(n);
        if self.napnav(n) {
            self.ch[n].dma_request = true;
        }
        self.ch[n].state = State::S010;
        self.pen_hi(n);
    }

    fn move_000_010(&mut self, n: usize) {
        self.ch[n].vol = self.ch[n].vol_latch;
        self.reload_period(n);
        self.load_buffer(n);
        self.raise(n);
        self.ch[n].state = State::S010;
        self.pen_hi(n);
    }

    fn move_010_011(&mut self, n: usize) {
        self.reload_period(n);
        if self.attach_period(n) {
            if n < 3 {
                self.ch[n + 1].per_latch = self.ch[n].dat;
            }
            self.attached_request(n);
        }
        self.ch[n].state = State::S011;
        self.pen_lo(n);
    }

    fn move_011_010(&mut self, n: usize) {
        self.reload_period(n);
        self.load_buffer(n);
        self.ch[n].vol = self.ch[n].vol_latch;
        if self.napnav(n) {
            self.attached_request(n);
        }
        self.ch[n].state = State::S010;
        self.pen_hi(n);
    }

    /// The request (DMA) or interrupt (no DMA) at the end of a word.
    fn attached_request(&mut self, n: usize) {
        if self.dma_on(n) {
            self.ch[n].dma_request = true;
            if self.ch[n].intreq2 {
                self.raise(n);
                self.ch[n].intreq2 = false;
            }
        } else {
            self.raise(n);
        }
    }

    fn period_ends(&mut self, n: usize) {
        match self.ch[n].state {
            State::S010 => self.move_010_011(n),
            State::S011 => {
                if self.dma_on(n) || !self.irq_pending(n) {
                    self.move_011_010(n);
                } else {
                    let c = &mut self.ch[n];
                    c.period_end = None;
                    c.intreq2 = false;
                    c.state = State::S000;
                }
            }
            // Only 010 and 011 run the period counter.
            _ => self.ch[n].period_end = None,
        }
    }
}

/// A write's clock within its frame.
fn at(w: &PaulaWrite) -> u32 {
    w.vpos as u32 * CLOCKS_PER_LINE + w.hpos as u32
}
