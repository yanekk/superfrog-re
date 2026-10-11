//! IFF ANIM: the end sequence's `END1` and `END2` (d3_c09, d3_c10; docs/10 "ANIM").
//!
//! `FORM ANIM` holds one `FORM ILBM` per frame. The first is a full picture (`BMHD`, `CMAP`, a
//! ByteRun1 `BODY` with rows interleaved by plane); each later one is an `ANHD` and a `DLTA` of
//! operation 5 (byte vertical delta). With interleave 0 a delta applies to the frame two back, as
//! a player drawing into two alternating buffers sees it; both start as frame 0.

use crate::error::{AssetError, Reader, Why};
use crate::gamedata::ChunkId;
use crate::palette::Palette;
use crate::picture::Bitmap;
use crate::t7mp::Tag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnimId {
    /// `END1` (d3_c09).
    End1,
    /// `END2` (d3_c10).
    End2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimFrame {
    /// The whole picture as shown at this frame.
    pub bitmap: Bitmap,
    /// `ANHD` reltime: jiffies after the previous frame (0 for frame 0, which has no `ANHD`).
    pub reltime: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anim {
    pub id: AnimId,
    /// From `CMAP`, each 8-bit component's top nibble (the file's low nibbles are zero).
    pub palette: Palette,
    /// The first frame's chunks other than `BMHD`, `CMAP` and `BODY` (`DPPS`, `CRNG`, `DPAN`,
    /// `CAMG`), as stored.
    pub chunks: Vec<(Tag, Vec<u8>)>,
    pub frames: Vec<AnimFrame>,
}

/// One IFF chunk: tag, payload, and where the next one starts (payloads are padded to even).
fn chunk<'a>(r: &Reader<'a>, at: usize) -> Result<(Tag, &'a [u8], usize), AssetError> {
    let tag = Tag(r.bytes(at, 4)?.try_into().unwrap());
    let len = r.u32(at + 4)? as usize;
    let payload = r.bytes(at + 8, len)?;
    Ok((tag, payload, at + 8 + len + (len & 1)))
}

/// Decode a `FORM ANIM`. Never panics; anything outside what the two files use is refused.
pub fn decode_anim(id: AnimId, chunk_id: ChunkId, bytes: &[u8]) -> Result<Anim, AssetError> {
    let what = format!("{id:?} animation");
    let r = Reader::new(bytes, chunk_id, &what);
    let bad = |m: String| r.err(Why::Malformed(m));
    let (form, anim, _) = chunk(&r, 0)?;
    if form != Tag(*b"FORM") || anim.get(..4) != Some(b"ANIM") {
        return Err(bad("not a FORM ANIM".into()));
    }
    let end = 8 + anim.len();
    let mut at = 12;
    let mut frames = Vec::new();
    let mut header = None;
    let mut chunks = Vec::new();
    let mut palette = Palette([0; 32]);
    // Plane-major working buffers: the two a double-buffered player alternates between.
    let mut bufs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    while at < end {
        let (tag, form, next) = chunk(&r, at)?;
        if tag != Tag(*b"FORM") || form.get(..4) != Some(b"ILBM") {
            return Err(bad(format!("byte {at}: not a FORM ILBM")));
        }
        let f = Reader::new(form, chunk_id, &what);
        let mut k = 4;
        let (mut body, mut dlta, mut anhd) = (None, None, None);
        while k < form.len() {
            let (t, p, n) = chunk(&f, k)?;
            match &t.0 {
                b"BMHD" => header = Some(p),
                b"CMAP" => {
                    for (i, c) in p.chunks_exact(3).take(32).enumerate() {
                        palette.0[i] =
                            (c[0] as u16 >> 4) << 8 | (c[1] as u16 >> 4) << 4 | c[2] as u16 >> 4;
                    }
                }
                b"BODY" => body = Some(p),
                b"DLTA" => dlta = Some(p),
                b"ANHD" => anhd = Some(p),
                _ if frames.is_empty() => chunks.push((t, p.to_vec())),
                _ => {}
            }
            k = n;
        }
        let h = header.ok_or_else(|| bad("no BMHD before the first frame's body".into()))?;
        let hr = Reader::new(h, chunk_id, &what);
        let (w, ht, depth) = (hr.u16(0)?, hr.u16(2)?, hr.u8(8)?);
        if w == 0 || w % 16 != 0 || ht == 0 || depth == 0 || depth > 8 {
            return Err(bad(format!("BMHD {w}x{ht}x{depth}")));
        }
        let bpr = w as usize / 8;
        let plane = bpr * ht as usize;
        if frames.is_empty() {
            let (masking, compression) = (hr.u8(9)?, hr.u8(10)?);
            if masking != 0 || compression != 1 {
                return Err(bad(format!(
                    "BMHD masking {masking}, compression {compression}"
                )));
            }
            let body = body.ok_or_else(|| bad("the first frame has no BODY".into()))?;
            let rows = unpack_byterun1(
                &Reader::new(body, chunk_id, &what),
                bpr * depth as usize * ht as usize,
            )?;
            let mut buf = vec![0u8; plane * depth as usize];
            for y in 0..ht as usize {
                for p in 0..depth as usize {
                    let src = (y * depth as usize + p) * bpr;
                    buf[p * plane + y * bpr..][..bpr].copy_from_slice(&rows[src..src + bpr]);
                }
            }
            bufs = [buf.clone(), buf];
            frames.push(frame(&bufs[0], w, ht, depth, 0));
        } else {
            let a = anhd.ok_or_else(|| bad(format!("frame {} has no ANHD", frames.len())))?;
            let ar = Reader::new(a, chunk_id, &what);
            let (op, reltime, interleave, bits) = (ar.u8(0)?, ar.u32(14)?, ar.u8(18)?, ar.u32(20)?);
            if op != 5 || !(interleave == 0 || interleave == 2) || bits != 0 {
                return Err(bad(format!(
                    "frame {}: ANHD operation {op}, interleave {interleave}, bits {bits:#x}",
                    frames.len()
                )));
            }
            let d = dlta.ok_or_else(|| bad(format!("frame {} has no DLTA", frames.len())))?;
            // Frame n lands in buffer n % 2, which holds frame n - 2.
            let buf = &mut bufs[frames.len() % 2];
            apply_op5(
                &Reader::new(d, chunk_id, &what),
                buf,
                bpr,
                ht as usize,
                depth as usize,
            )?;
            frames.push(frame(buf, w, ht, depth, reltime));
        }
        at = next;
    }
    if frames.is_empty() {
        return Err(bad("no frames".into()));
    }
    Ok(Anim {
        id,
        palette,
        chunks,
        frames,
    })
}

fn frame(buf: &[u8], w: u16, h: u16, depth: u8, reltime: u32) -> AnimFrame {
    AnimFrame {
        bitmap: Bitmap::from_planar(buf, w, h, depth).expect("buffer sized for the bitmap"),
        reltime,
    }
}

/// ByteRun1: `n` in 0..=127 copies `n + 1` bytes, -127..=-1 repeats the next byte `1 - n` times,
/// -128 does nothing. Exactly `len` bytes must come out.
fn unpack_byterun1(r: &Reader, len: usize) -> Result<Vec<u8>, AssetError> {
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
        return Err(r.err(Why::Malformed(format!("BODY unpacks past its {len} bytes"))));
    }
    Ok(out)
}

/// ANIM operation 5 into a plane-major buffer: per plane a pointer (16 longs at the start; 0 =
/// unchanged), then per byte column an op count and ops: 0 = run (count, value), bit 7 set =
/// copy that many bytes, else skip that many rows.
fn apply_op5(
    r: &Reader,
    buf: &mut [u8],
    bpr: usize,
    h: usize,
    depth: usize,
) -> Result<(), AssetError> {
    let overrun = |p: usize, col: usize| {
        r.err(Why::Malformed(format!(
            "delta runs off the bottom (plane {p}, column {col})"
        )))
    };
    for p in 0..16 {
        let ptr = r.u32(4 * p)? as usize;
        if ptr == 0 || p >= 8 {
            continue;
        }
        if p >= depth {
            return Err(r.err(Why::Malformed(format!("delta for plane {p} of {depth}"))));
        }
        let plane = &mut buf[p * bpr * h..(p + 1) * bpr * h];
        let mut at = ptr;
        for col in 0..bpr {
            let ops = r.u8(at)?;
            at += 1;
            let mut row = 0usize;
            for _ in 0..ops {
                let op = r.u8(at)?;
                at += 1;
                if op == 0 {
                    let (n, v) = (r.u8(at)? as usize, r.u8(at + 1)?);
                    at += 2;
                    if row + n > h {
                        return Err(overrun(p, col));
                    }
                    for y in row..row + n {
                        plane[y * bpr + col] = v;
                    }
                    row += n;
                } else if op & 0x80 != 0 {
                    let n = (op & 0x7f) as usize;
                    let src = r.bytes(at, n)?;
                    at += n;
                    if row + n > h {
                        return Err(overrun(p, col));
                    }
                    for (k, &b) in src.iter().enumerate() {
                        plane[(row + k) * bpr + col] = b;
                    }
                    row += n;
                } else {
                    row += op as usize;
                }
            }
        }
    }
    Ok(())
}
