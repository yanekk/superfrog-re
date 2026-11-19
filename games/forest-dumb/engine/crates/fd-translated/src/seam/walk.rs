//! A walk of the original's instructions for their time alone (translator T56, DESIGN §2.9).
//!
//! A hand-written routine mounted in place of a translated one spends the original's time by
//! walking the path the original takes through its instructions, each with its generated
//! [`Shape`]: the same `block_start`/`insn_start`, tick and `bus.sync` calls in the same order as the
//! generated code makes them, so the machine fits the same accesses onto the same bus slots. Only
//! the custom-chip accesses are made for real, by the instruction that makes them: a write takes
//! its value from what the hand engine wrote ([`Walk::write`]); a read reads the machine.
//!
//! The path is a list of [`Decision`]s, one at each conditional branch, `dbcc`, data-dependent
//! time and computed call, in the order the original meets them; a timing rule works it out from
//! the state the routine reads (`seam::rules`). A wait on the chips ([`Decision::Busy`],
//! [`Decision::Same`]) is decided by the reads the walk made, and stays at the queue's front until
//! the loop ends.

use std::collections::VecDeque;

use pcode_rt::shape::{self, Flow, Shape};
use pcode_rt::{Bus, Image, Regs, Stop, block_start, insn_start};

/// The custom chips' base.
const CHIPS: u32 = 0xdf_f000;

/// The decision at the next point where the original's path forks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// A conditional branch taken or not.
    Branch(bool),
    /// `dbcc`'s counter (its register's low word) before the instruction: the loop goes on unless
    /// it is 0.
    Count(u16),
    /// A data register's value before an instruction whose time it sets (a shift or multiply by a
    /// register).
    Reg(u8, u32),
    /// A `btst #6, $dff002` loop's branch: taken while the last DMACONR byte read has BBUSY set.
    Busy,
    /// A `cmp.b $dff006, d1` loop's branch: taken while the last two VHPOSR byte reads are equal.
    Same,
    /// A computed call's target, hunk 0 offset.
    Call(u32),
}

/// Where a [`Walk::run`] stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Until {
    /// At a custom write no value has been handed for, before the instruction; or at the return
    /// of a function the outermost one called.
    Write,
    /// Just after the read of this custom register.
    Read(u16),
    /// Before the outermost function's call of the function at this offset.
    Call(u32),
    /// At the outermost function's return.
    End,
}

/// What stopped a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The custom write at the next instruction wants a value (`Until::Write`).
    Write(u16),
    /// The register read and its value (`Until::Read`).
    Read(u16, u16),
    /// Before the call (`Until::Call`).
    Call,
    /// Back in the outermost function from one it called (`Until::Write`).
    Returned,
    /// The outermost function returned.
    Done,
}

struct Frame {
    hunk: u32,
    shapes: &'static [Shape],
    pc: u32,
}

/// A walk through the original's code, from a function's entry.
pub struct Walk {
    frames: Vec<Frame>,
    /// Control jumped (a branch taken, a call, a return): the next instruction is a block start.
    jumped: bool,
    decisions: VecDeque<Decision>,
    /// Custom writes handed by the hand engine, in order: (register, value, size in bytes).
    writes: VecDeque<(u16, u32, u8)>,
    /// The last two custom reads, newest last.
    reads: [(u16, u16); 2],
    /// A custom write with no value handed writes 0: timing alone, as the rules' check runs it.
    free: bool,
    shapes: fn((u32, u32)) -> Option<&'static [Shape]>,
}

impl Walk {
    /// A walk entering the function at `entry`, with its instructions' shapes from `shapes` (the
    /// tree's).
    pub fn new(entry: (u32, u32), shapes: fn((u32, u32)) -> Option<&'static [Shape]>) -> Walk {
        let mut w = Walk {
            frames: Vec::new(),
            jumped: true,
            decisions: VecDeque::new(),
            writes: VecDeque::new(),
            reads: [(0, 0); 2],
            free: false,
            shapes,
        };
        w.frames.push(Frame {
            hunk: entry.0,
            shapes: shapes(entry).unwrap_or(&[]),
            pc: entry.1,
        });
        w
    }

    /// The path onward: decisions in the order the original meets them.
    pub fn decide(&mut self, d: impl IntoIterator<Item = Decision>) {
        self.decisions.extend(d);
    }

    /// A custom write the hand engine made, for the instruction of the original that makes it.
    pub fn write(&mut self, reg: u16, value: u32, size: u8) {
        self.writes.push_back((reg & 0x1ff, value, size));
    }

    /// From now on a custom write no value was handed for writes 0 (the rules' check, where no
    /// hand engine runs).
    pub fn free_writes(&mut self) {
        self.free = true;
    }

    /// Decisions not yet taken and writes not yet made: what a rule or the hand engine handed and
    /// the original's path did not use.
    pub fn left(&self) -> (usize, usize) {
        (self.decisions.len(), self.writes.len())
    }

    /// Whether the walk is past the outermost function's return.
    pub fn done(&self) -> bool {
        self.frames.is_empty()
    }

    /// The function the walk is in and its next instruction.
    pub fn at(&self) -> Option<(u32, u32)> {
        self.frames.last().map(|f| (f.hunk, f.pc))
    }

    fn fail(&self, img: &Image, why: &str) -> Stop {
        let at = self
            .at()
            .map_or("the end".to_string(), |(h, o)| img.describe(img.h(h, o)));
        Stop::Unsupported {
            op: format!("seam walk: {why}"),
            at,
        }
    }

    fn next_decision(&mut self, img: &Image, what: &str) -> Result<Decision, Stop> {
        self.decisions
            .pop_front()
            .ok_or_else(|| self.fail(img, &format!("no decision left for {what}")))
    }

    /// Walk on until `until`.
    pub fn run(
        &mut self,
        bus: &mut dyn Bus,
        r: &mut Regs,
        img: &Image,
        until: Until,
    ) -> Result<Event, Stop> {
        loop {
            let Some(f) = self.frames.last() else {
                return Ok(Event::Done);
            };
            let hunk = f.hunk;
            let s = *shape::find(f.shapes, f.pc)
                .ok_or_else(|| self.fail(img, "no translated instruction here"))?;
            if s.uncounted {
                return Err(self.fail(img, "an instruction whose time is not counted"));
            }
            if let Flow::Call { to } = s.flow
                && until == Until::Call(to)
                && self.frames.len() == 1
            {
                if !self.writes.is_empty() {
                    return Err(self.fail(
                        img,
                        "the engine wrote a custom register the original does not",
                    ));
                }
                return Ok(Event::Call);
            }
            let custom = s.custom;
            if let Some((reg, size, true)) = custom
                && self.writes.is_empty()
                && self.free
            {
                self.writes.push_back((reg, 0, size));
            }
            if let Some((reg, _, true)) = custom
                && self.writes.is_empty()
            {
                if until == Until::Write {
                    return Ok(Event::Write(reg));
                }
                return Err(self.fail(
                    img,
                    &format!(
                        "the original writes $dff{reg:03x} here, the engine made no such write"
                    ),
                ));
            }
            // Where the generated code starts the instruction (`pcode_rust.py`, `Emit::function`).
            if self.jumped {
                block_start(bus, r, img)?;
                if !s.first {
                    insn_start(bus, r, img)?;
                }
            } else if s.first {
                block_start(bus, r, img)?;
            } else {
                insn_start(bus, r, img)?;
            }
            self.jumped = false;
            if s.dynamic {
                match self.next_decision(img, "a register a time hangs on")? {
                    Decision::Reg(n, v) => r.set_d(n as usize, v),
                    d => return Err(self.fail(img, &format!("{d:?} where a register is due"))),
                }
            }
            // The tick, but a conditional branch's, which hangs on the way it goes.
            let mut taken = false;
            match s.flow {
                Flow::Bcc { taken: tk, .. } => {
                    let d = *self
                        .decisions
                        .front()
                        .ok_or_else(|| self.fail(img, "no decision left for a branch"))?;
                    taken = match d {
                        Decision::Branch(b) => b,
                        Decision::Busy => self.reads[1].1 & 0x40 != 0,
                        Decision::Same => self.reads[0].1 == self.reads[1].1,
                        d => return Err(self.fail(img, &format!("{d:?} at a branch"))),
                    };
                    // A wait stays at the front until its loop ends.
                    if matches!(d, Decision::Branch(_)) || !taken {
                        self.decisions.pop_front();
                    }
                    if taken { tk(r) } else { (s.tick)(r) }
                }
                Flow::Dbcc { reg, .. } => {
                    let c = match self.next_decision(img, "a dbcc")? {
                        Decision::Count(c) => c,
                        d => return Err(self.fail(img, &format!("{d:?} at a dbcc"))),
                    };
                    let d = r.d(reg as usize);
                    r.set_d(reg as usize, d & 0xffff_0000 | c as u32);
                    (s.tick)(r);
                    r.set_d(reg as usize, d & 0xffff_0000 | c.wrapping_sub(1) as u32);
                    taken = c != 0;
                }
                _ => (s.tick)(r),
            }
            if s.mem {
                bus.sync(r);
            }
            let mut read = None;
            if let Some((reg, size, write)) = custom {
                let a = CHIPS + reg as u32;
                if write {
                    let (want, v, n) = self.writes.pop_front().expect("a write, checked above");
                    if want != reg || n != size {
                        return Err(self.fail(
                            img,
                            &format!(
                                "the original writes $dff{reg:03x} ({size} bytes), the engine $dff{want:03x} ({n})"
                            ),
                        ));
                    }
                    match size {
                        1 => bus.wb(a, v as u8),
                        4 => bus.wl(a, v),
                        _ => bus.ww(a, v as u16),
                    }
                } else {
                    let v = match size {
                        1 => bus.rb(a) as u16,
                        _ => bus.rw(a),
                    };
                    self.reads = [self.reads[1], (reg, v)];
                    read = Some((reg, v));
                }
            }
            let next = s.next();
            match s.flow {
                Flow::Next => self.top().pc = next,
                Flow::Bcc { to, .. } | Flow::Dbcc { to, .. } => {
                    self.top().pc = if taken { to } else { next };
                    self.jumped = taken;
                }
                Flow::Jump { to } => {
                    self.top().pc = to;
                    self.jumped = true;
                }
                Flow::Call { to } => self.call(hunk, to, next, img)?,
                Flow::CallInd => match self.next_decision(img, "a computed call")? {
                    Decision::Call(to) => self.call(hunk, to, next, img)?,
                    d => return Err(self.fail(img, &format!("{d:?} at a computed call"))),
                },
                Flow::JumpInd => return Err(self.fail(img, "a computed jump")),
                Flow::Ret => {
                    self.frames.pop();
                    self.jumped = true;
                    if until == Until::Write && self.frames.len() == 1 {
                        return Ok(Event::Returned);
                    }
                }
            }
            if let (Some((reg, v)), Until::Read(want)) = (read, until)
                && reg & 0x1fe == want & 0x1fe
            {
                return Ok(Event::Read(reg, v));
            }
        }
    }

    /// The call at the walk's next instruction made, its callee not walked: the time from the
    /// callee's entry to its return is the caller's to spend before the next [`Walk::run`], which
    /// goes on after the call.
    pub fn skip_call(&mut self, bus: &mut dyn Bus, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let f = self
            .frames
            .last()
            .ok_or_else(|| self.fail(img, "past the end"))?;
        let s = *shape::find(f.shapes, f.pc)
            .ok_or_else(|| self.fail(img, "no translated instruction here"))?;
        if !matches!(s.flow, Flow::Call { .. }) {
            return Err(self.fail(img, "not a call"));
        }
        if self.jumped {
            block_start(bus, r, img)?;
            if !s.first {
                insn_start(bus, r, img)?;
            }
        } else if s.first {
            block_start(bus, r, img)?;
        } else {
            insn_start(bus, r, img)?;
        }
        (s.tick)(r);
        if s.mem {
            bus.sync(r);
        }
        self.top().pc = s.next();
        self.jumped = true;
        Ok(())
    }

    fn top(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("a frame")
    }

    fn call(&mut self, hunk: u32, to: u32, ret: u32, img: &Image) -> Result<(), Stop> {
        self.top().pc = ret;
        let shapes = (self.shapes)((hunk, to))
            .ok_or_else(|| self.fail(img, &format!("a call of h{hunk}+${to:x}, no function")))?;
        self.frames.push(Frame {
            hunk,
            shapes,
            pc: to,
        });
        self.jumped = true;
        Ok(())
    }
}
