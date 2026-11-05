//! The game's `ESL!` files, unpacked in Rust (`docs/08-packers.md`). The app holds no Python and no
//! 68k runner, so this is what the engine reads level data through; `tools/unpack.py` runs the game's
//! own routine under Musashi and is what this is checked against.
//!
//! The format is PowerPacker's under another magic. The decoder follows the independent one in
//! `tools/test_unpack.py`. The game's routine trusts its input and has no failure path, so the checks
//! here are `unpack.py`'s: a damaged file is an `Err` naming what is wrong, never a panic and never
//! plausible bytes of the claimed size.

use std::path::Path;

use amiga_disk::ofs::OfsImage;

use crate::{read_adf, zip_paths};

const ESL: &[u8] = b"ESL!";
const HUNK_HEADER: &[u8] = &[0, 0, 3, 0xf3];
/// Where a self-unpacking stub keeps its payload, as `unpack.py`'s `STUB_WINDOW`.
const STUB_WINDOW: usize = 4096;

/// `packed` unpacked. A file that is not packed is returned unchanged. The crack intro's StoneCracker
/// executable is refused: the game never loads it, so there is no decoder for it here.
pub fn unpack(packed: &[u8]) -> Result<Vec<u8>, String> {
    if packed.starts_with(ESL) {
        return esl(packed);
    }
    if packed.starts_with(HUNK_HEADER)
        && packed[..packed.len().min(STUB_WINDOW)]
            .windows(4)
            .any(|w| w == b"S404")
    {
        return Err(
            "a StoneCracker 4 executable: no Rust unpacker, the game does not load it".into(),
        );
    }
    Ok(packed.to_vec())
}

/// One of the game's files, by `disk<N>/<name>` or by a name found on only one disk, read from the
/// zips in `disks` and unpacked, all in memory: nothing is written.
pub fn game_file(disks: &Path, name: &str) -> Result<Vec<u8>, String> {
    let (only, bare) = match name.split_once('/') {
        Some((d, rest)) if d.eq_ignore_ascii_case("disk1") => (Some(0), rest),
        Some((d, rest)) if d.eq_ignore_ascii_case("disk2") => (Some(1), rest),
        _ => (None, name),
    };
    let zips = zip_paths(disks)?;
    let mut found = Vec::new();
    // Disk 2 first: it holds the level files, so the common case inflates one zip only when named.
    for i in [1, 0] {
        if only.is_some_and(|o| o != i) {
            continue;
        }
        let adf = read_adf(&zips[i])?;
        let image =
            OfsImage::parse(&adf).map_err(|e| format!("{}: its ADF: {e}", zips[i].display()))?;
        if let Ok(bytes) = image.read_file(bare) {
            found.push((i, bytes));
        }
    }
    match found.len() {
        0 => Err(format!(
            "{name}: no such file on {} in {}",
            match only {
                Some(i) => format!("disk {}", i + 1),
                None => "either disk".into(),
            },
            disks.display()
        )),
        1 => {
            let (i, bytes) = found.pop().unwrap();
            unpack(&bytes).map_err(|e| format!("disk{}/{bare} in {}: {e}", i + 1, disks.display()))
        }
        _ => Err(format!(
            "{name}: on both disks in {}; name one as disk1/{bare} or disk2/{bare}",
            disks.display()
        )),
    }
}

/// The layout, from `docs/08-packers.md`: magic, four offset widths, the bit stream, a 24-bit
/// unpacked size and the count of bits to skip before the stream starts.
fn esl(data: &[u8]) -> Result<Vec<u8>, String> {
    let malformed =
        |why: &str| format!("not a well-formed ESL! file ({} bytes): {why}", data.len());
    if data.len() < 16 || !data.len().is_multiple_of(4) {
        return Err(malformed("too short or not whole longwords"));
    }
    let n = data.len();
    let size = u32::from_be_bytes([0, data[n - 4], data[n - 3], data[n - 2]]) as usize;
    let skip = data[n - 1] as u32;
    if size == 0 {
        return Err(malformed("it claims 0 bytes unpacked"));
    }
    if skip > 32 {
        return Err(malformed(&format!(
            "it skips {skip} bits, more than a longword"
        )));
    }
    let widths = [data[4], data[5], data[6], data[7]];
    if widths.iter().any(|&w| w == 0 || w > 15) {
        return Err(malformed(&format!("offset widths {widths:?}")));
    }
    let mut bits = Bits {
        stream: &data[8..n - 4],
        at: n - 12,
        buf: 0,
        left: 0,
    };
    let mut out = vec![0u8; size];
    let mut o = size;
    bits.read(skip)
        .ok_or_else(|| short("in its skipped bits", o))?;
    'stream: while o > 0 {
        if bits.read(1).ok_or_else(|| short("at a token", o))? == 0 {
            let mut run = 1;
            let x = loop {
                let x = bits.read(2).ok_or_else(|| short("in a literal count", o))?;
                if x != 3 {
                    break x;
                }
                run += 3;
            };
            let run = run + x as usize;
            if run > o {
                return Err(format!(
                    "a literal run of {run} bytes with {o} left to write"
                ));
            }
            for _ in 0..run {
                o -= 1;
                out[o] = bits.read(8).ok_or_else(|| short("in a literal", o))? as u8;
            }
            if o == 0 {
                break 'stream;
            }
        }
        // A literal run is always followed by a match, with no token bit.
        let x = bits.read(2).ok_or_else(|| short("in a match length", o))?;
        let mut width = widths[x as usize] as u32;
        let mut run = x as usize + 2;
        let offset;
        if x == 3 {
            if bits
                .read(1)
                .ok_or_else(|| short("in a match's width bit", o))?
                == 0
            {
                width = 7;
            }
            offset = bits
                .read(width)
                .ok_or_else(|| short("in a match offset", o))? as usize;
            loop {
                let y = bits.read(3).ok_or_else(|| short("in a match length", o))?;
                run += y as usize;
                if y != 7 {
                    break;
                }
            }
        } else {
            offset = bits
                .read(width)
                .ok_or_else(|| short("in a match offset", o))? as usize;
        }
        if run > o {
            return Err(format!("a match of {run} bytes with {o} left to write"));
        }
        if o + offset >= size {
            return Err(format!(
                "a match {} bytes back from byte {o} of {size}",
                offset + 1
            ));
        }
        for _ in 0..run {
            o -= 1;
            out[o] = out[o + 1 + offset];
        }
    }
    // Every real file ends in the stream's first byte with at most one bit unread; the routine
    // would return happily on a stream that stops short, so this is what catches a wrong size.
    if bits.at != 0 || bits.left > 1 {
        return Err(format!(
            "the stream did not end where it starts ({} bytes and {} bits unread)",
            bits.at, bits.left
        ));
    }
    Ok(out)
}

fn short(what: &str, o: usize) -> String {
    format!("the stream ran out {what} with {o} bytes still to write")
}

/// The stream read backwards from its end, each byte low bit first; a value's first bit is its top.
struct Bits<'a> {
    stream: &'a [u8],
    at: usize,
    buf: u8,
    left: u32,
}

impl Bits<'_> {
    fn read(&mut self, count: u32) -> Option<u32> {
        let mut value = 0;
        for _ in 0..count {
            if self.left == 0 {
                self.at = self.at.checked_sub(1)?;
                self.buf = self.stream[self.at];
                self.left = 8;
            }
            value = value << 1 | (self.buf & 1) as u32;
            self.buf >>= 1;
            self.left -= 1;
        }
        Some(value)
    }
}
