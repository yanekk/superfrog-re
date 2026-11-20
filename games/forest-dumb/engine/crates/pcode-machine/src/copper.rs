//! The copper on the machine (translator T24). At each frame's start the list at COP1LC is run
//! for the frame with `amiga_chips::copper::run_bus`, reading chip RAM as it is at each word's
//! beam position, and its writes go through the custom registers at their cycles as the CPU's
//! time passes them (`Machine::advance`): a MOVE to INTREQ raises its interrupt at its line, a MOVE
//! to BLTSIZE starts a blit then.
//!
//! The run is a prediction: the CPU and the blitter may yet write a list word the copper has not
//! read. Such a write marks it stale ([`crate::journal::Watch`]) and it is run again, reading the
//! frame's journal, before its next write is delivered; the writes already delivered stay, since
//! nothing they read changed. A CPU strobe of COPJMP1/COPJMP2, or copper DMA switched on, starts a
//! new run there (`run_from`); copper DMA switched off drops the writes not yet made.

use std::cell::RefCell;

use amiga_chips::copper::{self, Beam, Fetch};

use crate::Machine;
use crate::beam;

pub const COP1LC: u16 = 0x080;
pub const COP2LC: u16 = 0x084;
pub const COPJMP1: u16 = 0x088;
pub const COPJMP2: u16 = 0x08A;

/// DMACON's DMAEN and COPEN.
pub const COPPER_DMA: u16 = 0x0280;

/// INTREQ's copper bit.
pub const INT_COPER: u16 = 0x0010;

/// One run of the copper in the current frame.
#[derive(Debug, Clone)]
pub(crate) struct Run {
    pc: u32,
    /// Where its first fetch is; `None` for the vertical blank's restart (cycle 0 taken).
    from: Option<(u16, u16)>,
    fetch: Fetch,
    /// Its writes: (cycle, register, value), in beam order.
    writes: Vec<(i64, u16, u16)>,
    /// The DMA slots it takes, as slots from the frame's start, in order (`dma`, T49).
    pub(crate) bus: Vec<u32>,
    next: usize,
}

/// The copper in the current frame.
#[derive(Debug, Clone)]
pub(crate) struct Copper {
    /// The cycle the frame began at (negative for a snapshot's first, part-run frame).
    pub frame_start: i64,
    run: Option<Run>,
    /// The last cycle a write of this frame was delivered at: later runs deliver only after it.
    delivered: i64,
}

impl Copper {
    /// Whether the copper takes DMA slot `slot` (from the frame's start) in this frame.
    pub fn takes(&self, slot: u32) -> bool {
        self.run
            .as_ref()
            .is_some_and(|r| r.bus.binary_search(&slot).is_ok())
    }

    /// The writes not yet delivered: (cycle, register, value), in beam order.
    pub(crate) fn pending(&self) -> &[(i64, u16, u16)] {
        self.run.as_ref().map_or(&[], |r| &r.writes[r.next..])
    }

    /// The cycle of the next write not yet delivered.
    pub fn next_at(&self) -> Option<i64> {
        let r = self.run.as_ref()?;
        r.writes.get(r.next).map(|w| w.0)
    }
}

/// The DMA slots (from the frame's start, ascending) the copper takes in a frame that restarts it
/// at `pc` with chip RAM `chip` and the display `fetch`: what [`Machine::start_frame`] will find,
/// for a frame the bus reaches before the host has started it (`dma::Slots`, T54).
pub(crate) fn frame_bus(chip: &[u8], pc: u32, fetch: Fetch, lines: u16) -> Vec<u32> {
    let mask = chip.len() as u32 - 1;
    let word = |a: u32, _: u16, _: u16| {
        let i = (a & mask & !1) as usize;
        u16::from_be_bytes([chip[i], chip[i + 1]])
    };
    let mut bus = Vec::new();
    copper::run_bus(&word, pc, fetch, lines, &mut bus);
    let mut bus: Vec<u32> = bus.iter().map(|b| b.dma()).collect();
    bus.sort_unstable();
    bus
}

impl Machine {
    fn cop_lc(&self, reg: u16) -> u32 {
        (self.custom[reg as usize >> 1] as u32) << 16 | self.custom[(reg as usize >> 1) + 1] as u32
    }

    /// Start the frame that begins at cycle `start`: the registers as the frame finds them, an
    /// empty journal, and the copper restarted at COP1LC if its DMA is on. Writes before
    /// `deliver_from` are not made (a snapshot's frame, part run already).
    pub(crate) fn start_frame(&mut self, start: i64, deliver_from: i64) {
        self.frame_custom = self.custom;
        self.frame_dmacon = self.dmacon;
        self.journal.clear();
        self.watch.clear();
        self.copper = Some(Copper {
            frame_start: start,
            run: None,
            delivered: deliver_from - 1,
        });
        if self.dmacon & COPPER_DMA == COPPER_DMA {
            self.copper_run(self.cop_lc(COP1LC), None);
        }
    }

    /// The copper starts at `pc`: at the vertical blank (`from` `None`) or at the beam now.
    pub(crate) fn copper_run(&mut self, pc: u32, from: Option<(u16, u16)>) {
        let run = Run {
            pc,
            from,
            fetch: self.fetch(),
            writes: Vec::new(),
            bus: Vec::new(),
            next: 0,
        };
        if let Some(c) = self.copper.as_mut() {
            c.run = Some(run);
        }
        self.copper_rerun();
    }

    /// The copper stops (its DMA switched off): no more writes this frame.
    pub(crate) fn copper_stop(&mut self) {
        if let Some(c) = self.copper.as_mut() {
            c.run = None;
        }
        self.watch.clear();
    }

    /// A CPU strobe of COPJMP1 or COPJMP2: the copper starts at that list now.
    pub(crate) fn copper_strobe(&mut self, lc: u16) {
        if self.dmacon & COPPER_DMA == COPPER_DMA {
            let (line, h) = self.beam(self.now);
            let b = Beam::from_slot(line, h).plus_slots(2);
            self.copper_run(self.cop_lc(lc), Some((b.line, b.slot())));
        }
    }

    /// Copper DMA switched on mid-frame: it starts at COP1LC there.
    pub(crate) fn copper_on(&mut self) {
        let (line, h) = self.beam(self.now);
        let b = Beam::from_slot(line, h).plus_slots(2);
        self.copper_run(self.cop_lc(COP1LC), Some((b.line, b.slot())));
    }

    /// Run the current run again from its start, reading chip RAM as the frame's journal says it
    /// was at each read; keep the writes after the last one delivered.
    pub(crate) fn copper_rerun(&mut self) {
        let Some(c) = self.copper.as_ref() else {
            return;
        };
        let Some(run) = c.run.as_ref() else {
            self.watch.clear();
            return;
        };
        let (fs, delivered) = (c.frame_start, c.delivered);
        let (pc, from, fetch) = (run.pc, run.from, run.fetch);
        let history = self.journal.history();
        let chip = &self.chip;
        let mask = chip.len() as u32 - 1;
        let reads = RefCell::new(Vec::new());
        let word = |a: u32, line: u16, h: u16| {
            let off = a & mask & !1;
            let at = fs + beam::offset(line, h) as i64;
            reads.borrow_mut().push((off, at));
            history.word(chip, off, at)
        };
        let lines = self.cfg.frame_lines;
        let mut bus = Vec::new();
        let out = match from {
            None => copper::run_bus(&word, pc, fetch, lines, &mut bus),
            Some((line, h)) => {
                copper::run_from_bus(&word, pc, Beam::from_slot(line, h), fetch, lines, &mut bus)
            }
        };
        let mut bus: Vec<u32> = bus.iter().map(|b| b.dma()).collect();
        bus.sort_unstable();
        let writes: Vec<(i64, u16, u16)> = out
            .iter()
            .map(|w| (fs + beam::offset(w.line, w.h) as i64, w.reg, w.value))
            .collect();
        let next = writes.partition_point(|w| w.0 <= delivered);
        let chip_len = self.chip.len();
        self.watch.set(chip_len, reads.into_inner());
        let run = self
            .copper
            .as_mut()
            .and_then(|c| c.run.as_mut())
            .expect("checked above");
        run.writes = writes;
        run.bus = bus;
        run.next = next;
    }

    /// Make the copper's next write, which is due.
    pub(crate) fn copper_deliver(&mut self) {
        let Some(c) = self.copper.as_mut() else {
            return;
        };
        let Some(run) = c.run.as_mut() else {
            return;
        };
        let (at, reg, v) = run.writes[run.next];
        run.next += 1;
        c.delivered = at;
        self.now = at.max(0) as u64;
        self.reg_write(reg, v, false);
    }
}
