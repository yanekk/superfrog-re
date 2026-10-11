//! The `ATN!` decompressor: a port of the game's own 68000 routine (stage 1.5 `+0x2DA`, named
//! `atn_decompress` in `re/symbols.toml`), byte-identical to its emulation in `tools/atn.py` on
//! every chunk that decodes there (docs/03, DESIGN §2.4).
//!
//! The chunk is `"ATN!" | decomp_size | comp_size | stream…`, and a 46-byte trailer sits at
//! `comp_size` from the magic. The stream is read backward from `comp_size` down to the magic, and
//! the output is written backward from `decomp_size` down to 0. The routine first copies the
//! trailer's first 12 bytes over the header, so the stream's last 12 bytes read are those.
//!
//! Register names in the comments are the routine's: `a3` source, `a4` output, `d3` bit buffer,
//! `d2` literal count, `d4` match length, `a1` the 28-byte table copied from the trailer.

use std::fmt;

use crate::carve::MAX_DECOMP;

const MAGIC: &[u8; 4] = b"ATN!";
const HEADER: usize = 12;
/// Trailer bytes: 12 header replacements, `d2` (long), `d3` (word), 28-byte table.
const TRAILER: usize = 46;
/// Base added to the literal count, per match class (`atn_lenbase_tab`, `+0x42A`).
const LIT_BASE: [u8; 4] = [0x06, 0x0a, 0x0a, 0x12];
/// Bits in the literal count: index = class, +4 for the short base 2, +8 for `LIT_BASE` (`+0x42E`).
const LIT_BITS: [u8; 12] = [1, 1, 1, 1, 2, 3, 3, 4, 4, 5, 7, 14];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtnError {
    /// The chunk does not start with `ATN!`.
    BadMagic,
    /// The chunk ends before its header or its trailer (which sits at `comp_size`).
    Truncated,
    /// A match at output offset `at` copies from outside the output.
    Overrun { at: usize },
    /// The stream does not describe `expected` bytes: it ran out of input, or ended, at `got`.
    LengthMismatch { expected: u32, got: u32 },
    /// `decomp_size` is beyond the carver's sanity bound; nothing was allocated.
    TooLarge { decomp_size: u32 },
}

impl fmt::Display for AtnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtnError::BadMagic => write!(f, "not an ATN! chunk"),
            AtnError::Truncated => write!(f, "ATN! chunk is cut short before its trailer"),
            AtnError::Overrun { at } => {
                write!(
                    f,
                    "ATN! match at output offset {at} reads outside the output"
                )
            }
            AtnError::LengthMismatch { expected, got } => {
                write!(f, "ATN! stream gives {got} bytes, header says {expected}")
            }
            AtnError::TooLarge { decomp_size } => {
                write!(
                    f,
                    "ATN! header claims {decomp_size} bytes, beyond any real chunk"
                )
            }
        }
    }
}

impl std::error::Error for AtnError {}

/// Decompress one chunk: `chunk` is the carved bytes starting at `ATN!`, including the trailer.
///
/// Allocates exactly `decomp_size` bytes, once. Never panics.
pub fn decompress(chunk: &[u8]) -> Result<Vec<u8>, AtnError> {
    if chunk.len() < 4 || &chunk[..4] != MAGIC {
        return Err(AtnError::BadMagic);
    }
    if chunk.len() < HEADER {
        return Err(AtnError::Truncated);
    }
    let be32 =
        |at: usize| u32::from_be_bytes([chunk[at], chunk[at + 1], chunk[at + 2], chunk[at + 3]]);
    let decomp_size = be32(4);
    let comp_size = be32(8) as usize;
    if comp_size < HEADER || chunk.len() < comp_size.saturating_add(TRAILER) {
        return Err(AtnError::Truncated);
    }
    if decomp_size >= MAX_DECOMP {
        return Err(AtnError::TooLarge { decomp_size });
    }
    let trailer = &chunk[comp_size..comp_size + TRAILER];
    // `move.l (a2)+,-(a0)` ×3: the trailer's first three longs land over the header in reverse.
    let mut header = [0u8; HEADER];
    header[8..12].copy_from_slice(&trailer[0..4]);
    header[4..8].copy_from_slice(&trailer[4..8]);
    header[0..4].copy_from_slice(&trailer[8..12]);
    let d3_word = u16::from_be_bytes([trailer[16], trailer[17]]);
    let mut table = [0u8; 28];
    table.copy_from_slice(&trailer[18..46]);

    let mut d = Decoder {
        chunk,
        header,
        // `bmi` on the d3 word: a clear top bit means the stream's last byte is not part of it.
        src: if d3_word & 0x8000 != 0 {
            comp_size
        } else {
            comp_size - 1
        },
        bits: d3_word as u8,
        out: vec![0; decomp_size as usize],
        pos: decomp_size as i64,
    };
    let mut literals = be32(comp_size + 12) as u64;
    loop {
        // h0+$312: copy `d2` literal bytes.
        for _ in 0..literals {
            let b = d.byte(decomp_size)?;
            d.put(b);
        }
        // h0+$31c: done once the output pointer reaches the start. The original also needs the
        // input to be exactly used up; it stops even if it wrote below the start, which is refused
        // here instead, by decoding on without writing to learn the stream's real length.
        if d.pos <= 0 && d.src == 0 {
            return if d.pos == 0 {
                Ok(d.out)
            } else {
                Err(d.mismatch(decomp_size))
            };
        }
        // h0+$334..$3a4: the match length `d4` and its class `d0`, by a unary prefix.
        let (len, class) = if !d.bit(decomp_size)? {
            (2, 0)
        } else if !d.bit(decomp_size)? {
            (3, 1)
        } else if !d.bit(decomp_size)? {
            (4, 2)
        } else if !d.bit(decomp_size)? {
            (5, 3)
        } else if !d.bit(decomp_size)? {
            (d.read_bits(3, decomp_size)? as u8 + 6, 3)
        } else {
            (d.byte(decomp_size)?, 3)
        };
        // h0+$3a6..$3dc: the literal run that follows the match, `d2`.
        let (base, idx) = if !d.bit(decomp_size)? {
            (0, class)
        } else if d.bit(decomp_size)? {
            (LIT_BASE[class] as u64, class + 8)
        } else {
            (2, class + 4)
        };
        literals = d.read_bits(LIT_BITS[idx], decomp_size)? + base;
        // h0+$3de..$41e: the match offset. `movea.w` sign-extends the table's base word.
        let (base, idx) = if !d.bit(decomp_size)? {
            (0i16, class)
        } else if d.bit(decomp_size)? {
            (word(&table, 8 + 2 * class), class + 8)
        } else {
            (word(&table, 2 * class), class + 4)
        };
        let extra = d.read_bits(table[0x10 + idx], decomp_size)? as u32;
        let delta = 1i32.wrapping_add(base as i32).wrapping_add(extra as i32) as i64;
        // h0+$420: copy `d4` bytes backward from `a4 + delta`; a length byte of 0 copies 256.
        let n = if len == 0 { 256 } else { len as i64 };
        for _ in 0..n {
            let from = d.pos - 1 + delta;
            if d.pos > 0 {
                let b = match usize::try_from(from) {
                    Ok(f) if f < d.out.len() => d.out[f],
                    _ => {
                        return Err(AtnError::Overrun {
                            at: d.pos as usize - 1,
                        });
                    }
                };
                d.put(b);
            } else {
                d.pos -= 1;
            }
        }
    }
}

fn word(table: &[u8; 28], at: usize) -> i16 {
    i16::from_be_bytes([table[at], table[at + 1]])
}

struct Decoder<'a> {
    chunk: &'a [u8],
    /// The 12 bytes the routine writes over the header.
    header: [u8; HEADER],
    /// `a3 - chunk`: the next byte read is `src - 1`.
    src: usize,
    /// `d3.b`: the bit buffer, with a marker bit below the unread bits.
    bits: u8,
    out: Vec<u8>,
    /// `a4 - dest`: the next byte written is `pos - 1`. Below 0 nothing is written.
    pos: i64,
}

impl Decoder<'_> {
    /// `move.b -(a3),…`
    fn byte(&mut self, decomp_size: u32) -> Result<u8, AtnError> {
        if self.src == 0 {
            return Err(self.mismatch(decomp_size));
        }
        self.src -= 1;
        Ok(if self.src < HEADER {
            self.header[self.src]
        } else {
            self.chunk[self.src]
        })
    }

    /// `add.b d3,d3; bne; move.b -(a3),d3; addx.b d3,d3`, the carry out.
    fn bit(&mut self, decomp_size: u32) -> Result<bool, AtnError> {
        let carry = self.bits >> 7;
        self.bits <<= 1;
        if self.bits != 0 {
            return Ok(carry != 0);
        }
        let b = self.byte(decomp_size)?;
        self.bits = (b << 1) | carry;
        Ok(b & 0x80 != 0)
    }

    /// `n` bits MSB first; `n == 0` reads 256, as `subq.b #1; bne` does, keeping the low 32.
    fn read_bits(&mut self, n: u8, decomp_size: u32) -> Result<u64, AtnError> {
        let count = if n == 0 { 256 } else { n as u32 };
        let mut v: u32 = 0;
        for _ in 0..count {
            v = (v << 1) | self.bit(decomp_size)? as u32;
        }
        Ok(v as u64)
    }

    fn put(&mut self, b: u8) {
        self.pos -= 1;
        if self.pos >= 0 {
            self.out[self.pos as usize] = b;
        }
    }

    fn mismatch(&self, expected: u32) -> AtnError {
        // Bytes produced so far, counting those decoded past the start without being written.
        let got = (expected as i64 - self.pos).clamp(0, u32::MAX as i64);
        AtnError::LengthMismatch {
            expected,
            got: got as u32,
        }
    }
}
