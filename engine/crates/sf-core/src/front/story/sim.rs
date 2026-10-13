//! The CPU and the blitter on the bus, cycle by cycle, as the reference emulator runs them (T52):
//! what times the intro's draws, whose blits hold the CPU and run into the display.
//!
//! Each DMA cycle goes first to the fixed DMA ([`Fixed`]: refresh, the copper, the bitplanes), then
//! to the blitter, then to the CPU (vAmiga `Agnus::executeUntil` services its slots in that order;
//! the CPU asks for the cycle after them, `Agnus::executeUntilBusIsFree`).
//!
//! The blitter is vAmiga's `SlowBlitter` at accuracy level 2, copy blits without fill: BLTSIZE
//! written by the CPU in cycle `w` lands in `w + 1` and schedules `BLT_STRT1` for `w + 2`, which sets
//! the busy flag and waits for a cycle no DMA has (without taking it); `BLT_STRT2` the same, the blit
//! beginning there; its micro-program runs from the next cycle, one instruction per cycle it is not
//! held up in (`Blitter::exec`). The busy flag clears at the last word's `REPEAT`; the program's tail
//! (`HOLD_D`, D's last write) ends the blit.
//!
//! Without BLTPRI the blitter yields to a CPU that has waited two cycles (the BLS line): from its
//! third waiting cycle until it gets the bus. With BLTPRI it never does.
//!
//! Times are DMA cycles counted from the first frame's line 0, cycle 0, running on into the next
//! frames ([`FRAME`] cycles each).

use super::cpu::{self, LINE};
use crate::render::copper::{Beam, FRAME_LINES, Fetch, Write};

/// DMA cycles per (long PAL) frame.
pub const FRAME: u32 = FRAME_LINES as u32 * LINE;

/// DMACON's bits the blitter reads.
pub mod dmacon {
    pub const DMAEN: u16 = 0x0200;
    pub const BLTPRI: u16 = 0x0400;
    pub const BLTEN: u16 = 0x0040;
}

/// The cycles the fixed DMA takes: refresh (`$01`, `$03`, `$05`, `$E2`), the copper's
/// ([`crate::render::copper::run_bus`]) and the bitplanes' ([`Fetch::takes`]), frame after frame.
#[derive(Debug, Clone, Default)]
pub struct Fixed {
    frames: Vec<FixedFrame>,
}

/// One frame of [`Fixed`].
#[derive(Debug, Clone, Default)]
struct FixedFrame {
    /// The copper's cycles, in order.
    copper: Vec<u32>,
    /// The bitplane set-up from each cycle on: the frame's first, then after each copper write to
    /// one of its registers.
    fetch: Vec<(u32, Fetch)>,
}

impl Fixed {
    /// No frames yet: refresh only.
    pub fn new() -> Fixed {
        Fixed::default()
    }

    /// Append a frame: the copper's bus cycles and writes as `run_bus` gives them, and the bitplane
    /// set-up the frame starts with (the one `run_bus` was given).
    pub fn frame(mut self, copper: &[Beam], writes: &[Write], fetch: Fetch) -> Fixed {
        let at = |b: &Beam| b.line as u32 * LINE + b.h as u32;
        let mut f = fetch;
        let mut changes = vec![(0, f)];
        for w in writes {
            let before = f;
            f.write(w.reg, w.value);
            if f != before {
                // As `run_bus` has it: the write counts from the cycle after it.
                changes.push((at(&Beam { line: w.line, h: w.h }) + 1, f));
            }
        }
        self.frames.push(FixedFrame {
            copper: copper.iter().map(at).collect(),
            fetch: changes,
        });
        self
    }

    /// The bitplane set-up at cycle `t` of frame `k` (none past the frames given).
    fn fetch(&self, k: usize, t: u32) -> Option<Fetch> {
        let f = self.frames.get(k)?;
        let i = f.fetch.partition_point(|c| c.0 <= t);
        Some(f.fetch[i.max(1) - 1].1)
    }
}

impl cpu::Bus for Fixed {
    fn taken(&self, t: u32) -> bool {
        if cpu::refresh(t) {
            return true;
        }
        let (k, r) = ((t / FRAME) as usize, t % FRAME);
        let Some(f) = self.frames.get(k) else {
            return false;
        };
        f.copper.binary_search(&r).is_ok()
            || self
                .fetch(k, r)
                .is_some_and(|fe| fe.takes((r / LINE) as u16, (r % LINE) as u16))
    }
}

/// What a [`BlitEvent`] marks, as the oracle's (vAmiga `Blitter::beginBlit`, `clearBusyFlag`,
/// `endBlit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlitEventKind {
    Begin,
    Free,
    End,
}

/// A blitter event and its cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlitEvent {
    pub kind: BlitEventKind,
    pub t: u32,
}

/// A micro-instruction's demands on the bus (vAmiga `SlowBlitter`, OCS, no fill).
mod micro {
    /// Takes the bus (a fetch): waits for a free cycle and allocates it.
    pub const BUS: u8 = 1;
    /// Waits for a free cycle without taking it.
    pub const IDLE: u8 = 2;
    /// D's write: takes the bus, or only waits for a free cycle while D is locked (the first word).
    pub const WRITE_D: u8 = 4;
    /// The end of a word: back to the start, or past the last word the busy flag clears.
    pub const REPEAT: u8 = 8;
    /// The blit ends.
    pub const DONE: u8 = 16;
}

/// The copy programs by BLTCON0's channel bits (`USEA..USED`, bits 11..8), `copyBlitInstr[..][0][0]`.
fn program(channels: u16) -> &'static [u8] {
    use micro::*;
    const NOP: u8 = 0;
    match channels & 0xF {
        0x0 => &[IDLE, IDLE | REPEAT, NOP, DONE],
        0x1 => &[IDLE, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
        0x2 => &[IDLE, BUS | REPEAT, NOP, DONE],
        0x3 => &[IDLE, BUS, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
        0x4 => &[IDLE, BUS, IDLE | REPEAT, NOP, DONE],
        0x5 => &[IDLE, BUS, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
        0x6 => &[IDLE, BUS, BUS | REPEAT, NOP, DONE],
        0x7 => &[IDLE, BUS, BUS, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
        0x8 => &[BUS, IDLE | REPEAT, NOP, DONE],
        0x9 => &[BUS, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
        0xA => &[BUS, BUS | REPEAT, NOP, DONE],
        0xB => &[BUS, BUS, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
        0xC => &[BUS, BUS, IDLE | REPEAT, NOP, DONE],
        0xD => &[BUS, BUS, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
        0xE => &[BUS, BUS, BUS | REPEAT, NOP, DONE],
        _ => &[BUS, BUS, BUS, WRITE_D | REPEAT, NOP, WRITE_D | DONE],
    }
}

/// Where a blit is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// `BLT_STRT1` from cycle `.0`.
    Start1(u32),
    /// `BLT_STRT2` from cycle `.0`.
    Start2(u32),
    /// Running its program: the next instruction, from cycle `.1`.
    Run(usize, u32),
}

/// The blit in hand.
#[derive(Debug, Clone)]
struct Blit {
    stage: Stage,
    program: &'static [u8],
    /// Words left, counting the current one.
    words: u32,
    /// D's first write only waits (`lockD`).
    lock_d: bool,
}

/// The CPU and the blitter on the fixed DMA's bus.
pub struct Sim<'a> {
    fixed: &'a dyn cpu::Bus,
    /// The DMA cycle the CPU has reached.
    pub t: u32,
    /// The next cycle the blitter has not been run for.
    next: u32,
    dmacon: u16,
    /// DMACON writes not landed yet: (cycle they land, value written).
    dmacon_at: Vec<(u32, u16)>,
    /// BLTCON0 as the blitter will read it at its next start.
    bltcon0: u16,
    blit: Option<Blit>,
    busy: bool,
    /// The cycles the blitter took, in order.
    taken: Vec<u32>,
    /// The blitter's begin, free and end events, in order.
    pub events: Vec<BlitEvent>,
    /// The cycles of the CPU's writes, in order.
    pub writes: Vec<u32>,
    /// The CPU's custom-register writes ([`Sim::write_reg`]), in order: (cycle, register, value).
    pub regs: Vec<(u32, u16, u16)>,
    /// The cycle the CPU's last access asked for the bus in (before any wait for it).
    pub asked: u32,
}

impl<'a> Sim<'a> {
    /// The CPU at cycle `t`, DMACON as `dmacon`, the blitter idle.
    pub fn new(t: u32, fixed: &'a dyn cpu::Bus, dmacon: u16) -> Sim<'a> {
        Sim {
            fixed,
            t,
            next: t,
            dmacon,
            dmacon_at: Vec::new(),
            bltcon0: 0,
            blit: None,
            busy: false,
            taken: Vec::new(),
            events: Vec::new(),
            writes: Vec::new(),
            regs: Vec::new(),
            asked: t,
        }
    }

    fn bltdma(&self) -> bool {
        use dmacon::*;
        self.dmacon & (DMAEN | BLTEN) == DMAEN | BLTEN
    }

    /// Run the blitter for cycle `c` (the next one it has not run), the CPU holding the BLS line
    /// when `bls`.
    fn cycle(&mut self, c: u32, bls: bool) {
        debug_assert_eq!(c, self.next);
        self.next = c + 1;
        // DMACON lands at the cycle's start (vAmiga's register slot comes first).
        while let Some(&(at, v)) = self.dmacon_at.first() {
            if at > c {
                break;
            }
            self.dmacon = if v & 0x8000 != 0 {
                self.dmacon | v & 0x7ff
            } else {
                self.dmacon & !v & 0x7ff
            };
            self.dmacon_at.remove(0);
        }
        let dma = self.bltdma();
        let yields = bls && self.dmacon & dmacon::BLTPRI == 0;
        let free = dma && !yields && !self.fixed.taken(c);
        let Some(b) = self.blit.as_mut() else {
            return;
        };
        match b.stage {
            Stage::Start1(from) if c >= from => {
                // `prepareBlit` sets the busy flag each time the event fires.
                self.busy = true;
                if free {
                    b.stage = Stage::Start2(c + 1);
                }
            }
            Stage::Start2(from) if c >= from => {
                if free {
                    b.stage = Stage::Run(0, c + 1);
                    self.events.push(BlitEvent {
                        kind: BlitEventKind::Begin,
                        t: c,
                    });
                }
            }
            Stage::Run(pc, from) if c >= from => {
                use micro::*;
                let op = b.program[pc];
                let (bus, idle) = if op & WRITE_D != 0 {
                    (!b.lock_d, b.lock_d)
                } else {
                    (op & BUS != 0, op & IDLE != 0)
                };
                if (bus || idle) && !free {
                    return;
                }
                if bus {
                    self.taken.push(c);
                }
                let mut pc = pc + 1;
                if op & REPEAT != 0 {
                    b.lock_d = false;
                    if b.words > 1 {
                        b.words -= 1;
                        pc = 0;
                    } else {
                        self.busy = false;
                        self.events.push(BlitEvent {
                            kind: BlitEventKind::Free,
                            t: c,
                        });
                    }
                }
                if op & DONE != 0 {
                    self.events.push(BlitEvent {
                        kind: BlitEventKind::End,
                        t: c,
                    });
                    self.blit = None;
                } else {
                    b.stage = Stage::Run(pc, c + 1);
                }
            }
            _ => {}
        }
    }

    /// Run the blitter up to and including cycle `c`.
    fn run_to(&mut self, c: u32) {
        while self.next <= c {
            self.cycle(self.next, false);
        }
    }

    /// One CPU access: its bus cycle. The blitter runs first in every cycle; from the CPU's third
    /// cycle of waiting it yields unless BLTPRI is set (vAmiga `Agnus::executeUntilBusIsFree`).
    fn access(&mut self) -> u32 {
        self.t += 1;
        self.run_to(self.t);
        let asked = self.t;
        self.asked = asked;
        while self.fixed.taken(self.t) || self.blitter_took(self.t) {
            self.t += 1;
            self.cycle(self.t, self.t - asked >= 3);
        }
        let at = self.t;
        self.t += 1;
        at
    }

    /// Run the accesses and idle cycles `ops` ([`cpu`]'s letters `p`, `r`, `w`, `n`).
    pub fn run(&mut self, ops: &str) {
        for c in ops.bytes() {
            match c {
                b'n' => self.t += 1,
                b'p' | b'r' => {
                    self.access();
                }
                b'w' => {
                    let at = self.access();
                    self.writes.push(at);
                }
                _ => {}
            }
        }
    }

    /// `n` idle DMA cycles.
    pub fn idle(&mut self, n: u32) {
        self.t += n;
    }

    /// Whether the instruction that has just ended takes an interrupt that can be taken from cycle
    /// `from` (a blank's: its line 0, cycle 10): the CPU looks at the request when its last access
    /// asks for the bus, so one held off the bus across `from` (by a blit) ends without it and the
    /// next takes it (oracle, T54: frame 3 025's draw).
    pub fn irq_seen(&self, from: u32) -> bool {
        self.asked + 1 >= from
    }

    /// A read of DMACONR as an instruction's data access (`r`): whether the blitter was busy then.
    pub fn read_busy(&mut self) -> bool {
        self.access();
        self.busy
    }

    /// The blitter's busy flag as of the CPU's last access.
    pub fn busy(&self) -> bool {
        self.busy
    }

    /// A custom-register write as an instruction's data access (`w`): BLTCON0, BLTSIZE and
    /// DMACON reach the blitter; other registers only take the cycle.
    pub fn write_reg(&mut self, reg: u16, value: u16) -> u32 {
        let at = self.access();
        self.writes.push(at);
        self.regs.push((at, reg, value));
        self.poke(at, reg, value);
        at
    }

    /// A custom-register write the CPU made in cycle `at` (on or before its clock), as
    /// [`Sim::write_reg`] without the access: for a write timed elsewhere (the original's).
    pub fn poke(&mut self, at: u32, reg: u16, value: u16) {
        self.run_to(at);
        match reg {
            0x040 => self.bltcon0 = value,
            0x058 => {
                let h = match value & 0x3f {
                    0 => 64,
                    h => h as u32,
                };
                let v = match value >> 6 {
                    0 => 1024,
                    v => v as u32,
                };
                self.blit = Some(Blit {
                    stage: Stage::Start1(at + 2),
                    program: program(self.bltcon0 >> 8),
                    words: h * v,
                    lock_d: true,
                });
            }
            0x096 => self.dmacon_at.push((at + 2, value)),
            _ => {}
        }
    }

    /// Run the blitter alone until its blit ends (or `limit` cycles on): the CPU's clock moves
    /// with it, as a CPU stopped off the bus.
    pub fn finish(&mut self, limit: u32) {
        let end = self.next + limit;
        while self.blit.is_some() && self.next < end {
            self.cycle(self.next, false);
        }
        self.t = self.t.max(self.next);
    }

    /// Whether the blitter took cycle `c` (one it has been run for).
    pub fn blitter_took(&self, c: u32) -> bool {
        self.taken.binary_search(&c).is_ok()
    }

    /// Whether a blit is pending or running.
    pub fn blitting(&self) -> bool {
        self.blit.is_some()
    }
}

/// One step of a [`Prog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Accesses and idle cycles ([`cpu`]'s letters `p`, `r`, `w`, `n`).
    Ops(&'static str),
    /// `n` idle DMA cycles.
    Idle(u32),
    /// A custom-register write (a `w` that reaches [`Sim::write_reg`]).
    Reg(u16, u16),
    /// `btst #14,DMACONR` (`ppprp`, the `r` reading BLTBUSY), then `bne` back to it while busy:
    /// taken `npp`, not taken `nnp`, or `nnpp` for a word displacement (`true`).
    WaitBlit(bool),
    /// An instruction's end: where an interrupt can come in.
    End,
}

/// A run of the CPU's instructions, branch for branch, as their bus accesses: built by the code
/// it stands for (`bobs::draw`) beside its memory work, then run on a [`Sim`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prog {
    pub steps: Vec<Step>,
}

impl Prog {
    pub fn new() -> Prog {
        Prog::default()
    }

    /// One instruction of `letters`, each `W` a write of the next of `regs`.
    pub fn ins_regs(&mut self, letters: &'static str, regs: &[(u16, u16)]) {
        let mut regs = regs.iter();
        let mut from = 0;
        for (i, c) in letters.bytes().enumerate() {
            if c == b'W' {
                if from < i {
                    self.steps.push(Step::Ops(&letters[from..i]));
                }
                let &(reg, value) = regs.next().expect("a register for each W");
                self.steps.push(Step::Reg(reg, value));
                from = i + 1;
            }
        }
        if from < letters.len() {
            self.steps.push(Step::Ops(&letters[from..]));
        }
        self.steps.push(Step::End);
    }

    /// One instruction of `letters`.
    pub fn ins(&mut self, letters: &'static str) {
        self.ins_regs(letters, &[]);
    }

    /// One instruction of `letters`, then `idle` DMA cycles of internal work (shifts, `mulu`).
    pub fn ins_idle(&mut self, letters: &'static str, idle: u32) {
        self.steps.push(Step::Ops(letters));
        self.steps.push(Step::Idle(idle));
        self.steps.push(Step::End);
    }

    /// A conditional branch: taken `npp`; not taken `nnp`, `nnpp` with a word displacement.
    pub fn branch(&mut self, taken: bool, word: bool) {
        self.ins(match (taken, word) {
            (true, _) => "npp",
            (false, false) => "nnp",
            (false, true) => "nnpp",
        });
    }

    /// A wait for the blitter: [`Step::WaitBlit`].
    pub fn wait_blit(&mut self, word: bool) {
        self.steps.push(Step::WaitBlit(word));
    }

    /// The step of the first write of `value` to `reg`.
    pub fn find_reg(&self, reg: u16, value: u16) -> Option<usize> {
        self.steps.iter().position(|s| *s == Step::Reg(reg, value))
    }

    /// Run the steps from `from` on `sim`; `end` is called at each instruction's end (where an
    /// interrupt can come in).
    pub fn run(&self, sim: &mut Sim, from: usize, end: &mut dyn FnMut(&mut Sim)) {
        for step in &self.steps[from..] {
            match *step {
                Step::Ops(o) => sim.run(o),
                Step::Idle(n) => sim.idle(n),
                Step::Reg(reg, value) => {
                    sim.write_reg(reg, value);
                }
                Step::WaitBlit(word) => loop {
                    sim.run("ppp");
                    let busy = sim.read_busy();
                    sim.run("p");
                    end(sim);
                    sim.run(match (busy, word) {
                        (true, _) => "npp",
                        (false, false) => "nnp",
                        (false, true) => "nnpp",
                    });
                    end(sim);
                    if !busy {
                        break;
                    }
                },
                Step::End => end(sim),
            }
        }
    }
}

/// A cycle as (line, DMA cycle) of its frame, and how many frames on.
pub fn beam(t: u32) -> (u32, u16, u16) {
    let r = t % FRAME;
    (t / FRAME, (r / LINE) as u16, (r % LINE) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::copper::reg;

    struct Free;
    impl cpu::Bus for Free {
        fn taken(&self, _: u32) -> bool {
            false
        }
    }

    const ON: u16 = dmacon::DMAEN | dmacon::BLTEN | dmacon::BLTPRI;

    /// Begin, free, end of one blit of BLTCON0 `con0` and BLTSIZE `size` written at cycle 100.
    fn blit(bus: &dyn cpu::Bus, dma: u16, con0: u16, size: u16) -> Vec<u32> {
        let mut s = Sim::new(0, bus, dma);
        s.poke(90, 0x40, con0);
        s.poke(100, 0x58, size);
        s.finish(1 << 20);
        assert!(!s.blitting());
        s.events.iter().map(|e| e.t).collect()
    }

    #[test]
    fn on_a_free_bus_a_word_takes_one_cycle_per_channel() {
        // BLTSIZE at 100: STRT1 at 102, STRT2 and the begin at 103, the program from 104.
        // ABCD: four cycles a word; free at the last word's REPEAT, the end two cycles on.
        for (n, size) in [(1, 1 << 6 | 1), (6, 2 << 6 | 3), (64 * 1024, 0)] {
            assert_eq!(
                blit(&Free, ON, 0x0fca, size),
                [103, 103 + 4 * n, 105 + 4 * n],
                "ABCD, {n} words"
            );
        }
        // ABD three, AD two; the first word's D write (locked) waits but takes nothing.
        for (n, size) in [(1, 1 << 6 | 1), (336, 0xe06), (20 * 1024, 0x14)] {
            assert_eq!(
                blit(&Free, ON, 0x0dfc, size),
                [103, 103 + 3 * n, 105 + 3 * n]
            );
            assert_eq!(
                blit(&Free, ON, 0x09f0, size),
                [103, 103 + 2 * n, 105 + 2 * n]
            );
        }
        // A alone (no D): the tail has no write.
        assert_eq!(blit(&Free, ON, 0x08f0, 0x41), [103, 105, 107]);
    }

    #[test]
    fn every_program_has_the_hrm_word_length_and_bus_cycles() {
        // Cycles a word, by USEA..USED, from the HRM table in vAmiga's `SlowBlitter.cpp`.
        const WORD: [u32; 16] = [2, 2, 2, 3, 3, 3, 3, 4, 2, 2, 2, 3, 3, 3, 3, 4];
        for ch in 0..16u16 {
            for n in [1u32, 5] {
                let mut s = Sim::new(0, &Free, ON);
                s.poke(90, 0x40, ch << 8);
                s.poke(100, 0x58, 1 << 6 | n as u16);
                s.finish(1000);
                let t: Vec<u32> = s.events.iter().map(|e| e.t).collect();
                let w = WORD[ch as usize];
                assert_eq!(
                    t,
                    [103, 103 + w * n, 105 + w * n],
                    "channels {ch:x}, {n} words"
                );
                // A, B, C fetch every word; D writes every word but the first, then once more.
                let fetches = (ch >> 1).count_ones() * n + (ch & 1) as u32 * n;
                assert_eq!(s.taken.len() as u32, fetches, "channels {ch:x}, {n} words");
            }
        }
    }

    #[test]
    fn the_blitter_waits_for_the_fixed_dma_and_dma_on() {
        // Refresh at $01, $03, $05: written at 0, the start waits for 2, 4 and 6.
        let refresh = Fixed::new();
        let mut s = Sim::new(0, &refresh, ON);
        s.poke(0, 0x40, 0x09f0);
        s.poke(0, 0x58, 1 << 6 | 1);
        s.finish(1000);
        let t: Vec<u32> = s.events.iter().map(|e| e.t).collect();
        // STRT1 at 2 (free), STRT2 at 3 (refresh: waits), 4 (begins); A at 5 (taken) 6, D at 7.
        assert_eq!(t, [4, 7, 9]);
        // Blitter DMA off: nothing starts until DMACON turns it on, two cycles after its write.
        let mut s = Sim::new(0, &Free, dmacon::DMAEN);
        s.poke(10, 0x40, 0x09f0);
        s.poke(10, 0x58, 1 << 6 | 1);
        s.poke(50, 0x96, 0x8000 | dmacon::BLTEN);
        s.finish(1000);
        assert_eq!(s.events[0].t, 53);
    }

    #[test]
    fn the_copper_and_the_bitplanes_take_their_cycles() {
        let fetch = Fetch {
            bplcon0: 0x5200,
            diwstrt: 0x2c81,
            diwstop: 0x2cc1,
            ddfstrt: 0x38,
            ddfstop: 0xd0,
        };
        let copper = [Beam { line: 0, h: 0x10 }, Beam { line: 0, h: 0x12 }];
        let writes = [Write {
            line: 0x30,
            h: 0x10,
            reg: reg::BPLCON0,
            value: 0x0200,
        }];
        let f = Fixed::new().frame(&copper, &writes, fetch);
        use cpu::Bus;
        assert!(f.taken(0x10) && f.taken(0x12) && !f.taken(0x14));
        // Line $2c, five lores planes: the fifth's cycle in the unit from $38 is $3e.
        let at = |line: u32, h: u32| line * LINE + h;
        assert!(f.taken(at(0x2c, 0x3e)) && !f.taken(at(0x2c, 0x3c)));
        // BPLCON0 to no planes from line $30: no fetch after the write.
        assert!(!f.taken(at(0x30, 0x3e)) && f.taken(at(0x2f, 0x3e)));
        // Past the frames given: refresh only.
        assert!(f.taken(FRAME + 1) && !f.taken(FRAME + 0x10));
    }

    #[test]
    fn without_bltpri_the_blitter_yields_to_a_cpu_that_waited_two_cycles() {
        let dma = dmacon::DMAEN | dmacon::BLTEN;
        // AD, eight words, BLTSIZE written at 100: the begin at 103, A at 104, the first D (locked)
        // waits at 105 without taking it, then every cycle from 106 to 119 is the blitter's.
        let start = |pri: u16| {
            let mut s = Sim::new(100, &Free, dma | pri);
            s.poke(100, 0x40, 0x09f0);
            s.poke(100, 0x58, 1 << 6 | 8);
            s.idle(10);
            s
        };
        // The CPU asks for 111: taken, and 112 and 113 (BLS raised after 113); 114 is its.
        let mut s = start(0);
        s.run("r");
        assert_eq!(s.t, 115);
        // The blit lost that cycle and takes the next: free at 120, not 119.
        s.finish(1000);
        assert!(!s.blitter_took(114) && s.blitter_took(113) && s.blitter_took(115));
        let t: Vec<u32> = s.events.iter().map(|e| e.t).collect();
        assert_eq!(t, [103, 120, 122]);
        // With BLTPRI the CPU waits for the tail's idle cycle, 120, after the free at 119.
        let mut s = start(dmacon::BLTPRI);
        s.run("r");
        assert_eq!(s.t, 121);
        assert!(!s.busy());
        s.finish(1000);
        let t: Vec<u32> = s.events.iter().map(|e| e.t).collect();
        assert_eq!(t, [103, 119, 121]);
        // A CPU that finds the bus free at once is never held: the locked D's cycle, 105.
        let mut s = start(0);
        s.t = 104;
        s.run("r");
        assert_eq!(s.t, 106);
    }
}
