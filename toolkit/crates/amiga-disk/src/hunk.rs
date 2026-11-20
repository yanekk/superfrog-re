//! AmigaDOS hunk executables (`HUNK_HEADER`, `0x3F3`): the header, the walk over its blocks, and
//! [`parse`], the hunks as the file has them.
//!
//! Hunks are kept unrelocated, so a long the loader relocates reads as an offset into its target
//! hunk. A hunk the header declares and the file does not carry is kept as absent, sized from the
//! header, because a loader still gives it room. A loader with rules of its own (where it stops,
//! what it clears) walks [`Blocks`] itself rather than taking [`parse`]'s result.

use crate::error::ReadError;
use crate::read::Reader;

pub const HUNK_UNIT: u32 = 0x3e7;
pub const HUNK_NAME: u32 = 0x3e8;
pub const HUNK_CODE: u32 = 0x3e9;
pub const HUNK_DATA: u32 = 0x3ea;
pub const HUNK_BSS: u32 = 0x3eb;
pub const HUNK_RELOC32: u32 = 0x3ec;
pub const HUNK_RELOC16: u32 = 0x3ed;
pub const HUNK_RELOC8: u32 = 0x3ee;
pub const HUNK_EXT: u32 = 0x3ef;
pub const HUNK_SYMBOL: u32 = 0x3f0;
pub const HUNK_DEBUG: u32 = 0x3f1;
pub const HUNK_END: u32 = 0x3f2;
pub const HUNK_HEADER: u32 = 0x3f3;

/// Most hunks [`parse`] accepts in one file.
const MAX_HUNKS: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkKind {
    Code,
    Data,
    Bss,
}

/// One HUNK_RELOC32 group: longs at `offsets` in this hunk get hunk `target`'s base added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reloc {
    pub target: u16,
    pub offsets: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub kind: HunkKind,
    /// The header's memory flags: bit 0 chip, bit 1 fast (bits 30 and 31 of the size long).
    pub mem: u8,
    /// Bytes the loader gives the hunk, from the header.
    pub size: u32,
    /// Its contents (empty for BSS and for an absent hunk). May be shorter than `size`.
    pub data: Vec<u8>,
    pub relocs: Vec<Reloc>,
    /// False for a hunk the header declares and the file does not carry (kept as BSS).
    pub present: bool,
}

/// One hunk's entry in the header's size table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HunkSize {
    /// The long as the file has it, memory flags in bits 30 and 31.
    pub raw: u32,
    /// The memory flags: bit 0 chip, bit 1 fast.
    pub mem: u8,
    /// Bytes: the low 30 bits times four.
    pub bytes: u32,
}

/// `HUNK_HEADER` after its resident library names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The table size long: how many hunks the file declares.
    pub count: u32,
    pub first: u32,
    pub last: u32,
    /// Byte offset of the count long.
    pub table_at: usize,
    /// One entry per hunk `first..=last`.
    pub sizes: Vec<HunkSize>,
    /// Byte offset of the first block after the header.
    pub end: usize,
}

/// Read the header. Only its shape is checked (`last >= first`); whether the table suits a loader
/// is the loader's to decide.
pub fn header(bytes: &[u8]) -> Result<Header, ReadError> {
    let r = Reader::new(bytes);
    if r.u32(0)? != HUNK_HEADER {
        return Err(r.bad(0, "no HUNK_HEADER"));
    }
    let mut at = 4;
    // Resident library names: a list of (length in longs, name), ending with 0.
    loop {
        let n = r.u32(at)? as usize;
        at += 4;
        if n == 0 {
            break;
        }
        at = n
            .checked_mul(4)
            .and_then(|b| b.checked_add(at))
            .ok_or_else(|| r.bad(at - 4, "library name too long"))?;
    }
    let (count, first, last) = (r.u32(at)?, r.u32(at + 4)?, r.u32(at + 8)?);
    let table_at = at;
    if last < first {
        return Err(r.bad(at, format!("hunk table {count} hunks, {first}..{last}")));
    }
    at += 12;
    let mut sizes = Vec::new();
    for _ in first..=last {
        let raw = r.u32(at)?;
        at += 4;
        let mem = (raw >> 30) as u8;
        if mem == 3 {
            at += 4; // an extra long of memory attributes
        }
        sizes.push(HunkSize {
            raw,
            mem,
            bytes: (raw & 0x3fff_ffff) * 4,
        });
    }
    Ok(Header {
        count,
        first,
        last,
        table_at,
        sizes,
        end: at,
    })
}

/// One block after the header, its contents borrowed from the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block<'a> {
    /// `HUNK_UNIT` or `HUNK_NAME` (per [`Block::kind`]): a name, in longs, zero-padded.
    Unit(&'a [u8]),
    Name(&'a [u8]),
    Code(&'a [u8]),
    Data(&'a [u8]),
    Bss { longs: u32 },
    /// `(target hunk, offsets)` per group, in file order; the target is not checked here.
    Reloc32(Vec<(u32, Vec<u32>)>),
    Symbol,
    Debug,
    End,
}

impl Block<'_> {
    /// The block's type long.
    pub fn kind(&self) -> u32 {
        match self {
            Block::Unit(_) => HUNK_UNIT,
            Block::Name(_) => HUNK_NAME,
            Block::Code(_) => HUNK_CODE,
            Block::Data(_) => HUNK_DATA,
            Block::Bss { .. } => HUNK_BSS,
            Block::Reloc32(_) => HUNK_RELOC32,
            Block::Symbol => HUNK_SYMBOL,
            Block::Debug => HUNK_DEBUG,
            Block::End => HUNK_END,
        }
    }
}

/// The blocks from a byte offset (normally [`Header::end`]) to the end of the file, each with the
/// offset of its type long. A block type this walk cannot measure (`HUNK_RELOC16`, `HUNK_EXT`, an
/// unknown one) is an error, after which the walk ends; a loader that stops at such a block looks
/// at [`Blocks::peek`] first. A type long's memory bits (30, 31) are dropped.
pub struct Blocks<'a> {
    r: Reader<'a>,
    at: usize,
    done: bool,
}

impl<'a> Blocks<'a> {
    pub fn new(bytes: &'a [u8], at: usize) -> Blocks<'a> {
        Blocks {
            r: Reader::new(bytes),
            at,
            done: false,
        }
    }

    /// Where the next block starts.
    pub fn pos(&self) -> usize {
        self.at
    }

    /// The next block's offset and type long as the file has it (memory bits kept), without
    /// reading it; `None` at the end of the file or after an error.
    pub fn peek(&self) -> Option<Result<(usize, u32), ReadError>> {
        if self.done || self.at >= self.r.data.len() {
            return None;
        }
        Some(self.r.u32(self.at).map(|t| (self.at, t)))
    }

    fn block(&mut self) -> Result<(usize, Block<'a>), ReadError> {
        let r = self.r;
        let start = self.at;
        let t = r.u32(start)? & 0x3fff_ffff;
        let mut at = start + 4;
        let block = match t {
            HUNK_UNIT | HUNK_NAME | HUNK_CODE | HUNK_DATA => {
                let n = r.u32(at)? as usize * 4;
                let data = r.bytes(at + 4, n)?;
                at += 4 + n;
                match t {
                    HUNK_UNIT => Block::Unit(data),
                    HUNK_NAME => Block::Name(data),
                    HUNK_CODE => Block::Code(data),
                    _ => Block::Data(data),
                }
            }
            HUNK_BSS => {
                let longs = r.u32(at)?;
                at += 4;
                Block::Bss { longs }
            }
            HUNK_RELOC32 => {
                let mut groups = Vec::new();
                loop {
                    let n = r.u32(at)? as usize;
                    at += 4;
                    if n == 0 {
                        break;
                    }
                    let target = r.u32(at)?;
                    let offsets = (0..n)
                        .map(|k| r.u32(at + 4 + 4 * k))
                        .collect::<Result<Vec<_>, _>>()?;
                    groups.push((target, offsets));
                    at += 4 + 4 * n;
                }
                Block::Reloc32(groups)
            }
            // Symbol and debug contents are stepped over unread, so one cut short at the end of the
            // file ends the walk rather than failing it.
            HUNK_DEBUG => {
                at += 4 + 4 * r.u32(at)? as usize;
                Block::Debug
            }
            HUNK_SYMBOL => {
                // (length in longs, name, value) entries, ending with 0.
                loop {
                    let n = r.u32(at)? as usize;
                    at += 4;
                    if n == 0 {
                        break;
                    }
                    at += 4 * n + 4;
                }
                Block::Symbol
            }
            HUNK_END => Block::End,
            _ => return Err(r.bad(start, format!("unknown hunk block {t:#x} at byte {start}"))),
        };
        self.at = at;
        Ok((start, block))
    }
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Result<(usize, Block<'a>), ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.at >= self.r.data.len() {
            return None;
        }
        let b = self.block();
        self.done = b.is_err();
        Some(b)
    }
}

/// The hunks of an executable whose header numbers them from 0, at most 64. Never panics; anything
/// unexpected is a [`ReadError`].
pub fn parse(bytes: &[u8]) -> Result<Vec<Hunk>, ReadError> {
    let h = header(bytes)?;
    let (count, first, last) = (h.count, h.first, h.last);
    if first != 0 || (last - first).checked_add(1) != Some(count) || count > MAX_HUNKS {
        return Err(ReadError::Malformed {
            at: h.table_at,
            what: format!("hunk table {count} hunks, {first}..{last}"),
        });
    }
    let mut hunks: Vec<Hunk> = h
        .sizes
        .iter()
        .map(|s| Hunk {
            kind: HunkKind::Bss,
            mem: s.mem,
            size: s.bytes,
            data: Vec::new(),
            relocs: Vec::new(),
            present: false,
        })
        .collect();
    let mut index = 0;
    for b in Blocks::new(bytes, h.end) {
        let (at, block) = b?;
        let kind = block.kind();
        let hunk = hunks.get_mut(index).ok_or_else(|| ReadError::Malformed {
            at,
            what: format!("block {kind:#x} past the last declared hunk"),
        })?;
        match block {
            Block::Code(d) | Block::Data(d) => {
                hunk.data = d.to_vec();
                hunk.kind = if kind == HUNK_CODE {
                    HunkKind::Code
                } else {
                    HunkKind::Data
                };
                hunk.present = true;
            }
            Block::Bss { .. } => {
                hunk.kind = HunkKind::Bss;
                hunk.present = true;
            }
            Block::Reloc32(groups) => {
                for (target, offsets) in groups {
                    if target >= count {
                        return Err(ReadError::Malformed {
                            at,
                            what: format!("relocation to hunk {target}"),
                        });
                    }
                    hunk.relocs.push(Reloc {
                        target: target as u16,
                        offsets,
                    });
                }
            }
            Block::End => index += 1,
            Block::Unit(_) | Block::Name(_) | Block::Symbol | Block::Debug => {}
        }
    }
    Ok(hunks)
}
