//! An instruction's time as the generated code spends it, apart from what it does: the translator
//! emits one [`Shape`] per instruction of every function (`SHAPES_*` beside the function), so a
//! hand-written routine standing in for a translated one can spend the original's time instruction
//! by instruction along the path it takes (fd-translated's `seam::walk`, translator T56).
//!
//! Each shape makes the very calls the generated instruction makes before its own lines: its
//! `r.tick`, `r.off_bus` and `r.idle_first` ([`Shape::tick`]), `bus.sync` when it reads or writes
//! memory ([`Shape::mem`]), and `block_start` or `insn_start` by where its block starts
//! ([`Shape::first`]). The time a machine fits onto the bus is a function of those calls alone, so a
//! walk that makes them in the same order spends the same time, contention included.

use crate::Regs;

/// Where control goes after the instruction.
#[derive(Clone, Copy)]
pub enum Flow {
    /// To the next instruction.
    Next,
    /// A conditional branch: [`Shape::tick`] is the path not taken, `taken` the other, to `to`.
    Bcc { taken: fn(&mut Regs), to: u32 },
    /// `dbcc` to `to`: [`Shape::tick`] reads the counter, data register `reg`, as it stands before
    /// the instruction.
    Dbcc { to: u32, reg: u8 },
    /// `bra` to `to`.
    Jump { to: u32 },
    /// A jump whose target the instruction computes.
    JumpInd,
    /// `bsr`/`jsr` to the function at `to` in the same hunk.
    Call { to: u32 },
    /// A call whose target the instruction computes.
    CallInd,
    /// `rts`, `rte`.
    Ret,
}

/// One instruction of a translated function.
#[derive(Clone, Copy)]
pub struct Shape {
    /// Hunk and offset.
    pub h: u8,
    pub at: u32,
    pub len: u8,
    /// The first instruction of a generated block: reached in sequence, `block_start` runs before
    /// it in place of `insn_start`.
    pub first: bool,
    /// Its lines read or write memory: `bus.sync` runs after its tick.
    pub mem: bool,
    /// Its time hangs on a data register or the flags (a shift or multiply by a register, `scc` on
    /// one), not `dbcc`'s counter: the walk sets that register before [`Shape::tick`] reads it.
    pub dynamic: bool,
    /// Its time is one the translator does not count (the generated code stops there).
    pub uncounted: bool,
    /// A custom-chip register it reads or writes by absolute address (`$dffXXX.l`): the offset,
    /// the access's size in bytes and whether it writes it.
    pub custom: Option<(u16, u8, bool)>,
    pub tick: fn(&mut Regs),
    pub flow: Flow,
}

impl Shape {
    /// The offset of the instruction after it.
    pub fn next(&self) -> u32 {
        self.at + self.len as u32
    }
}

/// The shape of the instruction at `at` in `shapes` (sorted by offset).
pub fn find(shapes: &[Shape], at: u32) -> Option<&Shape> {
    shapes
        .binary_search_by_key(&at, |s| s.at)
        .ok()
        .map(|i| &shapes[i])
}
