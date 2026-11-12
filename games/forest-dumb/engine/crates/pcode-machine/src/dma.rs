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
//! Not taken: the copper's instruction fetches, sprite, disk and audio slots. The copper takes
//! even slots only between WAITs; sprites, disk and audio take odd slots, which the CPU, running
//! on even slots after its first wait, rarely wants (the oracle's busy-copper reading is 0.4% over
//! the plain one, `contention.rs`).

use amiga_chips::copper::Fetch;
use amiga_clock::is_refresh;

/// DMACON's bits the bus depends on.
pub const DMAEN: u16 = 0x0200;
pub const BPLEN: u16 = 0x0100;
pub const BLTEN: u16 = 0x0040;
pub const BLTPRI: u16 = 0x0400;

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
    /// Set when BBUSY clears (the last word's REPEAT).
    pub(crate) free_at: Option<u64>,
    /// Set when BLTDONE runs: the interrupt follows a slot later.
    pub(crate) done_at: Option<u64>,
}

impl BlitRun {
    /// A blit of `rows` rows of `words` words under BLTCON0 `con0`.
    pub(crate) fn new(con0: u16, rows: u16, words: u16) -> BlitRun {
        BlitRun {
            program: PROGRAMS[(con0 >> 8 & 15) as usize],
            pc: 0,
            width: words,
            x: words,
            y: rows,
            lock_d: true,
            stage: Stage::Start1,
            free_at: None,
            done_at: None,
        }
    }

    /// The blitter's turn in slot `at` (its start in CPU cycles), `taken` by a fixed DMA channel
    /// or not, with the CPU asking for it (`bls`) and BLTPRI `pri`. Returns whether the blitter
    /// took the bus.
    fn slot(&mut self, at: u64, taken: bool, bls: bool, pri: bool) -> bool {
        if self.done_at.is_some() {
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
}

impl Slots<'_> {
    /// Whether refresh or a bitplane takes slot `h` of `line`.
    fn fixed(&self, line: u16, h: u16) -> bool {
        is_refresh(h as u32)
            || (self.dmacon & (DMAEN | BPLEN) == (DMAEN | BPLEN) && self.fetch.takes(line, h))
    }

    /// Agnus runs the slot starting at cycle count `at`, at beam `(line, h)`, the CPU asking for
    /// the bus with BLS `bls`. Returns whether a chip has it.
    fn step(&mut self, at: u64, (line, h): (u16, u16), bls: bool) -> bool {
        let line = line % self.frame_lines;
        let fixed = self.fixed(line, h);
        let pri = self.dmacon & BLTPRI != 0;
        let blit = match &mut self.blit {
            Some(b) if self.dmacon & (DMAEN | BLTEN) == (DMAEN | BLTEN) => {
                b.slot(at, fixed, bls, pri)
            }
            _ => false,
        };
        fixed || blit
    }

    /// Run the slots from `from` up to `to` (cycle counts) with no CPU access: the chips alone.
    pub fn idle(&mut self, from: u64, to: u64, beam: &dyn Fn(u64) -> (u16, u16)) {
        let mut t = from;
        while t < to {
            self.step(t, beam(t), false);
            t += 2;
        }
    }

    /// The CPU's work from cycle count `start`: instructions of (cycles, of them off the bus), in
    /// order. Returns the count at which the work ends with the chips' slots taken out. An
    /// instruction's bus accesses are `(cycles - off) / 4`, made first, its other cycles after
    /// them (Moira's register shifts prefetch then shift; some instructions idle first, which this
    /// does not tell apart).
    pub fn cpu(&mut self, start: u64, work: &[(u32, u32)], beam: &dyn Fn(u64) -> (u16, u16)) -> u64 {
        let mut t = start;
        for &(cycles, off) in work {
            let (cycles, off) = (cycles as u64, (off as u64).min(cycles as u64));
            let accesses = (cycles - off) / 4;
            for _ in 0..accesses {
                // Moira's `sync(2)` runs Agnus a slot, the access checks that slot (waiting a
                // slot at a time while a chip has it; vAmiga raises BLS once the CPU has waited
                // two slots, so the third is the first the blitter yields), then `sync(2)` runs
                // one more.
                let mut waited = 0;
                while self.step(t, beam(t), waited >= 3) {
                    waited += 1;
                    t += 2;
                }
                t += 2;
                self.step(t, beam(t), false);
                t += 2;
            }
            let spare = cycles - 4 * accesses;
            let end = t + spare;
            while t + 2 <= end {
                self.step(t, beam(t), false);
                t += 2;
            }
            t = end;
        }
        t
    }
}
