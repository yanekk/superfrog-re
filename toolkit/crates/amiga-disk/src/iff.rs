//! IFF: the chunk walk, ByteRun1, and `FORM ILBM` pictures.
//!
//! An IFF chunk is a four-byte tag, a big-endian length and that many bytes, padded to even. An
//! ILBM's `BMHD` gives its size, depth, masking and compression; its `BODY` holds the rows with the
//! planes interleaved (row 0 of plane 0, row 0 of plane 1, …), raw or ByteRun1-packed per row.

use crate::error::ReadError;
use crate::palette::Palette;
use crate::planar::Bitmap;
use crate::read::Reader;

/// One IFF chunk at `at` in `r`: its tag, its payload as a reader, and where the next one starts.
pub(crate) fn chunk<'a>(r: &Reader<'a>, at: usize) -> Result<([u8; 4], Reader<'a>, usize), ReadError> {
    let tag: [u8; 4] = r.bytes(at, 4)?.try_into().unwrap();
    let len = r.u32(at + 4)? as usize;
    let payload = r.sub(at + 8, len)?;
    Ok((tag, payload, at + 8 + len + (len & 1)))
}

/// The chunks of `bytes` from `at` to its end, as `(tag, payload)`. Payload lengths are checked;
/// tags are not.
pub fn chunks(bytes: &[u8], at: usize) -> Result<Vec<([u8; 4], &[u8])>, ReadError> {
    let r = Reader::new(bytes);
    let mut out = Vec::new();
    let mut k = at;
    while k < bytes.len() {
        let (tag, p, next) = chunk(&r, k)?;
        out.push((tag, p.data));
        k = next;
    }
    Ok(out)
}

/// ByteRun1: `n` in 0..=127 copies `n + 1` bytes, -127..=-1 repeats the next byte `1 - n` times,
/// -128 does nothing. Exactly `len` bytes must come out.
pub fn byterun1(packed: &[u8], len: usize) -> Result<Vec<u8>, ReadError> {
    unpack_byterun1(&Reader::new(packed), len)
}

pub(crate) fn unpack_byterun1(r: &Reader, len: usize) -> Result<Vec<u8>, ReadError> {
    let mut out = Vec::with_capacity(len);
    let mut at = 0;
    while out.len() < len {
        let n = r.u8(at)? as i8;
        at += 1;
        if n >= 0 {
            out.extend_from_slice(r.bytes(at, n as usize + 1)?);
            at += n as usize + 1;
        } else if n != -128 {
            let b = r.u8(at)?;
            at += 1;
            out.extend(std::iter::repeat_n(b, 1 + (-(n as i16)) as usize));
        }
    }
    if out.len() != len {
        return Err(r.bad(at, format!("BODY unpacks past its {len} bytes")));
    }
    Ok(out)
}

/// The `BMHD` fields a reader needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bmhd {
    pub width: u16,
    pub height: u16,
    pub depth: u8,
    /// 0 none, 1 a mask plane, 2 a transparent colour, 3 lasso.
    pub masking: u8,
    /// 0 none, 1 ByteRun1.
    pub compression: u8,
}

impl Bmhd {
    /// Bytes per row of one plane.
    pub fn row_bytes(&self) -> usize {
        self.width as usize / 8
    }
}

/// A `BMHD` payload, refused unless the width is a multiple of 16 (so rows need no padding) and
/// the depth is 1..=8.
pub(crate) fn bmhd(h: &Reader) -> Result<Bmhd, ReadError> {
    let (width, height, depth) = (h.u16(0)?, h.u16(2)?, h.u8(8)?);
    if width == 0 || width % 16 != 0 || height == 0 || depth == 0 || depth > 8 {
        return Err(h.bad(0, format!("BMHD {width}x{height}x{depth}")));
    }
    Ok(Bmhd {
        width,
        height,
        depth,
        masking: h.u8(9)?,
        compression: h.u8(10)?,
    })
}

/// A `CMAP` payload as OCS colours: each 8-bit component's top nibble, the first 32 entries.
pub(crate) fn cmap(p: &[u8], palette: &mut Palette) {
    for (i, c) in p.chunks_exact(3).take(32).enumerate() {
        palette.0[i] = (c[0] as u16 >> 4) << 8 | (c[1] as u16 >> 4) << 4 | c[2] as u16 >> 4;
    }
}

/// A `BODY` as plane-major bytes. The header's masking must be 0 or 2 (no mask plane).
pub(crate) fn body(b: &Reader, h: &Bmhd) -> Result<Vec<u8>, ReadError> {
    let (bpr, rows, depth) = (h.row_bytes(), h.height as usize, h.depth as usize);
    let len = bpr * depth * rows;
    let interleaved = match h.compression {
        0 => b.bytes(0, len)?.to_vec(),
        1 => unpack_byterun1(b, len)?,
        c => return Err(b.bad(0, format!("BMHD compression {c}"))),
    };
    let plane = bpr * rows;
    let mut out = vec![0u8; plane * depth];
    for y in 0..rows {
        for p in 0..depth {
            let src = (y * depth + p) * bpr;
            out[p * plane + y * bpr..][..bpr].copy_from_slice(&interleaved[src..src + bpr]);
        }
    }
    Ok(out)
}

/// A `FORM ILBM` picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ilbm {
    pub bitmap: Bitmap,
    /// From `CMAP`, if the file has one.
    pub palette: Option<Palette>,
    /// The chunks other than `BMHD`, `CMAP` and `BODY` (`CAMG`, `CRNG`, …), as stored.
    pub chunks: Vec<([u8; 4], Vec<u8>)>,
}

/// Decode a `FORM ILBM`. Never panics. Refused: a mask plane (masking 1 or 3), compression other
/// than none or ByteRun1, and any size [`Bmhd`] refuses.
pub fn ilbm(bytes: &[u8]) -> Result<Ilbm, ReadError> {
    let r = Reader::new(bytes);
    let (tag, form, _) = chunk(&r, 0)?;
    if &tag != b"FORM" || form.data.get(..4) != Some(b"ILBM") {
        return Err(r.bad(0, "not a FORM ILBM"));
    }
    let (mut header, mut body_chunk, mut palette, mut chunks) = (None, None, None, Vec::new());
    let mut k = 4;
    while k < form.data.len() {
        let (t, p, n) = chunk(&form, k)?;
        match &t {
            b"BMHD" => header = Some(bmhd(&p)?),
            b"CMAP" => cmap(p.data, palette.get_or_insert(Palette([0; 32]))),
            b"BODY" => body_chunk = Some(p),
            _ => chunks.push((t, p.data.to_vec())),
        }
        k = n;
    }
    let h = header.ok_or_else(|| form.bad(0, "no BMHD"))?;
    if h.masking != 0 && h.masking != 2 {
        return Err(form.bad(0, format!("BMHD masking {}", h.masking)));
    }
    let b = body_chunk.ok_or_else(|| form.bad(0, "no BODY"))?;
    let planes = body(&b, &h)?;
    Ok(Ilbm {
        bitmap: Bitmap::from_planar(&planes, h.width, h.height, h.depth).expect("buffer sized for the bitmap"),
        palette,
        chunks,
    })
}
