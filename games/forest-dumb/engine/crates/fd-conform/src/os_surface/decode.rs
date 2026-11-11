//! Just enough of a 68000 decoder for the OS survey: each instruction's length, its effective
//! addresses with how each is used, and how control leaves it. Lengths are checked against
//! Moira's disassembler by the caller, so a gap here shows as an `undecoded` site, never as a
//! silent miss.

/// An effective address. `Pc` holds the target already resolved; an index register's value is not
/// known statically, so the indexed forms keep only the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ea {
    D(u8),
    A(u8),
    Ind(u8),
    Post(u8),
    Pre(u8),
    Disp(u8, i16),
    Idx(u8),
    AbsW(u32),
    AbsL(u32),
    Pc(u32),
    PcIdx,
    Imm(u32),
}

impl Ea {
    /// The absolute address this names, for the two absolute modes.
    pub fn absolute(self) -> Option<u32> {
        match self {
            Ea::AbsW(a) | Ea::AbsL(a) => Some(a),
            _ => None,
        }
    }

    /// The address register memory is reached through, for the register-relative modes.
    pub fn through(self) -> Option<u8> {
        match self {
            Ea::Ind(n) | Ea::Post(n) | Ea::Pre(n) | Ea::Disp(n, _) | Ea::Idx(n) => Some(n),
            _ => None,
        }
    }
}

/// How an operand is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Access {
    Read,
    Write,
    /// Read, then written (`bset`, `addq`, `or Dn,<ea>`).
    Modify,
    /// Only its address is taken (`lea`, `pea`, a long immediate that is a pointer).
    Address,
    /// The target of `jsr`/`jmp`.
    Control,
}

/// How control leaves the instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Next,
    /// `jsr`/`bsr`: control comes back to the next instruction.
    Call,
    /// `jmp`/`bra`: it does not.
    Jump,
    /// `bcc`, `dbcc`: it may fall through.
    Branch,
    /// `rts`, `rte`, `rtr`, `illegal`, a trap: the straight line ends.
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insn {
    pub len: u32,
    /// Operand size in bytes, 0 where it has none.
    pub size: u8,
    pub ops: Vec<(Ea, Access)>,
    pub flow: Flow,
    /// Registers written that no operand names (`movem` to registers): bit n is dn, bit 8+n an.
    pub clobbers: u16,
}

struct Rd<'a> {
    b: &'a [u8],
    pc: u32,
    pos: usize,
}

impl Rd<'_> {
    fn word(&mut self) -> Option<u16> {
        let w = u16::from_be_bytes([*self.b.get(self.pos)?, *self.b.get(self.pos + 1)?]);
        self.pos += 2;
        Some(w)
    }

    fn long(&mut self) -> Option<u32> {
        Some(((self.word()? as u32) << 16) | self.word()? as u32)
    }

    fn ea(&mut self, mode: u16, reg: u16, size: u8) -> Option<Ea> {
        let r = reg as u8;
        Some(match mode {
            0 => Ea::D(r),
            1 => Ea::A(r),
            2 => Ea::Ind(r),
            3 => Ea::Post(r),
            4 => Ea::Pre(r),
            5 => Ea::Disp(r, self.word()? as i16),
            6 => {
                self.word()?;
                Ea::Idx(r)
            }
            _ => match reg {
                0 => Ea::AbsW(self.word()? as i16 as i32 as u32),
                1 => Ea::AbsL(self.long()?),
                2 => {
                    // PC-relative counts from the extension word itself.
                    let at = self.pc.wrapping_add(self.pos as u32);
                    Ea::Pc(at.wrapping_add(self.word()? as i16 as i32 as u32))
                }
                3 => {
                    self.word()?;
                    Ea::PcIdx
                }
                4 => match size {
                    1 => Ea::Imm((self.word()? & 0xff) as u32),
                    2 => Ea::Imm(self.word()? as u32),
                    4 => Ea::Imm(self.long()?),
                    _ => return None,
                },
                _ => return None,
            },
        })
    }

    /// The effective address in the low six bits of `op`.
    fn low(&mut self, op: u16, size: u8) -> Option<Ea> {
        self.ea((op >> 3) & 7, op & 7, size)
    }
}

/// Sizes in the usual two-bit field: byte, word, long.
fn size2(bits: u16) -> Option<u8> {
    match bits {
        0 => Some(1),
        1 => Some(2),
        2 => Some(4),
        _ => None,
    }
}

/// Decode the instruction at `pc` from `bytes` (its first ten bytes or more). `None` for a word
/// this decoder does not know or bytes that run out.
pub fn decode(bytes: &[u8], pc: u32) -> Option<Insn> {
    let mut r = Rd {
        b: bytes,
        pc,
        pos: 0,
    };
    let op = r.word()?;
    let dreg = ((op >> 9) & 7) as u8;
    let mode = (op >> 3) & 7;
    let mut ops = Vec::new();
    let mut flow = Flow::Next;
    let mut size = 0u8;
    let mut clobbers = 0u16;
    match op >> 12 {
        0x0 => {
            if op & 0x0100 != 0 {
                if mode == 1 {
                    // movep: d16(Ay) and Dn; bit 7 is register to memory.
                    let d = r.word()? as i16;
                    size = if op & 0x40 != 0 { 4 } else { 2 };
                    let mem = Ea::Disp((op & 7) as u8, d);
                    if op & 0x80 != 0 {
                        ops.push((Ea::D(dreg), Access::Read));
                        ops.push((mem, Access::Write));
                    } else {
                        ops.push((mem, Access::Read));
                        ops.push((Ea::D(dreg), Access::Write));
                    }
                } else {
                    // btst/bchg/bclr/bset Dn,<ea>.
                    size = if mode == 0 { 4 } else { 1 };
                    let ea = r.low(op, 1)?;
                    ops.push((Ea::D(dreg), Access::Read));
                    let acc = if (op >> 6) & 3 == 0 {
                        Access::Read
                    } else {
                        Access::Modify
                    };
                    ops.push((ea, acc));
                }
            } else {
                match (op >> 9) & 7 {
                    4 => {
                        // Static bit operations: the bit number's word, then the address.
                        r.word()?;
                        size = if mode == 0 { 4 } else { 1 };
                        let ea = r.low(op, 1)?;
                        let acc = if (op >> 6) & 3 == 0 {
                            Access::Read
                        } else {
                            Access::Modify
                        };
                        ops.push((ea, acc));
                    }
                    k @ (0 | 1 | 2 | 3 | 5 | 6) => {
                        if op & 0x3f == 0x3c && matches!(k, 0 | 1 | 5) {
                            // ori/andi/eori to CCR or SR.
                            r.word()?;
                        } else {
                            size = size2((op >> 6) & 3)?;
                            let imm = r.ea(7, 4, size)?;
                            let ea = r.low(op, size)?;
                            ops.push((imm, Access::Read));
                            let acc = if k == 6 { Access::Read } else { Access::Modify };
                            ops.push((ea, acc));
                        }
                    }
                    _ => return None,
                }
            }
        }
        0x1..=0x3 => {
            size = match op >> 12 {
                1 => 1,
                3 => 2,
                _ => 4,
            };
            let src = r.low(op, size)?;
            let dst = r.ea((op >> 6) & 7, (op >> 9) & 7, size)?;
            ops.push((src, Access::Read));
            ops.push((dst, Access::Write));
        }
        0x4 => match op {
            0x4afc => flow = Flow::End,
            0x4e70 | 0x4e71 | 0x4e76 => {}
            0x4e72 => {
                r.word()?;
            }
            0x4e73 | 0x4e75 | 0x4e77 => flow = Flow::End,
            0x4e40..=0x4e4f => flow = Flow::End,
            0x4e50..=0x4e57 => {
                r.word()?;
                ops.push((Ea::A((op & 7) as u8), Access::Modify));
            }
            0x4e58..=0x4e5f => ops.push((Ea::A((op & 7) as u8), Access::Modify)),
            0x4e60..=0x4e67 => ops.push((Ea::A((op & 7) as u8), Access::Read)),
            0x4e68..=0x4e6f => ops.push((Ea::A((op & 7) as u8), Access::Write)),
            _ if op & 0xffc0 == 0x4e80 || op & 0xffc0 == 0x4ec0 => {
                ops.push((r.low(op, 4)?, Access::Control));
                flow = if op & 0x40 == 0 {
                    Flow::Call
                } else {
                    Flow::Jump
                };
            }
            _ if op & 0xf1c0 == 0x41c0 => {
                size = 4;
                ops.push((r.low(op, 4)?, Access::Address));
                ops.push((Ea::A(dreg), Access::Write));
            }
            _ if op & 0xf1c0 == 0x4180 => {
                size = 2;
                ops.push((r.low(op, 2)?, Access::Read));
                ops.push((Ea::D(dreg), Access::Read));
            }
            _ if op & 0xffc0 == 0x40c0 => {
                size = 2;
                ops.push((r.low(op, 2)?, Access::Write));
            }
            _ if op & 0xffc0 == 0x44c0 || op & 0xffc0 == 0x46c0 => {
                size = 2;
                ops.push((r.low(op, 2)?, Access::Read));
            }
            _ if matches!(op & 0xff00, 0x4000 | 0x4200 | 0x4400 | 0x4600) => {
                size = size2((op >> 6) & 3)?;
                let acc = if op & 0xff00 == 0x4200 {
                    Access::Write
                } else {
                    Access::Modify
                };
                ops.push((r.low(op, size)?, acc));
            }
            _ if op & 0xffc0 == 0x4800 => {
                size = 1;
                ops.push((r.low(op, 1)?, Access::Modify));
            }
            _ if op & 0xfff8 == 0x4840 => {
                size = 4;
                ops.push((Ea::D((op & 7) as u8), Access::Modify));
            }
            _ if op & 0xffc0 == 0x4840 => {
                size = 4;
                ops.push((r.low(op, 4)?, Access::Address));
            }
            _ if op & 0xffb8 == 0x4880 => {
                size = if op & 0x40 != 0 { 4 } else { 2 };
                ops.push((Ea::D((op & 7) as u8), Access::Modify));
            }
            _ if op & 0xfb80 == 0x4880 => {
                size = if op & 0x40 != 0 { 4 } else { 2 };
                let mask = r.word()?;
                let ea = r.low(op, size)?;
                if op & 0x0400 != 0 {
                    ops.push((ea, Access::Read));
                    clobbers = mask;
                } else {
                    ops.push((ea, Access::Write));
                }
            }
            _ if op & 0xffc0 == 0x4ac0 => {
                size = 1;
                ops.push((r.low(op, 1)?, Access::Modify));
            }
            _ if op & 0xff00 == 0x4a00 => {
                size = size2((op >> 6) & 3)?;
                ops.push((r.low(op, size)?, Access::Read));
            }
            _ => return None,
        },
        0x5 => {
            if (op >> 6) & 3 == 3 {
                if mode == 1 {
                    let at = pc.wrapping_add(2);
                    let d = r.word()? as i16 as i32 as u32;
                    ops.push((Ea::D((op & 7) as u8), Access::Modify));
                    ops.push((Ea::Pc(at.wrapping_add(d)), Access::Control));
                    flow = Flow::Branch;
                } else {
                    size = 1;
                    ops.push((r.low(op, 1)?, Access::Write));
                }
            } else {
                size = size2((op >> 6) & 3)?;
                ops.push((r.low(op, size)?, Access::Modify));
            }
        }
        0x6 => {
            let d8 = (op & 0xff) as u8;
            let d = if d8 == 0 {
                r.word()? as i16 as i32 as u32
            } else {
                d8 as i8 as i32 as u32
            };
            ops.push((Ea::Pc(pc.wrapping_add(2).wrapping_add(d)), Access::Control));
            flow = match (op >> 8) & 0xf {
                0 => Flow::Jump,
                1 => Flow::Call,
                _ => Flow::Branch,
            };
        }
        0x7 => {
            if op & 0x0100 != 0 {
                return None;
            }
            size = 4;
            ops.push((Ea::D(dreg), Access::Write));
        }
        0x8 | 0xc => {
            let opmode = (op >> 6) & 7;
            if opmode == 3 || opmode == 7 {
                // divu/divs (8), mulu/muls (c).
                size = 2;
                ops.push((r.low(op, 2)?, Access::Read));
                ops.push((Ea::D(dreg), Access::Modify));
            } else if op & 0x1f0 == 0x100 {
                // sbcd/abcd: registers or predecrement pairs.
                size = 1;
                let (y, x) = ((op & 7) as u8, dreg);
                if op & 8 != 0 {
                    ops.push((Ea::Pre(y), Access::Read));
                    ops.push((Ea::Pre(x), Access::Modify));
                } else {
                    ops.push((Ea::D(y), Access::Read));
                    ops.push((Ea::D(x), Access::Modify));
                }
            } else if op >> 12 == 0xc && matches!(op & 0x1f8, 0x140 | 0x148 | 0x188) {
                // exg.
                size = 4;
                let (x, y) = (dreg, (op & 7) as u8);
                let (a, b) = match op & 0x1f8 {
                    0x140 => (Ea::D(x), Ea::D(y)),
                    0x148 => (Ea::A(x), Ea::A(y)),
                    _ => (Ea::D(x), Ea::A(y)),
                };
                ops.push((a, Access::Modify));
                ops.push((b, Access::Modify));
            } else {
                size = size2(opmode & 3)?;
                let ea = r.low(op, size)?;
                if opmode < 4 {
                    ops.push((ea, Access::Read));
                    ops.push((Ea::D(dreg), Access::Modify));
                } else {
                    ops.push((Ea::D(dreg), Access::Read));
                    ops.push((ea, Access::Modify));
                }
            }
        }
        0x9 | 0xd => {
            let opmode = (op >> 6) & 7;
            if opmode == 3 || opmode == 7 {
                size = if opmode == 7 { 4 } else { 2 };
                ops.push((r.low(op, size)?, Access::Read));
                ops.push((Ea::A(dreg), Access::Modify));
            } else if opmode >= 4 && mode <= 1 {
                // subx/addx.
                size = size2(opmode & 3)?;
                let (y, x) = ((op & 7) as u8, dreg);
                if mode == 1 {
                    ops.push((Ea::Pre(y), Access::Read));
                    ops.push((Ea::Pre(x), Access::Modify));
                } else {
                    ops.push((Ea::D(y), Access::Read));
                    ops.push((Ea::D(x), Access::Modify));
                }
            } else {
                size = size2(opmode & 3)?;
                let ea = r.low(op, size)?;
                if opmode < 4 {
                    ops.push((ea, Access::Read));
                    ops.push((Ea::D(dreg), Access::Modify));
                } else {
                    ops.push((Ea::D(dreg), Access::Read));
                    ops.push((ea, Access::Modify));
                }
            }
        }
        0xb => {
            let opmode = (op >> 6) & 7;
            if opmode == 3 || opmode == 7 {
                size = if opmode == 7 { 4 } else { 2 };
                ops.push((r.low(op, size)?, Access::Read));
                ops.push((Ea::A(dreg), Access::Read));
            } else if opmode < 3 {
                size = size2(opmode)?;
                ops.push((r.low(op, size)?, Access::Read));
                ops.push((Ea::D(dreg), Access::Read));
            } else if mode == 1 {
                size = size2(opmode & 3)?;
                ops.push((Ea::Post((op & 7) as u8), Access::Read));
                ops.push((Ea::Post(dreg), Access::Read));
            } else {
                size = size2(opmode & 3)?;
                ops.push((Ea::D(dreg), Access::Read));
                ops.push((r.low(op, size)?, Access::Modify));
            }
        }
        0xe => {
            if (op >> 6) & 3 == 3 {
                size = 2;
                ops.push((r.low(op, 2)?, Access::Modify));
            } else {
                size = size2((op >> 6) & 3)?;
                ops.push((Ea::D((op & 7) as u8), Access::Modify));
            }
        }
        // Line A and line F traps.
        _ => flow = Flow::End,
    }
    Some(Insn {
        len: r.pos as u32,
        size,
        ops,
        flow,
        clobbers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(bytes: &[u8]) -> Insn {
        decode(bytes, 0x1000).unwrap()
    }

    #[test]
    fn library_calls_and_absolute_forms() {
        // jsr -$1e(a5)
        let i = d(&[0x4e, 0xad, 0xff, 0xe2]);
        assert_eq!((i.len, i.flow), (4, Flow::Call));
        assert_eq!(i.ops, vec![(Ea::Disp(5, -0x1e), Access::Control)]);
        // movea.l $4.w,a6
        let i = d(&[0x2c, 0x78, 0x00, 0x04]);
        assert_eq!(
            i.ops,
            vec![(Ea::AbsW(4), Access::Read), (Ea::A(6), Access::Write)]
        );
        // movea.l $4.l,a6
        let i = d(&[0x2c, 0x79, 0, 0, 0, 4]);
        assert_eq!((i.len, i.ops[0]), (6, (Ea::AbsL(4), Access::Read)));
        // move.b #$7f,$bfed01.l: the immediate's word comes before the destination's long.
        let i = d(&[0x13, 0xfc, 0x00, 0x7f, 0x00, 0xbf, 0xed, 0x01]);
        assert_eq!(i.len, 8);
        assert_eq!(
            i.ops,
            vec![
                (Ea::Imm(0x7f), Access::Read),
                (Ea::AbsL(0xbfed01), Access::Write)
            ]
        );
        // btst #7,$bfe001.l
        let i = d(&[0x08, 0x39, 0x00, 0x07, 0x00, 0xbf, 0xe0, 0x01]);
        assert_eq!((i.len, i.ops[0]), (8, (Ea::AbsL(0xbfe001), Access::Read)));
        // move.b $bfec01.l,$1f78.l
        let i = d(&[0x13, 0xf9, 0x00, 0xbf, 0xec, 0x01, 0, 0, 0x1f, 0x78]);
        assert_eq!(i.len, 10);
        assert_eq!(i.ops[0], (Ea::AbsL(0xbfec01), Access::Read));
        // move.l $26(a1),$1e32.l
        let i = d(&[0x23, 0xe9, 0x00, 0x26, 0, 0, 0x1e, 0x32]);
        assert_eq!(i.ops[0], (Ea::Disp(1, 0x26), Access::Read));
        // move.l $6c.w,$1e26.l and move.l #$1eda,$6c.w
        assert_eq!(
            d(&[0x23, 0xf8, 0, 0x6c, 0, 0, 0x1e, 0x26]).ops[0].0,
            Ea::AbsW(0x6c)
        );
        let i = d(&[0x21, 0xfc, 0, 0, 0x1e, 0xda, 0, 0x6c]);
        assert_eq!(
            i.ops,
            vec![
                (Ea::Imm(0x1eda), Access::Read),
                (Ea::AbsW(0x6c), Access::Write)
            ]
        );
        // move.l $1abe(pc),$80(a6): PC-relative from the extension word.
        let i = d(&[0x2d, 0x7a, 0x1a, 0xbe, 0x00, 0x80]);
        assert_eq!(i.ops[0].0, Ea::Pc(0x1002 + 0x1abe));
    }

    #[test]
    fn flow_and_odd_forms() {
        assert_eq!(d(&[0x4e, 0x75]).flow, Flow::End);
        assert_eq!(d(&[0x4e, 0x73]).flow, Flow::End);
        // bra.w and beq.b
        assert_eq!(d(&[0x60, 0x00, 0x00, 0x10]).flow, Flow::Jump);
        assert_eq!(d(&[0x67, 0x04]).ops[0].0, Ea::Pc(0x1006));
        // dbra d7
        let i = d(&[0x51, 0xcf, 0xff, 0xf6]);
        assert_eq!((i.len, i.flow), (4, Flow::Branch));
        // movem.l (a7)+,d0-d7/a0-a6
        let i = d(&[0x4c, 0xdf, 0x7f, 0xff]);
        assert_eq!((i.len, i.clobbers), (4, 0x7fff));
        // ext.w d0 is not movem
        assert_eq!(d(&[0x48, 0x80]).len, 2);
        // lea $dff000.l,a6
        let i = d(&[0x4d, 0xf9, 0x00, 0xdf, 0xf0, 0x00]);
        assert_eq!(
            i.ops,
            vec![
                (Ea::AbsL(0xdff000), Access::Address),
                (Ea::A(6), Access::Write)
            ]
        );
        // cmpi.l #$45534c21,$437c8.l
        assert_eq!(
            d(&[0x0c, 0xb9, 0x45, 0x53, 0x4c, 0x21, 0, 4, 0x37, 0xc8]).len,
            10
        );
        // movea.l (a7)+,a1 writes a1
        assert_eq!(d(&[0x22, 0x5f]).ops[1], (Ea::A(1), Access::Write));
        // moveq #0,d0
        assert_eq!(d(&[0x70, 0x00]).ops, vec![(Ea::D(0), Access::Write)]);
    }
}
