//! DMA contention (translator T49): who has the bus in each DMA slot, and how long the CPU's work
//! takes on the beam once the chips have taken theirs.
//!
//! The rules are the reference emulator's (vAmiga `Agnus::executeUntilBusIsFree`, `busIsFree`,
//! `allocateBus`, the SlowBlitter's micro-programs), which the oracle runs; the readings that hold
//! the machine to them are `toolkit/crates/amiga-oracle/tests/contention.rs`'s.
//!
//! - A slot (two CPU cycles) is the refresh's (`$01`, `$03`, `$05`, `$E2`) or a bitplane fetch's
//!   (`amiga_chips::copper::Fetch::takes`, with DMAEN and BPLEN on), else the blitter's when a
//!   blit's micro-instruction needs the bus there, else free.
//! - A CPU bus access is two slots: the access's own, then one Agnus runs while the CPU finishes
//!   the cycle (free for any chip). While the first is taken the CPU waits a slot at a time; from
//!   its third waited slot on the blitter gives way (BLS) unless BLTPRI is set.
//! - A slot the CPU spends off the bus (an instruction's internal cycles, `Regs::off_bus`) is free
//!   for the chips.
//!
//! - The copper takes the slots its run fetches in (`amiga_chips::copper::run_bus`).
//! - Sprite `n` takes slots `$15 + 4n` and `$17 + 4n` on the lines its DMA fetches (`Sprites`, as
//!   the picture's sprite channels run: the control words on line 25 and at the stop line, data
//!   inside the window). Odd slots, but the CPU runs on odd slots after an instruction's two
//!   internal cycles, so they cost it time (a line or more a pass in a scrolling stretch, T49).
//!
//! - The audio channels take their slots (`$0D` to `$13`, `audio`, T52).
//!
//! Not taken: disk slots (the program loads no file in play).

use amiga_chips::copper::Fetch;
use amiga_clock::is_refresh;

/// DMACON's bits the bus depends on.
pub const DMAEN: u16 = 0x0200;
pub const BPLEN: u16 = 0x0100;
pub const BLTEN: u16 = 0x0040;
pub const BLTPRI: u16 = 0x0400;
pub const SPREN: u16 = 0x0020;

/// The line sprite DMA reads every channel's control words on (`video.rs`'s reset line).
const SPRITE_RESET_LINE: u16 = 25;

/// The eight sprite channels' DMA as the bus sees it: each channel's pointer and control words, and
/// which channels fetch on the line the bus is on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sprites {
    channels: [(u32, u16, u16); 8],
    /// The line whose fetches `fetching` holds; `None` before the bus has run a line, and until a
    /// reset line for a machine started part way down a frame (its pointers are unknown).
    line: Option<u16>,
    known: bool,
    fetching: u8,
}

impl Sprites {
    /// Enter `line`: which channels fetch on it, reading control words from `chip` and the
    /// pointers from `custom` (SPRxPT, as last written) on the reset line.
    fn enter(&mut self, line: u16, chip: &[u8], custom: &[u16; 256]) {
        self.line = Some(line);
        self.fetching = 0;
        if line == SPRITE_RESET_LINE {
            for (n, c) in self.channels.iter_mut().enumerate() {
                let r = 0x120 / 2 + 2 * n;
                c.0 = (custom[r] as u32 & 0x1F) << 16 | custom[r + 1] as u32 & 0xFFFE;
            }
            self.known = true;
        }
        if !self.known || line < SPRITE_RESET_LINE {
            return;
        }
        let word = |a: u32| {
            let i = (a as usize) & (chip.len() - 1) & !1;
            u16::from_be_bytes([chip[i], chip[i + 1]])
        };
        for (n, c) in self.channels.iter_mut().enumerate() {
            let (start, stop) = amiga_chips::sprite::lines(c.1, c.2);
            if line == SPRITE_RESET_LINE || line == stop {
                c.1 = word(c.0);
                c.2 = word(c.0 + 2);
            } else if !(line >= start && line < stop) {
                continue;
            }
            c.0 += 4;
            self.fetching |= 1 << n;
        }
    }

    /// Whether a sprite fetch takes slot `h` of the line entered last.
    fn takes(&self, h: u16) -> bool {
        (0x15..=0x34).contains(&h) && h % 2 == 1 && self.fetching >> ((h - 0x15) / 4) & 1 != 0
    }
}

/// One micro-instruction of a copy blit (vAmiga SlowBlitter, accuracy level 2, no fill).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Micro {
    /// FETCH_A/B/C: a read, which takes the bus.
    fetch: bool,
    /// WRITE_D: takes the bus, except for the first word, whose D stage is still locked (then it
    /// only waits for a free bus, as BUSIDLE).
    write_d: bool,
    /// BUSIDLE: waits for a free bus without taking it.
    idle: bool,
    /// REPEAT: the end of a word: back to the program's start while words remain.
    repeat: bool,
    /// BLTDONE: the blit's last instruction.
    done: bool,
}

const fn m(fetch: bool, write_d: bool, idle: bool, repeat: bool, done: bool) -> Micro {
    Micro {
        fetch,
        write_d,
        idle,
        repeat,
        done,
    }
}

const F: Micro = m(true, false, false, false, false);
const FR: Micro = m(true, false, false, true, false);
const WR: Micro = m(false, true, false, true, false);
const WD: Micro = m(false, true, false, false, true);
const I: Micro = m(false, false, true, false, false);
const IR: Micro = m(false, false, true, true, false);
const N: Micro = m(false, false, false, false, false);
const D: Micro = m(false, false, false, false, true);

/// The micro-programs by BLTCON0's USE bits (A B C D, bit 3 to 0), as vAmiga's
/// `copyBlitInstr[use][0][0]`: HOLD and AGAIDLE steps take no bus on OCS (they are `N`).
const PROGRAMS: [&[Micro]; 16] = [
    &[I, IR, N, D],     // 0: -- -- | -- --
    &[I, WR, N, WD],    // 1: -- D0 -- D1 | -- D2
    &[I, FR, N, D],     // 2: C0 -- C1 -- | -- C2
    &[I, F, WR, N, WD], // 3: C0 -- -- C1 D0 -- C2 D1 -- | -- D2
    &[I, F, IR, N, D],  // 4: B0 -- -- B1 -- -- | -- B2
    &[I, F, WR, N, WD], // 5
    &[I, F, FR, N, D],  // 6
    &[I, F, F, WR, N, WD], // 7
    &[F, IR, N, D],     // 8: A0 -- A1 -- | -- --
    &[F, WR, N, WD],    // 9: A0 -- -- A1 D0 -- A2 D1 -- | -- D2
    &[F, FR, N, D],     // A
    &[F, F, WR, N, WD], // B
    &[F, F, IR, N, D],  // C
    &[F, F, WR, N, WD], // D
    &[F, F, FR, N, D],  // E
    &[F, F, F, WR, N, WD], // F: A0 B0 C0 -- A1 B1 C1 D0 .. | -- D2
];

/// Where a blit's bus timing has got to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// BLTSIZE written; the blitter's two start-up cycles, each waiting for a free bus (vAmiga
    /// `BLT_STRT1`, `BLT_STRT2`), are still to come. The first runs the slot after the write.
    Start1,
    Start2,
    /// The first micro-instruction runs the slot after `BLT_STRT2`.
    Begin,
    Run,
}

/// A blit's bus timing: its memory effect was whole at the BLTSIZE write (`blitter.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlitRun {
    program: &'static [Micro],
    pc: usize,
    width: u16,
    x: u16,
    y: u16,
    lock_d: bool,
    pub(crate) stage: Stage,
    /// The cycle count of the BLTSIZE write: the blit takes no slot before it.
    from: u64,
    /// Set when BBUSY clears (the last word's REPEAT).
    pub(crate) free_at: Option<u64>,
    /// Set when BLTDONE runs: the interrupt follows a slot later.
    pub(crate) done_at: Option<u64>,
    /// The slot of each D write the bus has run, word by word (T52: the picture sees each word
    /// from there).
    pub(crate) d_at: Vec<u64>,
}

impl BlitRun {
    /// A blit of `rows` rows of `words` words under BLTCON0 `con0`.
    pub(crate) fn new(con0: u16, rows: u16, words: u16, from: u64) -> BlitRun {
        BlitRun {
            program: PROGRAMS[(con0 >> 8 & 15) as usize],
            pc: 0,
            width: words,
            x: words,
            y: rows,
            lock_d: true,
            stage: Stage::Start1,
            from,
            free_at: None,
            done_at: None,
            d_at: Vec::new(),
        }
    }

    /// The blitter's turn in slot `at` (its start in CPU cycles), `taken` by a fixed DMA channel
    /// or not, with the CPU asking for it (`bls`) and BLTPRI `pri`. Returns whether the blitter
    /// took the bus.
    fn slot(&mut self, at: u64, taken: bool, bls: bool, pri: bool) -> bool {
        if self.done_at.is_some() || at < self.from {
            return false;
        }
        let free = !taken && !(bls && !pri);
        match self.stage {
            Stage::Start1 => {
                if free {
                    self.stage = Stage::Start2;
                }
                false
            }
            Stage::Start2 => {
                if free {
                    self.stage = Stage::Begin;
                }
                false
            }
            Stage::Begin => {
                self.stage = Stage::Run;
                self.run(at, free)
            }
            Stage::Run => self.run(at, free),
        }
    }

    fn run(&mut self, at: u64, free: bool) -> bool {
        let op = self.program[self.pc];
        let (bus, idle) = if op.write_d {
            (!self.lock_d, self.lock_d)
        } else {
            (op.fetch, op.idle)
        };
        if (bus || idle) && !free {
            return false;
        }
        if op.write_d && bus {
            self.d_at.push(at);
        }
        self.pc += 1;
        if op.repeat {
            self.lock_d = false;
            if self.x > 1 {
                self.pc = 0;
                self.x -= 1;
            } else if self.y > 1 {
                self.pc = 0;
                self.x = self.width;
                self.y -= 1;
            } else {
                self.free_at = Some(at);
            }
        }
        if op.done {
            self.done_at = Some(at);
        }
        bus
    }
}

/// The bus as the chips hold it: the display's fetch, DMACON and a running blit.
pub(crate) struct Slots<'a> {
    pub fetch: Fetch,
    pub dmacon: u16,
    pub blit: Option<&'a mut BlitRun>,
    pub frame_lines: u16,
    /// The current frame's copper and the cycle count its frame began at: a slot it fetches in is
    /// taken; a slot in the next frame is that frame's copper's (`next_copper`).
    pub copper: Option<(&'a crate::copper::Copper, i64)>,
    pub sprites: &'a mut Sprites,
    /// The audio channels' slots (T52), looked up as slots from the copper's frame start.
    pub(crate) audio: &'a mut crate::audio::Audio,
    pub chip: &'a [u8],
    pub custom: &'a [u16; 256],
    /// The current frame's copper writes not yet delivered, and how many of them the slots run
    /// so far have passed: one to BPLCON0, DIWSTRT/DIWSTOP, DDFSTRT/DDFSTOP or DMACON changes
    /// who takes the slots after it. The CPU's fitted work can run many lines past such a write
    /// before `Machine::advance` delivers it (a wait under a BLTPRI blit, T54).
    pub(crate) pending: &'a [(i64, u16, u16)],
    pub(crate) passed: usize,
    /// The next frame's copper slots, run once the bus first reaches that frame: a CPU waiting
    /// out a blit across the frame's end meets the vertical blank's restart before the host has
    /// started the frame (T54).
    pub(crate) next_copper: Option<Vec<u32>>,
    /// The start of the last CPU access, before its wait for the bus: where the 68000 samples
    /// its IPL lines (Moira's `POLL_IPL`, before the access's bus check; T54).
    pub(crate) sampled_at: &'a mut u64,
}

impl Slots<'_> {
    /// Whether refresh, a bitplane or the copper takes the slot starting at cycle count `at`,
    /// slot `h` of `line`.
    fn fixed(&mut self, at: u64, line: u16, h: u16) -> bool {
        if is_refresh(h as u32)
            || (self.dmacon & (DMAEN | BPLEN) == (DMAEN | BPLEN) && self.fetch.takes(line, h))
        {
            return true;
        }
        match self.copper {
            Some((c, start)) => {
                let slot = (at as i64 - start).div_euclid(2);
                let frame = self.frame_lines as i64 * amiga_clock::LINE_SLOTS as i64;
                if (0..frame).contains(&slot) {
                    c.takes(slot as u32) || self.audio.takes(slot as u32)
                } else if (frame..2 * frame).contains(&slot) {
                    let next = (slot - frame) as u32;
                    self.next_copper_takes(next) || self.audio.takes_next(next)
                } else {
                    false
                }
            }
            None => false,
        }
    }

    /// Agnus runs the slot starting at cycle count `at`, at beam `(line, h)`, the CPU asking for
    /// the bus with BLS `bls`. Returns whether a chip has it.
    fn step(&mut self, at: u64, (line, h): (u16, u16), bls: bool) -> bool {
        self.pass_writes(at);
        let line = line % self.frame_lines;
        if self.sprites.line != Some(line) {
            self.sprites.enter(line, self.chip, self.custom);
        }
        let fixed = self.fixed(at, line, h)
            || (self.dmacon & (DMAEN | SPREN) == (DMAEN | SPREN) && self.sprites.takes(h));
        let pri = self.dmacon & BLTPRI != 0;
        let blit = match &mut self.blit {
            Some(b) if self.dmacon & (DMAEN | BLTEN) == (DMAEN | BLTEN) => {
                b.slot(at, fixed, bls, pri)
            }
            _ => false,
        };
        fixed || blit
    }

    /// Whether the next frame's copper takes its slot `slot`: restarted at COP1LC if copper DMA
    /// is on, from chip RAM as it is (the CPU is waiting, so nothing writes the list meanwhile).
    fn next_copper_takes(&mut self, slot: u32) -> bool {
        if self.dmacon & crate::copper::COPPER_DMA != crate::copper::COPPER_DMA {
            return false;
        }
        let bus = self.next_copper.get_or_insert_with(|| {
            let lc = crate::copper::COP1LC as usize >> 1;
            let pc = (self.custom[lc] as u32) << 16 | self.custom[lc + 1] as u32;
            crate::copper::frame_bus(self.chip, pc, self.fetch, self.frame_lines)
        });
        bus.binary_search(&slot).is_ok()
    }

    /// Apply the copper's writes made by the slot starting at cycle count `at`.
    fn pass_writes(&mut self, at: u64) {
        use amiga_chips::copper::reg;
        while let Some(&(t, r, v)) = self.pending.get(self.passed) {
            if t > at as i64 {
                break;
            }
            self.passed += 1;
            match r {
                reg::BPLCON0 => self.fetch.bplcon0 = v,
                reg::DIWSTRT => self.fetch.diwstrt = v,
                reg::DIWSTOP => self.fetch.diwstop = v,
                reg::DDFSTRT => self.fetch.ddfstrt = v,
                reg::DDFSTOP => self.fetch.ddfstop = v,
                crate::custom::DMACON => {
                    self.dmacon = crate::irq::set_clear(self.dmacon, v) & 0x07FF
                }
                _ => {}
            }
        }
    }

    /// Run the slots from `from` up to `to` (cycle counts) with no CPU access: the chips alone.
    pub fn idle(&mut self, from: u64, to: u64, beam: &dyn Fn(u64) -> (u16, u16)) {
        self.free(from, to, beam);
    }

    /// The slots `from`, `from + 2`, .. before `to`, none of which the CPU asks for. Only a running
    /// blit takes a free slot, so without one nothing but the sprites' line state moves: each line
    /// the slots reach is entered and the slots are not run one by one (the same result, and the
    /// machine's hot path, T49).
    fn free(&mut self, from: u64, to: u64, beam: &dyn Fn(u64) -> (u16, u16)) {
        let blitting = self.dmacon & (DMAEN | BLTEN) == (DMAEN | BLTEN)
            && self.blit.as_ref().is_some_and(|b| b.done_at.is_none());
        let mut t = from;
        if blitting {
            while t < to {
                self.step(t, beam(t), false);
                t += 2;
            }
            return;
        }
        while t < to {
            self.pass_writes(t);
            let (line, h) = beam(t);
            let line = line % self.frame_lines;
            if self.sprites.line != Some(line) {
                self.sprites.enter(line, self.chip, self.custom);
            }
            // The next line's first slot, on the same parity as `from`.
            t += 2 * (amiga_clock::LINE_SLOTS as u64).saturating_sub(h as u64).max(1);
        }
    }

    /// The CPU's work from cycle count `start`: instructions of (cycles, of them off the bus, of
    /// those before its bus accesses), in order, the first with `skip.0` of its bus accesses made
    /// already and, if `skip.1`, its leading idle cycles spent. Returns the count at which the work
    /// ends with the chips' slots taken out. An instruction's bus accesses are `(cycles - off) / 4`:
    /// its leading idle cycles come first (a branch's, `-(An)`'s; `pcode_rt::cycles::lead`), then
    /// the accesses, then its other cycles (Moira's register shifts prefetch then shift).
    pub fn cpu(
        &mut self,
        start: u64,
        work: &[(u32, u32, u32)],
        skip: (u64, bool),
        beam: &dyn Fn(u64) -> (u16, u16),
    ) -> u64 {
        let mut t = start;
        for (k, &(cycles, off, lead)) in work.iter().enumerate() {
            let (cycles, off) = (cycles as u64, (off as u64).min(cycles as u64));
            let lead = (lead as u64).min(off);
            let accesses = (cycles - off) / 4;
            let (done, led) = if k == 0 { (skip.0.min(accesses), skip.1) } else { (0, false) };
            if !led {
                t = self.cpu_idle(t, t + lead, beam);
            }
            t = self.accesses(t, accesses - done, beam);
            let end = t + (cycles - 4 * accesses - lead);
            t = self.cpu_idle(t, end, beam);
            // Time a host counts with no access (every 68000 instruction prefetches): the CPU is
            // taken to sample its IPL lines at its end.
            if accesses == 0 {
                *self.sampled_at = t;
            }
        }
        t
    }

    /// The CPU off the bus from cycle count `t` to `end`: the chips run those slots.
    pub fn cpu_idle(&mut self, t: u64, end: u64, beam: &dyn Fn(u64) -> (u16, u16)) -> u64 {
        // The slots starting before `end - 1`: a slot the CPU's time ends inside is the CPU's.
        self.free(t, end.saturating_sub(1), beam);
        end.max(t)
    }

    /// `n` CPU bus accesses from cycle count `t`; returns where they end.
    pub fn accesses(&mut self, mut t: u64, n: u64, beam: &dyn Fn(u64) -> (u16, u16)) -> u64 {
        for _ in 0..n {
            // Moira's `sync(2)` runs Agnus a slot, the access checks that slot (waiting a slot at
            // a time while a chip has it; vAmiga raises BLS once the CPU has waited two slots, so
            // the third is the first the blitter yields), then `sync(2)` runs one more.
            *self.sampled_at = t;
            let mut waited = 0;
            while self.step(t, beam(t), waited >= 3) {
                waited += 1;
                t += 2;
            }
            self.free(t + 2, t + 4, beam);
            t += 4;
        }
        t
    }
}
