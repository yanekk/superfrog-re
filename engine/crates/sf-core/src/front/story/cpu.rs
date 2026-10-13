//! The 68000's bus cycles, for code whose writes race the copper (T46): when the vertical blank's
//! fade step rewrites a copper list's colour word decides whether the copper reads it old or new
//! this frame, and the step's writes land within a cycle or two of the reads.
//!
//! An instruction is the sequence of its bus accesses and idle cycles as the reference emulator's
//! CPU core (Moira, vAmiga) makes them, written one letter each: `p` a program word (prefetch or
//! extension word), `r` a data read, `w` a data write, `n` one DMA cycle (two CPU clocks) of
//! internal work. An access takes two DMA cycles and uses the bus in the second; when another
//! owner (refresh, the copper, a DMA channel) has that cycle, the CPU waits for the next free one
//! (vAmiga `Agnus::executeUntilBusIsFree`).

/// DMA cycles per line.
pub const LINE: u32 = crate::render::copper::LINE_CYCLES as u32;

/// The bus as the CPU finds it: whether DMA cycle `t` (counted from the frame's line 0, cycle 0)
/// is taken by a DMA owner.
pub trait Bus {
    fn taken(&self, t: u32) -> bool;
}

/// Refresh's four cycles in each line (vAmiga `DAS_REFRESH`: `$01`, `$03`, `$05`, `$E2`).
pub fn refresh(t: u32) -> bool {
    matches!(t % LINE, 0x01 | 0x03 | 0x05 | 0xe2)
}

/// The CPU's clock on the bus.
pub struct Cpu<'a> {
    /// The DMA cycle the CPU has reached (from the frame's line 0, cycle 0).
    pub t: u32,
    bus: &'a dyn Bus,
    /// The cycles of the writes made, in order.
    pub writes: Vec<u32>,
}

impl<'a> Cpu<'a> {
    pub fn new(t: u32, bus: &'a dyn Bus) -> Cpu<'a> {
        Cpu {
            t,
            bus,
            writes: Vec::new(),
        }
    }

    /// One access: its bus cycle.
    fn access(&mut self) -> u32 {
        self.t += 1;
        while self.bus.taken(self.t) {
            self.t += 1;
        }
        let at = self.t;
        self.t += 1;
        at
    }

    /// Run the accesses and idle cycles `ops` (`p`, `r`, `w`, `n`; anything else is ignored).
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
}

/// The instructions the intro's timed code uses, as Moira runs them on a 68000.
pub mod op {
    /// `lea d(pc), An`; `lea abs.l, An`.
    pub const LEA_PC: &str = "pp";
    pub const LEA_ABS_L: &str = "ppp";
    /// `bsr` (byte or word); `jsr abs.l`; `rts`.
    pub const BSR: &str = "nwwpp";
    pub const JSR_ABS_L: &str = "ppwwpp";
    pub const RTS: &str = "rrpp";
    /// `bcc` taken; not taken, byte and word displacement.
    pub const BCC_TAKEN: &str = "npp";
    pub const BCC_B_NOT: &str = "nnp";
    pub const BCC_W_NOT: &str = "nnpp";
    /// `cmpi.w #i, d(An)`; `addi.w #i, d(An)`; `tst.w d(An)`; `clr.w d(An)`.
    pub const CMPI_W_DI: &str = "pprp";
    pub const ADDI_W_DI: &str = "pprpw";
    pub const TST_W_DI: &str = "prp";
    pub const CLR_W_DI: &str = "prpw";
    /// `move.w d(An), Dn`; `cmp.w d(An), Dn`; `movea.l d(An), An`; `move.w #i, d(An)`.
    pub const MOVE_W_DI_DN: &str = "prp";
    pub const CMP_W_DI_DN: &str = "prp";
    pub const MOVEA_L_DI: &str = "prrp";
    pub const MOVE_W_IM_DI: &str = "ppwp";
    /// `movea.l #i, An`; `clr.l Dn`; `move.l Dn, Dn` and other register-only word or long work
    /// of four clocks; `cmp.l Dn, Dn`.
    pub const MOVEA_L_IM: &str = "ppp";
    pub const CLR_L_DN: &str = "pn";
    pub const REG: &str = "p";
    pub const CMP_L_DN: &str = "pn";
    /// `adda.l #i, An`; `addq.l #i, An`; `subq.w #i, Dn`.
    pub const ADDA_L_IM: &str = "pppnn";
    pub const ADDQ_L_AN: &str = "pnn";
    pub const SUBQ_W_DN: &str = "p";
    /// `move.w (An), Dn`; `move.b (An)+, Dn`; `andi.w #i, Dn`; `add.w Dn, (An)`; `sub.w Dn,
    /// (An)`; `andi.w #i, (An)`; `or.w Dn, (An)`.
    pub const MOVE_W_AI_DN: &str = "rp";
    pub const ANDI_W_DN: &str = "pp";
    pub const ADD_W_DN_AI: &str = "rpw";
    /// `add.w Dn, d(An)` and the other word read-modify-writes of `d(An)` from a register
    /// (`sub`, `or`, `subq`, `clr`).
    pub const ADD_W_DN_DI: &str = "prpw";
    pub const ANDI_W_AI: &str = "prpw";
    /// `mulu.w #3, Dn` (34 clocks of work for two set bits).
    pub const MULU_3: &str = "ppnnnnnnnnnnnnnnnnn";

    /// `lsr`, `lsl` (word) by `n`: a prefetch, then `1 + n` DMA cycles.
    pub fn shift(n: u32) -> String {
        let mut s = String::from("p");
        s.extend(std::iter::repeat_n('n', 1 + n as usize));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Free;
    impl Bus for Free {
        fn taken(&self, _: u32) -> bool {
            false
        }
    }
    struct Refresh;
    impl Bus for Refresh {
        fn taken(&self, t: u32) -> bool {
            refresh(t)
        }
    }

    #[test]
    fn an_access_takes_two_cycles_and_waits_for_a_free_one() {
        let mut c = Cpu::new(0, &Free);
        c.run(op::ADD_W_DN_AI);
        assert_eq!((c.t, c.writes.clone()), (6, vec![5]));
        // From cycle 0 the first access falls on refresh's cycle 1 and waits for 2.
        let mut c = Cpu::new(0, &Refresh);
        c.run("r");
        assert_eq!(c.t, 3);
        c.run(&op::shift(4));
        assert_eq!(c.t, 3 + 2 + 5);
    }
}
