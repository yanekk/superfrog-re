//! The 68000's instruction times, for the cycle counter translated code keeps (translator DESIGN
//! §2.6, §2.7): the beam position, and so every raster wait, comes from this count.
//!
//! The fixed part (T11): an instruction's time is its base time ([`base_cycles`]) plus the
//! effective-address calculation times of the operands it reads or writes ([`ea_cycles`]), as the
//! MC68000 User's Manual tables give them (section 8) and Moira counts them with no bus contention.
//! Generated code calls [`of`] inside `const { }`, so each instruction's time is a constant and a
//! kind this table does not hold is a compile error, not a silent zero. A conditional branch's two
//! times are [`bcc`]. The data-dependent kinds (multiply, divide, `dbcc`, `movem`, shifts by a
//! register, bit ops numbered by a register, `scc` on a register) are T12's.
//!
//! Every `const fn` here, so the generated code's `const { }` evaluates it at build time.

/// An operand's size. An instruction with no size suffix (`lea`, `moveq`, `exg`) uses `L`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    B,
    W,
    L,
}

/// An addressing mode, as the census names them; `None` where the instruction has no such operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ea {
    None,
    Dn,
    An,
    /// `(An)`
    Ind,
    /// `(An)+`
    PostInc,
    /// `-(An)`
    PreDec,
    /// `(d16,An)`
    D16An,
    /// `(d8,An,Xn)`
    D8AnXn,
    AbsW,
    AbsL,
    /// `(d16,PC)`
    D16Pc,
    /// `(d8,PC,Xn)`
    D8PcXn,
    /// `#imm`
    Imm,
}

/// A timing class: mnemonics that share one row of the manual's tables. A bit number or shift
/// count the time depends on rides with its op; it is the instruction's immediate, so still fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Move,
    Movea,
    Moveq,
    /// `add`, `sub`, `and`, `or` with a data register destination, and `eor Dn,Dn`.
    AluToDn,
    /// `add`, `sub`, `and`, `or`, `eor` from a data register into memory.
    AluToMem,
    Cmp,
    /// `adda`, `suba`.
    Adda,
    Cmpa,
    Cmpm,
    /// `addi`, `subi`, `andi`, `ori`, `eori`. Moira counts `andi.l #,Dn` 16 like the rest, where the
    /// manual's table 8-9 gives 14 (translator T11, measured on the census).
    Imm,
    Cmpi,
    /// `addq`, `subq`.
    Quick,
    /// `clr`, `neg`, `negx`, `not`.
    Clr,
    Tst,
    /// `addx`, `subx`.
    Addx,
    /// `abcd`, `sbcd`.
    Bcd,
    Nbcd,
    Ext,
    Swap,
    Exg,
    Nop,
    Lea,
    Pea,
    Jsr,
    Jmp,
    Bra,
    Bsr,
    Rts,
    Rte,
    Rtr,
    Link,
    Unlk,
    /// A shift or rotate: of a data register by an immediate count (1 to 8), or of a word in memory
    /// by one (count 1).
    Shift(u32),
    Btst,
    /// `bset` and `bchg`, with the bit number of an immediate form (only a data register's time
    /// depends on it).
    Bset(u32),
    Bclr(u32),
    /// `scc` on memory; on a data register its time is the condition's (T12).
    Scc,
    Tas,
}

/// One instruction kind: its timing class, size and the census's two operand modes (a lone
/// operand is `src`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind {
    pub op: Op,
    pub size: Size,
    pub src: Ea,
    pub dst: Ea,
}

/// The effective-address calculation time of `mode` at `size` (MC68000UM table 8-1): the extension
/// words and the operand's read, 8 cycles more for a long's second word.
pub const fn ea_cycles(mode: Ea, size: Size) -> u32 {
    let word = match mode {
        Ea::None | Ea::Dn | Ea::An => return 0,
        Ea::Ind | Ea::PostInc | Ea::Imm => 4,
        Ea::PreDec => 6,
        Ea::D16An | Ea::AbsW | Ea::D16Pc => 8,
        Ea::D8AnXn | Ea::D8PcXn => 10,
        Ea::AbsL => 12,
    };
    match size {
        Size::L => word + 4,
        _ => word,
    }
}

const fn is_reg(m: Ea) -> bool {
    matches!(m, Ea::Dn | Ea::An)
}

const fn long(s: Size) -> bool {
    matches!(s, Size::L)
}

/// `a` for a byte or word, `b` for a long.
const fn bw_l(s: Size, a: u32, b: u32) -> u32 {
    if long(s) { b } else { a }
}

/// The instruction's own time, without the effective-address times [`cycles`] adds. Panics (at
/// build time, under `const { }`) on a kind the 68000 has no fixed time for here.
pub const fn base_cycles(k: Kind) -> u32 {
    let Kind { op, size, src, dst } = k;
    match op {
        Op::Move | Op::Movea | Op::Moveq | Op::Tst => 4,
        // The long's 2 extra cycles on a register or immediate source are the ALU's idle bus time.
        Op::AluToDn => bw_l(
            size,
            4,
            if is_reg(src) || matches!(src, Ea::Imm) {
                8
            } else {
                6
            },
        ),
        Op::AluToMem => bw_l(size, 8, 12),
        Op::Cmp => bw_l(size, 4, 6),
        Op::Adda => {
            if long(size) && !(is_reg(src) || matches!(src, Ea::Imm)) {
                6
            } else {
                8
            }
        }
        Op::Cmpa => 6,
        Op::Cmpm => bw_l(size, 12, 20),
        // The immediate's fetch is in these base times, not an `Imm` operand time.
        Op::Imm => match dst {
            Ea::Dn => bw_l(size, 8, 16),
            _ => bw_l(size, 12, 20),
        },
        Op::Cmpi => match dst {
            Ea::Dn => bw_l(size, 8, 14),
            _ => bw_l(size, 8, 12),
        },
        Op::Quick => match dst {
            Ea::Dn => bw_l(size, 4, 8),
            Ea::An => 8,
            _ => bw_l(size, 8, 12),
        },
        Op::Clr => match src {
            Ea::Dn => bw_l(size, 4, 6),
            _ => bw_l(size, 8, 12),
        },
        Op::Addx => match src {
            Ea::Dn => bw_l(size, 4, 8),
            _ => bw_l(size, 18, 30),
        },
        Op::Bcd => match src {
            Ea::Dn => 6,
            _ => 18,
        },
        Op::Nbcd => match src {
            Ea::Dn => 6,
            _ => 8,
        },
        Op::Ext | Op::Swap | Op::Nop => 4,
        Op::Exg => 6,
        Op::Lea => match src {
            Ea::Ind => 4,
            Ea::D16An | Ea::AbsW | Ea::D16Pc => 8,
            Ea::D8AnXn | Ea::D8PcXn | Ea::AbsL => 12,
            _ => panic!("lea: no such mode"),
        },
        Op::Pea => match src {
            Ea::Ind => 12,
            Ea::D16An | Ea::AbsW | Ea::D16Pc => 16,
            Ea::D8AnXn | Ea::D8PcXn | Ea::AbsL => 20,
            _ => panic!("pea: no such mode"),
        },
        Op::Jsr => match src {
            Ea::Ind => 16,
            Ea::D16An | Ea::AbsW | Ea::D16Pc => 18,
            Ea::AbsL => 20,
            Ea::D8AnXn | Ea::D8PcXn => 22,
            _ => panic!("jsr: no such mode"),
        },
        Op::Jmp => match src {
            Ea::Ind => 8,
            Ea::D16An | Ea::AbsW | Ea::D16Pc => 10,
            Ea::AbsL => 12,
            Ea::D8AnXn | Ea::D8PcXn => 14,
            _ => panic!("jmp: no such mode"),
        },
        Op::Bra => 10,
        Op::Bsr => 18,
        Op::Rts => 16,
        Op::Rte | Op::Rtr => 20,
        Op::Link => 16,
        Op::Unlk => 12,
        Op::Shift(n) => match dst {
            Ea::Dn => bw_l(size, 6, 8) + 2 * n,
            Ea::None => 8,
            _ => panic!("shift: a register shift's destination is Dn"),
        },
        Op::Btst => match (src, dst) {
            (Ea::Imm, Ea::Dn) => 10,
            (Ea::Imm, _) => 8,
            (Ea::Dn, Ea::Dn) => 6,
            (Ea::Dn, _) => 4,
            _ => panic!("btst: no such mode"),
        },
        Op::Bset(bit) => match (src, dst) {
            (Ea::Imm, Ea::Dn) => 10 + if bit % 32 >= 16 { 2 } else { 0 },
            (Ea::Imm, _) => 12,
            (Ea::Dn, Ea::Dn) => panic!("bset Dn,Dn: its time is the bit number's (T12)"),
            (Ea::Dn, _) => 8,
            _ => panic!("bset: no such mode"),
        },
        Op::Bclr(bit) => match (src, dst) {
            (Ea::Imm, Ea::Dn) => 12 + if bit % 32 >= 16 { 2 } else { 0 },
            (Ea::Imm, _) => 12,
            (Ea::Dn, Ea::Dn) => panic!("bclr Dn,Dn: its time is the bit number's (T12)"),
            (Ea::Dn, _) => 8,
            _ => panic!("bclr: no such mode"),
        },
        Op::Scc => match src {
            Ea::Dn => panic!("scc Dn: its time is the condition's (T12)"),
            _ => 8,
        },
        Op::Tas => match src {
            Ea::Dn => 4,
            _ => 10,
        },
    }
}

/// The effective-address time [`cycles`] adds to the base: the memory operand's, or for `move` both.
const fn operand_cycles(k: Kind) -> u32 {
    let Kind { op, size, src, dst } = k;
    match op {
        // A move's destination is written, not read: `-(An)` costs what `(An)` does there.
        Op::Move => {
            let d = if matches!(dst, Ea::PreDec) {
                Ea::Ind
            } else {
                dst
            };
            ea_cycles(src, size) + ea_cycles(d, size)
        }
        Op::Movea | Op::AluToDn | Op::Cmp | Op::Adda | Op::Cmpa | Op::Tst => ea_cycles(src, size),
        Op::AluToMem | Op::Imm | Op::Cmpi | Op::Quick => ea_cycles(dst, size),
        Op::Clr | Op::Nbcd | Op::Scc | Op::Tas => ea_cycles(src, size),
        Op::Shift(_) if matches!(dst, Ea::None) => ea_cycles(src, Size::W),
        // Bit ops on memory touch one byte; their immediate bit number is in the base time.
        Op::Btst | Op::Bset(_) | Op::Bclr(_) => ea_cycles(dst, Size::B),
        _ => 0,
    }
}

/// The whole time of a fixed-time instruction: its base and its operands' address times.
pub const fn cycles(k: Kind) -> u32 {
    base_cycles(k) + operand_cycles(k)
}

/// [`cycles`] of the kind these four make: what generated code calls, inside `const { }`.
pub const fn of(op: Op, size: Size, src: Ea, dst: Ea) -> u32 {
    cycles(Kind { op, size, src, dst })
}

/// A conditional branch (`bcc`, not `bra`/`bsr`): 10 cycles taken; not taken, 8 for a byte
/// displacement and 12 for a word one (the word is fetched and skipped).
pub const fn bcc(taken: bool, size: Size) -> u32 {
    match (taken, size) {
        (true, _) => 10,
        (false, Size::B) => 8,
        (false, _) => 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ea::*;
    use Size::*;

    // The examples are the MC68000 User's Manual's (section 8) tables, read row by row.

    #[test]
    fn move_is_four_plus_both_operands() {
        assert_eq!(of(Op::Move, W, Dn, Dn), 4);
        assert_eq!(of(Op::Move, L, Dn, Dn), 4);
        assert_eq!(of(Op::Move, W, Ind, Dn), 8);
        assert_eq!(of(Op::Move, L, Ind, Dn), 12);
        assert_eq!(of(Op::Move, W, Dn, Ind), 8);
        assert_eq!(of(Op::Move, L, Dn, Ind), 12);
        assert_eq!(of(Op::Move, W, Dn, PreDec), 8, "-(An) written costs (An)");
        assert_eq!(of(Op::Move, L, Dn, PreDec), 12);
        assert_eq!(of(Op::Move, W, PreDec, Dn), 10, "-(An) read costs 6");
        assert_eq!(of(Op::Move, B, D8AnXn, Dn), 14);
        assert_eq!(of(Op::Move, W, AbsL, AbsL), 28);
        assert_eq!(of(Op::Move, L, AbsL, AbsL), 36);
        assert_eq!(of(Op::Move, L, Imm, AbsW), 24);
        assert_eq!(of(Op::Move, W, Imm, D16An), 16);
        assert_eq!(of(Op::Movea, L, AbsL, An), 20);
        assert_eq!(of(Op::Moveq, L, Imm, Dn), 4);
    }

    #[test]
    fn add_to_a_register_and_to_memory() {
        assert_eq!(of(Op::AluToDn, W, Dn, Dn), 4);
        assert_eq!(of(Op::AluToDn, L, Dn, Dn), 8);
        assert_eq!(of(Op::AluToDn, L, Imm, Dn), 16);
        assert_eq!(of(Op::AluToDn, L, AbsL, Dn), 22);
        assert_eq!(of(Op::AluToDn, W, D16Pc, Dn), 12);
        assert_eq!(of(Op::AluToMem, W, Dn, Ind), 12);
        assert_eq!(of(Op::AluToMem, L, Dn, AbsL), 28);
        assert_eq!(of(Op::Adda, W, Imm, An), 12);
        assert_eq!(of(Op::Adda, L, Dn, An), 8);
        assert_eq!(of(Op::Adda, L, Imm, An), 16);
        assert_eq!(of(Op::Adda, L, AbsL, An), 22);
        assert_eq!(of(Op::Imm, L, Imm, Dn), 16);
        assert_eq!(of(Op::Imm, W, Imm, AbsL), 24);
        assert_eq!(of(Op::Imm, L, Imm, AbsL), 36);
        assert_eq!(of(Op::Imm, L, Imm, Dn), 16, "andi.l too, on Moira");
        assert_eq!(of(Op::Imm, B, Imm, Dn), 8);
        assert_eq!(of(Op::Quick, W, Imm, An), 8);
        assert_eq!(of(Op::Quick, L, Imm, An), 8);
        assert_eq!(of(Op::Quick, L, Imm, Dn), 8);
        assert_eq!(of(Op::Quick, B, Imm, AbsL), 20);
        assert_eq!(of(Op::Addx, L, Dn, Dn), 8);
        assert_eq!(of(Op::Addx, W, PreDec, PreDec), 18);
        assert_eq!(of(Op::Bcd, B, PreDec, PreDec), 18);
        assert_eq!(of(Op::Bcd, B, Dn, Dn), 6);
    }

    #[test]
    fn compares_tests_and_single_operands() {
        assert_eq!(of(Op::Cmp, L, Dn, Dn), 6);
        assert_eq!(of(Op::Cmp, W, D8AnXn, Dn), 14);
        assert_eq!(of(Op::Cmpa, L, Imm, An), 14);
        assert_eq!(of(Op::Cmpa, L, An, An), 6);
        assert_eq!(of(Op::Cmpi, L, Imm, Dn), 14);
        assert_eq!(of(Op::Cmpi, B, Imm, Ind), 12);
        assert_eq!(of(Op::Cmpi, L, Imm, AbsL), 28);
        assert_eq!(of(Op::Cmpm, B, PostInc, PostInc), 12);
        assert_eq!(of(Op::Cmpm, L, PostInc, PostInc), 20);
        assert_eq!(of(Op::Tst, L, Ind, None), 12);
        assert_eq!(of(Op::Tst, B, Dn, None), 4);
        assert_eq!(of(Op::Clr, L, Dn, None), 6);
        assert_eq!(of(Op::Clr, W, AbsL, None), 20);
        assert_eq!(of(Op::Clr, L, PostInc, None), 20);
        assert_eq!(of(Op::Nbcd, B, D16An, None), 16);
        assert_eq!(of(Op::Swap, L, Dn, None), 4);
        assert_eq!(of(Op::Exg, L, Dn, Dn), 6);
    }

    #[test]
    fn lea_jsr_and_returns() {
        assert_eq!(of(Op::Lea, L, Ind, An), 4);
        assert_eq!(of(Op::Lea, L, D16Pc, An), 8);
        assert_eq!(of(Op::Lea, L, D8AnXn, An), 12);
        assert_eq!(of(Op::Lea, L, AbsL, An), 12);
        assert_eq!(of(Op::Pea, L, AbsL, None), 20);
        assert_eq!(of(Op::Jsr, L, D16An, None), 18);
        assert_eq!(of(Op::Jsr, L, AbsL, None), 20);
        assert_eq!(of(Op::Jmp, L, Ind, None), 8);
        assert_eq!(of(Op::Bsr, W, None, None), 18);
        assert_eq!(of(Op::Bra, B, None, None), 10);
        assert_eq!(of(Op::Rts, L, None, None), 16);
        assert_eq!(of(Op::Rte, L, None, None), 20);
    }

    #[test]
    fn bcc_taken_and_not() {
        assert_eq!(bcc(true, B), 10);
        assert_eq!(bcc(true, W), 10);
        assert_eq!(bcc(false, B), 8);
        assert_eq!(bcc(false, W), 12);
    }

    #[test]
    fn shifts_bits_and_scc() {
        assert_eq!(of(Op::Shift(1), W, Imm, Dn), 8);
        assert_eq!(of(Op::Shift(8), B, Imm, Dn), 22);
        assert_eq!(of(Op::Shift(4), L, Imm, Dn), 16);
        assert_eq!(of(Op::Shift(1), W, AbsL, None), 20);
        assert_eq!(of(Op::Btst, L, Imm, Dn), 10);
        assert_eq!(of(Op::Btst, B, Imm, D16An), 16);
        assert_eq!(of(Op::Btst, B, Dn, Ind), 8);
        assert_eq!(of(Op::Bset(3), L, Imm, Dn), 10);
        assert_eq!(of(Op::Bset(31), L, Imm, Dn), 12);
        assert_eq!(
            of(Op::Bset(35), L, Imm, Dn),
            10,
            "the bit number is modulo 32"
        );
        // Moira's cyclesBit: 2 more for a bit above 15 (MoiraALU_cpp.h), so 15 and 16 are the edge.
        assert_eq!(of(Op::Bset(15), L, Imm, Dn), 10);
        assert_eq!(of(Op::Bset(16), L, Imm, Dn), 12);
        assert_eq!(of(Op::Bclr(15), L, Imm, Dn), 12);
        assert_eq!(of(Op::Bclr(16), L, Imm, Dn), 14);
        assert_eq!(of(Op::Bset(7), B, Imm, AbsL), 24);
        assert_eq!(of(Op::Bclr(0), L, Imm, Dn), 12);
        assert_eq!(of(Op::Bclr(16), L, Imm, Dn), 14);
        assert_eq!(of(Op::Bclr(1), B, Imm, AbsL), 24);
        assert_eq!(of(Op::Scc, B, Ind, None), 12);
        assert_eq!(of(Op::Scc, B, AbsL, None), 20);
        assert_eq!(of(Op::Tas, B, Dn, None), 4);
        assert_eq!(of(Op::Tas, B, Ind, None), 14);
    }

    #[test]
    fn ea_times_by_mode_and_size() {
        let table = [
            (Dn, 0, 0),
            (An, 0, 0),
            (Ind, 4, 8),
            (PostInc, 4, 8),
            (PreDec, 6, 10),
            (D16An, 8, 12),
            (D8AnXn, 10, 14),
            (AbsW, 8, 12),
            (AbsL, 12, 16),
            (D16Pc, 8, 12),
            (D8PcXn, 10, 14),
            (Imm, 4, 8),
        ];
        for (m, bw, l) in table {
            assert_eq!(ea_cycles(m, B), bw, "{m:?}.b");
            assert_eq!(ea_cycles(m, W), bw, "{m:?}.w");
            assert_eq!(ea_cycles(m, L), l, "{m:?}.l");
        }
        assert_eq!(
            base_cycles(Kind {
                op: Op::Move,
                size: L,
                src: AbsL,
                dst: AbsL
            }),
            4
        );
    }

    #[test]
    fn const_evaluation_works_as_generated_code_uses_it() {
        const N: u32 = of(Op::Move, L, Dn, AbsL);
        assert_eq!(N, 20);
        assert_eq!(const { bcc(false, W) }, 12);
    }
}
