//! One hunk of the main program as its loader leaves it in memory: its bytes, zero-padded to the
//! size the header gives it, every `HUNK_RELOC32` long with its target hunk's base added
//! (plans/forest-dumb-level-1 T08). The engine reads the original's step and frame tables from
//! hunk 0; a pointer among them is only equal with the original's when the hunks sit where the
//! oracle loads them, so the caller passes those bases.
//!
//! A reader of the AmigaDOS hunk format this program uses (`HUNK_HEADER`, then per hunk `CODE`,
//! `DATA` or `BSS`, `RELOC32`, `SYMBOL`, `DEBUG`, `END`) and no more: the toolkit's `amiga-disk`
//! reads it in full, but would bring a zip decoder into this pure crate.

use crate::{AssetError, Why};

const HUNK_UNIT: u32 = 0x3e7;
const HUNK_NAME: u32 = 0x3e8;
const HUNK_CODE: u32 = 0x3e9;
const HUNK_DATA: u32 = 0x3ea;
const HUNK_BSS: u32 = 0x3eb;
const HUNK_RELOC32: u32 = 0x3ec;
const HUNK_SYMBOL: u32 = 0x3f0;
const HUNK_DEBUG: u32 = 0x3f1;
const HUNK_END: u32 = 0x3f2;
const HUNK_HEADER: u32 = 0x3f3;

struct R<'a> {
    b: &'a [u8],
    file: &'a str,
}

impl R<'_> {
    fn bad(&self, at: usize, what: impl Into<String>) -> AssetError {
        AssetError::new(
            self.file,
            Why::Malformed {
                at,
                what: what.into(),
            },
        )
    }

    fn long(&self, at: usize) -> Result<u32, AssetError> {
        self.b
            .get(at..at.wrapping_add(4))
            .map(|l| u32::from_be_bytes([l[0], l[1], l[2], l[3]]))
            .ok_or_else(|| self.bad(at, "ends inside a long"))
    }

    /// `n` longs from `at`, as a byte offset past them.
    fn skip(&self, at: usize, n: u32) -> Result<usize, AssetError> {
        let end = (n as usize)
            .checked_mul(4)
            .and_then(|k| k.checked_add(at))
            .filter(|&e| e <= self.b.len())
            .ok_or_else(|| self.bad(at, format!("{n} longs run past the end")))?;
        Ok(end)
    }
}

/// Hunk `hunk` of `exe` (named `file` in errors) as loaded with hunk `k` at `bases[k]`.
pub fn loaded_hunk(
    exe: &[u8],
    file: &str,
    hunk: usize,
    bases: &[u32],
) -> Result<Vec<u8>, AssetError> {
    let r = R { b: exe, file };
    if r.long(0)? != HUNK_HEADER {
        return Err(r.bad(0, "no HUNK_HEADER"));
    }
    let mut at = 4;
    loop {
        let n = r.long(at)?;
        at = r.skip(at + 4, n)?;
        if n == 0 {
            break;
        }
    }
    let (count, first, last) = (r.long(at)?, r.long(at + 4)?, r.long(at + 8)?);
    if first != 0 || last < first || (last - first + 1) != count || count as usize != bases.len() {
        return Err(r.bad(
            at,
            format!(
                "hunk table {count} hunks {first}..{last}, not the {} expected",
                bases.len()
            ),
        ));
    }
    at += 12;
    let mut sizes = Vec::new();
    for _ in 0..count {
        let raw = r.long(at)?;
        at += if raw >> 30 == 3 { 8 } else { 4 };
        sizes.push((raw & 0x3fff_ffff) as usize * 4);
    }
    let mut out = vec![0u8; sizes[hunk]];
    let mut index = 0;
    while index <= hunk && at < exe.len() {
        let id = r.long(at)? & 0x3fff_ffff;
        at += 4;
        let mine = index == hunk;
        match id {
            HUNK_CODE | HUNK_DATA => {
                let n = r.long(at)?;
                let end = r.skip(at + 4, n)?;
                if mine {
                    let data = &exe[at + 4..end];
                    if data.len() > out.len() {
                        out.resize(data.len(), 0);
                    }
                    out[..data.len()].copy_from_slice(data);
                }
                at = end;
            }
            HUNK_BSS => at += 4,
            HUNK_RELOC32 => loop {
                let n = r.long(at)?;
                if n == 0 {
                    at += 4;
                    break;
                }
                let target = r.long(at + 4)? as usize;
                let base = *bases
                    .get(target)
                    .ok_or_else(|| r.bad(at + 4, format!("a relocation to hunk {target}")))?;
                for k in 0..n as usize {
                    let off = r.long(at + 8 + 4 * k)? as usize;
                    if mine {
                        let l = out
                            .get_mut(off..off + 4)
                            .ok_or_else(|| r.bad(at, format!("a relocation at {off:#x}")))?;
                        let v = u32::from_be_bytes([l[0], l[1], l[2], l[3]]).wrapping_add(base);
                        l.copy_from_slice(&v.to_be_bytes());
                    }
                }
                at = r.skip(at + 8, n)?;
            },
            HUNK_SYMBOL => loop {
                let n = r.long(at)?;
                if n == 0 {
                    at += 4;
                    break;
                }
                at = r.skip(at + 4, n)? + 4;
            },
            HUNK_DEBUG | HUNK_UNIT | HUNK_NAME => {
                let n = r.long(at)?;
                at = r.skip(at + 4, n)?;
            }
            HUNK_END => index += 1,
            other => return Err(r.bad(at - 4, format!("block {other:#x}"))),
        }
    }
    if index <= hunk {
        return Err(r.bad(at, format!("hunk {hunk} is missing")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn longs(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|l| l.to_be_bytes()).collect()
    }

    /// Two hunks: code of 3 longs (size 4), its long 1 pointing at hunk 1 offset 8; BSS.
    fn exe() -> Vec<u8> {
        longs(&[
            HUNK_HEADER,
            0,
            2,
            0,
            1,
            4,
            0x4000_0010,
            HUNK_CODE,
            3,
            0x1111_1111,
            8,
            0x3333_3333,
            HUNK_RELOC32,
            1,
            1,
            4,
            0,
            HUNK_SYMBOL,
            1,
            0x4142_4344,
            0,
            0,
            HUNK_END,
            HUNK_BSS,
            0x10,
            HUNK_END,
        ])
    }

    #[test]
    fn a_hunk_is_padded_and_relocated() {
        let h0 = loaded_hunk(&exe(), "x", 0, &[0x100, 0x2000]).unwrap();
        assert_eq!(h0, longs(&[0x1111_1111, 0x2008, 0x3333_3333, 0]));
        let h1 = loaded_hunk(&exe(), "x", 1, &[0x100, 0x2000]).unwrap();
        assert_eq!(h1, vec![0; 0x40]);
    }

    #[test]
    fn a_bad_file_is_an_error_naming_it() {
        let e = loaded_hunk(&[0; 8], "PROG", 0, &[0, 0]).unwrap_err();
        assert_eq!(e.file, "PROG");
        let mut short = exe();
        short.truncate(40);
        assert!(loaded_hunk(&short, "PROG", 0, &[0, 0]).is_err());
        // The wrong number of bases.
        assert!(loaded_hunk(&exe(), "PROG", 0, &[0]).is_err());
    }
}
