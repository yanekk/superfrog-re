//! The 68000's instruction times, for the cycle counter translated code keeps (translator DESIGN
//! §2.6, §2.7): the beam position, and so every raster wait, comes from this count.
//!
//! The fixed part (T11): an instruction's time is its base time ([`base_cycles`]) plus the
//! effective-address calculation times of the operands it reads or writes ([`ea_cycles`]), as the
//! MC68000 User's Manual tables give them (section 8) and Moira counts them with no bus contention.
//! Generated code calls [`of`] inside `const { }`, so each instruction's time is a constant and a
//! kind this table does not hold is a compile error, not a silent zero. A conditional branch's two
//! times are [`bcc`].
//!
//! The data-dependent part (T12): multiply by the source's bits ([`mulu_cycles`], [`muls_cycles`]),
//! divide by its steps ([`divu_cycles`], [`divs_cycles`]), a shift by a register's count
//! ([`shift_cycles`]), `dbcc` by its outcome ([`dbcc`]), `scc` on a register ([`scc_reg`]) and a bit
//! op numbered by a register ([`of`] at run time). Generated code computes these from the registers
//! before the instruction runs, so the tick still comes first. `movem` ([`movem_cycles`]) depends
//! only on its register mask, the instruction's own word, so it is a constant like the rest.
//! These follow Moira's rules (`MoiraALU_cpp.h` `cyclesMul`, `cyclesDiv`, `cyclesBit`), the
//! reference the instruction check judges against.
//!
//! Every function here is a `const fn`, so the generated code's `const { }` evaluates the fixed ones
//! at build time.

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
    /// A shift or rotate: of a data register by a count (an immediate's 1 to 8, or a register's 0 to
    /// 63 at run time, [`shift_cycles`]), or of a word in memory by one (count 1).
    Shift(u32),
    Btst,
    /// `bset` and `bchg`, with the bit number (only a data register's time depends on it): an
    /// immediate's, or by a register the register's at run time.
    Bset(u32),
    Bclr(u32),
    /// `scc` on memory; on a data register its time is the condition's ([`scc_reg`]).
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
        // A bit number above 15 costs 2 more on a register (Moira's cyclesBit); by a register, the
        // number is the register's at run time.
        Op::Bset(bit) => match (src, dst) {
            (Ea::Imm, Ea::Dn) => 10 + if bit % 32 >= 16 { 2 } else { 0 },
            (Ea::Imm, _) => 12,
            (Ea::Dn, Ea::Dn) => 6 + if bit % 32 >= 16 { 2 } else { 0 },
            (Ea::Dn, _) => 8,
            _ => panic!("bset: no such mode"),
        },
        Op::Bclr(bit) => match (src, dst) {
            (Ea::Imm, Ea::Dn) => 12 + if bit % 32 >= 16 { 2 } else { 0 },
            (Ea::Imm, _) => 12,
            (Ea::Dn, Ea::Dn) => 8 + if bit % 32 >= 16 { 2 } else { 0 },
            (Ea::Dn, _) => 8,
            _ => panic!("bclr: no such mode"),
        },
        Op::Scc => match src {
            Ea::Dn => panic!("scc Dn: its time is the condition's, scc_reg"),
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

/// `mulu.w`'s time without its source operand's address time: 38 plus 2 for each 1 bit of the
/// source (Moira's cyclesMul, 2 x (17 + ones), and the prefetch's 4).
pub const fn mulu_cycles(src: u16) -> u32 {
    38 + 2 * src.count_ones()
}

/// `muls.w`'s time without its source operand's address time: 38 plus 2 for each change between
/// adjacent bits of the source with a 0 appended below it (Moira's cyclesMul).
pub const fn muls_cycles(src: u16) -> u32 {
    38 + 2 * ((src << 1) ^ src).count_ones()
}

/// `divu.w`'s time without its divisor operand's address time, by Moira's cyclesDiv: 10 when the
/// quotient overflows a word, else 76 plus 1 or 2 cycles' pairs a step of its 15-step loop. A zero
/// divisor takes the divide-by-zero exception: 38 to the handler's first instruction, the manual's
/// total, which the instruction check cannot judge (Moira's step reports the exception, no time).
pub const fn divu_cycles(dividend: u32, divisor: u16) -> u32 {
    if divisor == 0 {
        return DIV_ZERO;
    }
    if (dividend >> 16) >= divisor as u32 {
        return 10;
    }
    let mut dividend = dividend;
    let hdivisor = (divisor as u32) << 16;
    let mut m = 38;
    let mut i = 0;
    while i < 15 {
        if (dividend as i32) < 0 {
            dividend = (dividend << 1).wrapping_sub(hdivisor);
        } else {
            dividend <<= 1;
            if dividend >= hdivisor {
                dividend = dividend.wrapping_sub(hdivisor);
                m += 1;
            } else {
                m += 2;
            }
        }
        i += 1;
    }
    // cyclesDiv's 2 x m, less the 4 Moira syncs early, plus the prefetch's 4.
    2 * m
}

/// `divs.w`'s time without its divisor operand's address time, by Moira's cyclesDiv: by the signs
/// and by the absolute quotient's bits; 16 or 18 when the quotient overflows. A zero divisor is
/// [`divu_cycles`]'s.
pub const fn divs_cycles(dividend: i32, divisor: i16) -> u32 {
    if divisor == 0 {
        return DIV_ZERO;
    }
    let mut m: i32 = if dividend < 0 { 7 } else { 6 };
    let (ad, av) = (dividend.unsigned_abs(), divisor.unsigned_abs() as u32);
    if (ad >> 16) >= av {
        return 2 * (m as u32 + 2);
    }
    m += 55;
    if divisor >= 0 {
        m += if dividend < 0 { 1 } else { -1 };
    }
    let mut q = ad / av;
    let mut i = 0;
    while i < 15 {
        if (q as i16) >= 0 {
            m += 1;
        }
        q <<= 1;
        i += 1;
    }
    2 * m as u32
}

/// A divide by zero's time, to the handler's first instruction (MC68000UM table 8-11, plus the
/// divisor's address time). Moira's step stops at the exception without reporting a time, so this is
/// the manual's, unjudged.
pub const DIV_ZERO: u32 = 38;

/// `trap #n`'s time to the handler's first instruction (MC68000UM table 8-11), unjudged as
/// [`DIV_ZERO`]: the translated code stops at a trap and the machine takes the vector.
pub const TRAP: u32 = 34;

/// `chk`'s time without its bound's address time: 10 when in bounds, 40 to the handler's first
/// instruction when out (MC68000UM table 8-11; the out-of-bounds time unjudged as [`DIV_ZERO`]).
pub const fn chk_cycles(trapped: bool) -> u32 {
    if trapped { 40 } else { 10 }
}

/// A shift or rotate of a data register by the count in another, 0 to 63 (the register modulo
/// 64): 6 (byte, word) or 8 (long) plus 2 a step.
pub const fn shift_cycles(size: Size, count: u32) -> u32 {
    of(Op::Shift(count & 63), size, Ea::Dn, Ea::Dn)
}

/// `movem`'s direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// Registers to memory.
    ToMem,
    /// Memory to registers.
    ToRegs,
}

/// `movem`'s time: its base by mode (MC68000UM table 8-14) and 4 a register a word, 8 a long. `ea`
/// is the memory operand's mode; `-(An)` only to memory, `(An)+` and the PC modes only from it.
pub const fn movem_cycles(dir: Dir, ea: Ea, size: Size, nregs: u32) -> u32 {
    let base = match ea {
        Ea::Ind | Ea::PostInc | Ea::PreDec => 8,
        Ea::D16An | Ea::AbsW | Ea::D16Pc => 12,
        Ea::D8AnXn | Ea::D8PcXn => 14,
        Ea::AbsL => 16,
        _ => panic!("movem: no such mode"),
    };
    // Memory to registers reads one word more than it moves (the 68000's extra read).
    let extra = match dir {
        Dir::ToRegs => 4,
        Dir::ToMem => 0,
    };
    base + extra + nregs * bw_l(size, 4, 8)
}

/// `dbcc`'s time: 12 when the condition holds (no decrement); else 10 when the loop goes on and
/// 14 when the counter, before the decrement, is 0 and so expires.
pub const fn dbcc(holds: bool, counter: u16) -> u32 {
    if holds {
        12
    } else if counter == 0 {
        14
    } else {
        10
    }
}

/// `scc` on a data register: 6 when the condition holds (the byte set), 4 when not.
pub const fn scc_reg(holds: bool) -> u32 {
    if holds { 6 } else { 4 }
}

/// Whether condition `cc` (the opcode's 4-bit field: 0 `t`, 1 `f`, 2 `hi` .. 15 `le`) holds on
/// the flags in `r`: what `dbcc` and `scc` time by, read before the instruction runs.
pub fn cc_holds(cc: u32, r: &crate::Regs) -> bool {
    let f = |off: usize| r.get(off, 1) != 0;
    let (n, z, v, c) = (f(crate::NF), f(crate::ZF), f(crate::VF), f(crate::CF));
    match cc & 15 {
        0 => true,
        1 => false,
        2 => !c && !z,
        3 => c || z,
        4 => !c,
        5 => c,
        6 => !z,
        7 => z,
        8 => !v,
        9 => v,
        10 => !n,
        11 => n,
        12 => n == v,
        13 => n != v,
        14 => !z && n == v,
        _ => z || n != v,
    }
}

// The cycles off the bus (translator T49). An instruction's time is its bus cycles, four each (the
// manual's `n(r/w)`: reads and writes, the prefetch's included), and internal cycles the bus is free
// in; a machine that models DMA contention needs to know which. Every row below is the manual's
// total less four times its reads and writes (MC68000UM section 8); unjudged against the oracle
// instruction by instruction, held as a whole by `pcode-machine`'s contention readings.

/// The cycles `mode`'s address calculation spends off the bus: `-(An)` and the index modes' 2.
pub const fn ea_off_bus(mode: Ea) -> u32 {
    match mode {
        Ea::PreDec | Ea::D8AnXn | Ea::D8PcXn => 2,
        _ => 0,
    }
}

/// The instruction's own cycles off the bus, without its operands' ([`off_bus`] adds them).
pub const fn base_off_bus(k: Kind) -> u32 {
    let Kind { op, size, src, dst } = k;
    let reg_or_imm = is_reg(src) || matches!(src, Ea::Imm);
    match op {
        Op::Move | Op::Movea | Op::Moveq | Op::Tst | Op::AluToMem | Op::Cmpm => 0,
        // `add.l <ea>,Dn` 6(1/0)+, from a register or an immediate 8(1/0).
        Op::AluToDn => bw_l(size, 0, if reg_or_imm { 4 } else { 2 }),
        Op::Cmp => bw_l(size, 0, 2),
        // `adda.w` 8(1/0)+, `adda.l` 6(1/0)+ or 8(1/0).
        Op::Adda => base_cycles(k) - 4,
        Op::Cmpa => 2,
        Op::Imm => match dst {
            Ea::Dn => bw_l(size, 0, 4),
            _ => 0,
        },
        Op::Cmpi => match dst {
            Ea::Dn => bw_l(size, 0, 2),
            _ => 0,
        },
        Op::Quick => match dst {
            Ea::Dn => bw_l(size, 0, 4),
            Ea::An => 4,
            _ => 0,
        },
        Op::Clr => match src {
            Ea::Dn => bw_l(size, 0, 2),
            _ => 0,
        },
        Op::Addx => match src {
            Ea::Dn => bw_l(size, 0, 4),
            _ => 2,
        },
        Op::Bcd => 2,
        Op::Nbcd => match src {
            Ea::Dn => 2,
            _ => 0,
        },
        Op::Ext | Op::Swap | Op::Nop => 0,
        Op::Exg => 2,
        Op::Lea | Op::Pea => match src {
            Ea::D8AnXn | Ea::D8PcXn => 4,
            _ => 0,
        },
        Op::Jsr => match src {
            Ea::D16An | Ea::AbsW | Ea::D16Pc => 2,
            Ea::D8AnXn | Ea::D8PcXn => 6,
            _ => 0,
        },
        Op::Jmp => match src {
            Ea::D16An | Ea::AbsW | Ea::D16Pc | Ea::D8AnXn | Ea::D8PcXn => 2,
            _ => 0,
        },
        Op::Bra | Op::Bsr => 2,
        Op::Rts | Op::Rte | Op::Rtr | Op::Link | Op::Unlk => 0,
        // A register shift reads one word: the rest is the shifter's.
        Op::Shift(_) => match dst {
            Ea::Dn => base_cycles(k) - 4,
            _ => 0,
        },
        Op::Btst => match dst {
            Ea::Dn => 2,
            _ => 0,
        },
        // On a register: two words read with an immediate bit number, one with a register's.
        Op::Bset(_) | Op::Bclr(_) => match (src, dst) {
            (Ea::Imm, Ea::Dn) => base_cycles(k) - 8,
            (_, Ea::Dn) => base_cycles(k) - 4,
            _ => 0,
        },
        Op::Scc => 0,
        Op::Tas => match src {
            Ea::Dn => 0,
            _ => 2,
        },
    }
}

/// The operands' address calculations off the bus, as [`operand_cycles`] counts their times.
const fn operand_off_bus(k: Kind) -> u32 {
    let Kind { op, src, dst, .. } = k;
    match op {
        // A move's `-(An)` destination is written as `(An)` is.
        Op::Move => {
            ea_off_bus(src)
                + if matches!(dst, Ea::PreDec) {
                    0
                } else {
                    ea_off_bus(dst)
                }
        }
        Op::Movea | Op::AluToDn | Op::Cmp | Op::Adda | Op::Cmpa | Op::Tst => ea_off_bus(src),
        Op::AluToMem | Op::Imm | Op::Cmpi | Op::Quick => ea_off_bus(dst),
        Op::Clr | Op::Nbcd | Op::Scc | Op::Tas => ea_off_bus(src),
        Op::Shift(_) if matches!(dst, Ea::None) => ea_off_bus(src),
        Op::Btst | Op::Bset(_) | Op::Bclr(_) => ea_off_bus(dst),
        _ => 0,
    }
}

/// Of a fixed-time instruction's [`cycles`], those off the bus.
pub const fn off_bus(k: Kind) -> u32 {
    base_off_bus(k) + operand_off_bus(k)
}

/// [`off_bus`] of the kind these four make, as [`of`] is [`cycles`]'.
pub const fn off_of(op: Op, size: Size, src: Ea, dst: Ea) -> u32 {
    off_bus(Kind { op, size, src, dst })
}

/// A conditional branch's cycles off the bus: 2 taken (10, two words read); 4 not taken (8 with
/// one word, 12 with two). All before its reads ([`lead`]).
pub const fn bcc_off(taken: bool) -> u32 {
    if taken { 2 } else { 4 }
}

/// `dbcc`'s cycles off the bus: 4 when the condition holds (12, two words), else 2. All before its
/// reads ([`lead`]).
pub const fn dbcc_off(holds: bool) -> u32 {
    if holds { 4 } else { 2 }
}

/// Of an instruction's cycles off the bus, those spent before its first bus access (Moira's
/// `SYNC` ahead of the reads, translator T49): a branch, jump or call idles first, and so does the
/// address calculation of `-(An)` and the index modes when it is the instruction's first operand.
/// The rest of [`off_bus`] follows the accesses. Where the bus is free decides which slots the CPU
/// asks for, which matters beside the bitplanes' even slots.
pub const fn lead(k: Kind) -> u32 {
    let Kind { op, src, dst, .. } = k;
    match op {
        Op::Bra | Op::Bsr | Op::Jmp | Op::Jsr => off_bus(k),
        Op::Move | Op::Movea | Op::AluToDn | Op::Cmp | Op::Adda | Op::Cmpa | Op::Tst | Op::Clr
        | Op::Nbcd | Op::Scc | Op::Tas | Op::Lea | Op::Pea => ea_off_bus(src),
        Op::Shift(_) if matches!(dst, Ea::None) => ea_off_bus(src),
        Op::AluToMem | Op::Quick => ea_off_bus(dst),
        Op::Btst | Op::Bset(_) | Op::Bclr(_) if !matches!(src, Ea::Imm) => ea_off_bus(dst),
        _ => 0,
    }
}

/// [`lead`] of the kind these four make, as [`off_of`] is [`off_bus`]'.
pub const fn lead_of(op: Op, size: Size, src: Ea, dst: Ea) -> u32 {
    lead(Kind { op, size, src, dst })
}

/// A data-dependent instruction of one bus word plus its operand (`mulu`, `muls`, `divu`, `divs`,
/// a shift or bit op by a register, `scc` on a register): its time `cycles` (without the operand's
/// address time) less the one word read.
pub const fn one_word_off(cycles: u32) -> u32 {
    cycles.saturating_sub(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ea::*;
    use Size::*;

    // The examples are the MC68000 User's Manual's (section 8) tables, read row by row.

    /// Moira's order (MoiraExec_cpp.h, MoiraDataflow_cpp.h): a branch, jump or call SYNCs before
    /// its prefetches or push; `-(An)` and the index modes before their read (T49).
    #[test]
    fn what_idles_before_its_accesses() {
        let l = |op, size, src, dst| lead_of(op, size, src, dst);
        assert_eq!(l(Op::Bra, W, None, None), 2);
        assert_eq!(l(Op::Bsr, W, None, None), 2);
        assert_eq!(l(Op::Jsr, L, D16An, None), 2);
        assert_eq!(l(Op::Jsr, L, D8AnXn, None), 6);
        assert_eq!(l(Op::Jmp, L, Ind, None), 0);
        assert_eq!(l(Op::Move, W, PreDec, Dn), 2);
        assert_eq!(l(Op::Move, W, Dn, PreDec), 0);
        assert_eq!(l(Op::AluToMem, W, Dn, PreDec), 2);
        // `lea (d8,An,Xn)` idles 2 before its prefetch and 2 after (off the bus 4).
        assert_eq!((l(Op::Lea, L, D8AnXn, An), off_of(Op::Lea, L, D8AnXn, An)), (2, 4));
        // After their accesses: a register add, an immediate to a register, a shift.
        assert_eq!(l(Op::Adda, L, Dn, An), 0);
        assert_eq!(l(Op::Imm, L, Imm, Dn), 0);
        assert_eq!(l(Op::Shift(8), L, Imm, Dn), 0);
        assert_eq!((bcc_off(true), bcc_off(false), dbcc_off(true), dbcc_off(false)), (2, 4, 4, 2));
    }

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

    // The data-dependent helpers (T12): Moira's rules, also judged on Moira in fd-conform's
    // insn_cycles_dynamic.rs and insn_cycles.rs.

    #[test]
    fn multiply_by_the_sources_bits() {
        assert_eq!(mulu_cycles(0), 38);
        assert_eq!(mulu_cycles(0xffff), 70);
        assert_eq!(mulu_cycles(0x5555), 54);
        assert_eq!(muls_cycles(0), 38);
        assert_eq!(muls_cycles(0xffff), 40, "-1: one change, at the appended 0");
        assert_eq!(muls_cycles(0xaaaa), 68);
        assert_eq!(muls_cycles(0x5555), 70);
    }

    #[test]
    fn divide_by_steps_overflow_and_zero() {
        assert_eq!(divu_cycles(0x0001_0000, 1), 10, "a quotient over a word");
        assert_eq!(divu_cycles(0xffff_ffff, 0xffff), 10);
        assert_eq!(divu_cycles(0, 1), 136, "every step 2: the longest");
        assert_eq!(divu_cycles(7, 0), DIV_ZERO);
        assert_eq!(
            divs_cycles(0x0001_0000, 1),
            16,
            "overflow, dividend positive"
        );
        assert_eq!(
            divs_cycles(-0x0001_0000, 1),
            18,
            "overflow, dividend negative"
        );
        assert_eq!(divs_cycles(-7, 0), DIV_ZERO);
        // Moira: 6 + 55 - 1, plus 1 for each of 15 quotient bits clear after shifting; quotient 0.
        assert_eq!(divs_cycles(0, 1), 2 * (60 + 15));
        assert!(
            divs_cycles(100, -3) > divs_cycles(100, 3),
            "a negative divisor skips the -1"
        );
    }

    #[test]
    fn a_register_shift_counts_its_count_modulo_64() {
        assert_eq!(shift_cycles(W, 0), 6);
        assert_eq!(shift_cycles(L, 0), 8);
        assert_eq!(shift_cycles(B, 63), 6 + 126);
        assert_eq!(shift_cycles(L, 64), 8, "64 is 0");
        assert_eq!(shift_cycles(L, 65), 10);
    }

    #[test]
    fn bit_ops_by_register_on_a_register() {
        assert_eq!(of(Op::Bset(15), L, Dn, Dn), 6);
        assert_eq!(of(Op::Bset(16), L, Dn, Dn), 8);
        assert_eq!(of(Op::Bclr(15), L, Dn, Dn), 8);
        assert_eq!(of(Op::Bclr(16), L, Dn, Dn), 10);
        assert_eq!(scc_reg(true), 6);
        assert_eq!(scc_reg(false), 4);
    }

    #[test]
    fn dbcc_three_outcomes_and_movem_by_count() {
        assert_eq!(dbcc(true, 0), 12);
        assert_eq!(dbcc(false, 5), 10);
        assert_eq!(dbcc(false, 0), 14);
        assert_eq!(movem_cycles(Dir::ToMem, PreDec, L, 1), 16);
        assert_eq!(movem_cycles(Dir::ToMem, Ind, L, 8), 72);
        assert_eq!(movem_cycles(Dir::ToRegs, PostInc, L, 8), 76);
        assert_eq!(movem_cycles(Dir::ToRegs, PostInc, W, 16), 76);
        assert_eq!(movem_cycles(Dir::ToRegs, AbsL, L, 0), 20);
        assert_eq!(movem_cycles(Dir::ToMem, D8AnXn, W, 2), 22);
    }

    #[test]
    fn conditions_read_the_flags() {
        let mut r = crate::Regs::new();
        assert!(cc_holds(0, &r) && !cc_holds(1, &r));
        assert!(cc_holds(6, &r) && !cc_holds(7, &r), "ne with Z clear");
        r.set(crate::ZF, 1, 1);
        assert!(
            cc_holds(7, &r) && cc_holds(15, &r) && !cc_holds(14, &r),
            "eq, le, not gt"
        );
        r.set(crate::ZF, 1, 0);
        r.set(crate::NF, 1, 1);
        assert!(cc_holds(13, &r) && !cc_holds(12, &r), "lt with N and not V");
        r.set(crate::VF, 1, 1);
        assert!(cc_holds(12, &r) && cc_holds(14, &r), "ge, gt");
        r.set(crate::CF, 1, 1);
        assert!(
            cc_holds(3, &r) && cc_holds(5, &r) && !cc_holds(2, &r),
            "ls, cs, not hi"
        );
    }

    #[test]
    fn const_evaluation_works_as_generated_code_uses_it() {
        const N: u32 = of(Op::Move, L, Dn, AbsL);
        assert_eq!(N, 20);
        assert_eq!(const { bcc(false, W) }, 12);
    }
    #[test]
    fn off_bus_is_the_manuals_total_less_four_a_read_or_write() {
        // (kind, total, reads + writes), MC68000UM tables 8-2 to 8-10.
        let rows = [
            (of(Op::Move, W, Ind, Ind), Kind { op: Op::Move, size: W, src: Ind, dst: Ind }, 3),
            (0, Kind { op: Op::AluToDn, size: L, src: Dn, dst: Dn }, 1),
            (0, Kind { op: Op::AluToDn, size: L, src: Ind, dst: Dn }, 3),
            (0, Kind { op: Op::Adda, size: W, src: Dn, dst: An }, 1),
            (0, Kind { op: Op::Quick, size: L, src: Imm, dst: Dn }, 1),
            (0, Kind { op: Op::Quick, size: W, src: Imm, dst: An }, 1),
            (0, Kind { op: Op::Imm, size: L, src: Imm, dst: Dn }, 3),
            (0, Kind { op: Op::Lea, size: L, src: D8AnXn, dst: An }, 2),
            (0, Kind { op: Op::Jsr, size: L, src: D8AnXn, dst: None }, 4),
            (0, Kind { op: Op::Shift(8), size: L, src: Imm, dst: Dn }, 1),
            (0, Kind { op: Op::Shift(4), size: W, src: Imm, dst: Dn }, 1),
            (0, Kind { op: Op::Bclr(3), size: L, src: Imm, dst: Dn }, 2),
            (0, Kind { op: Op::Move, size: W, src: D8AnXn, dst: PreDec }, 4),
            (0, Kind { op: Op::Clr, size: L, src: Dn, dst: None }, 1),
            (0, Kind { op: Op::Rts, size: L, src: None, dst: None }, 4),
            (0, Kind { op: Op::Bsr, size: W, src: None, dst: None }, 4),
        ];
        for (_, k, bus) in rows {
            assert_eq!(off_bus(k), cycles(k) - 4 * bus, "{k:?}");
        }
        assert_eq!(cycles(rows[0].1), rows[0].0);
        assert_eq!(bcc(true, B) - bcc_off(true), 8);
        assert_eq!(bcc(false, B) - bcc_off(false), 4);
        assert_eq!(bcc(false, W) - bcc_off(false), 8);
        assert_eq!(dbcc(true, 3) - dbcc_off(true), 8);
        assert_eq!(dbcc(false, 0) - dbcc_off(false), 12);
        assert_eq!(dbcc(false, 3) - dbcc_off(false), 8);
        assert_eq!(one_word_off(mulu_cycles(0xFFFF)), 70 - 4);
    }
}
