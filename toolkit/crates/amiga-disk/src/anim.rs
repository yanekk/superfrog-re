//! IFF ANIM with byte vertical delta (operation 5).
//!
//! `FORM ANIM` holds one `FORM ILBM` per frame. The first is a full picture (`BMHD`, `CMAP`, a
//! ByteRun1 `BODY`); each later one is an `ANHD` and a `DLTA` of operation 5. With interleave 0 a
//! delta applies to the frame two back, as a player drawing into two alternating buffers sees it;
//! both start as frame 0.

use crate::error::ReadError;
use crate::iff::{bmhd, body, chunk, cmap};
use crate::palette::Palette;
use crate::planar::Bitmap;
use crate::read::Reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimFrame {
    /// The whole picture as shown at this frame.
    pub bitmap: Bitmap,
    /// `ANHD` reltime: jiffies after the previous frame (0 for frame 0, which has no `ANHD`).
    pub reltime: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anim {
    /// From `CMAP`, each 8-bit component's top nibble.
    pub palette: Palette,
    /// The first frame's chunks other than `BMHD`, `CMAP` and `BODY` (`DPPS`, `CRNG`, `DPAN`,
    /// `CAMG`), as stored.
    pub chunks: Vec<([u8; 4], Vec<u8>)>,
    pub frames: Vec<AnimFrame>,
}

/// Decode a `FORM ANIM`. Never panics. Refused: a first frame that is not an unmasked ByteRun1
/// picture, a delta other than operation 5 (interleave 0 or 2, no option bits), and a later frame
/// whose `BMHD` differs from the first's.
pub fn anim(bytes: &[u8]) -> Result<Anim, ReadError> {
    let r = Reader::new(bytes);
    let (form, anim, _) = chunk(&r, 0)?;
    if &form != b"FORM" || anim.data.get(..4) != Some(b"ANIM") {
        return Err(r.bad(0, "not a FORM ANIM"));
    }
    let end = 8 + anim.data.len();
    let mut at = 12;
    let mut frames = Vec::new();
    let mut header: Option<Reader> = None;
    let mut chunks = Vec::new();
    let mut palette = Palette([0; 32]);
    // Plane-major working buffers: the two a double-buffered player alternates between.
    let mut bufs: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    while at < end {
        let (tag, form, next) = chunk(&r, at)?;
        if &tag != b"FORM" || form.data.get(..4) != Some(b"ILBM") {
            return Err(r.bad(at, format!("byte {at}: not a FORM ILBM")));
        }
        let mut k = 4;
        let (mut body_chunk, mut dlta, mut anhd) = (None, None, None);
        while k < form.data.len() {
            let (t, p, n) = chunk(&form, k)?;
            match &t {
                // The buffers are sized by the first frame's; a later one may only repeat it.
                b"BMHD" if !frames.is_empty() && header.map(|h| h.data) != Some(p.data) => {
                    return Err(form.bad(k, format!("frame {}: a BMHD of another size", frames.len())));
                }
                b"BMHD" => header = Some(p),
                b"CMAP" => cmap(p.data, &mut palette),
                b"BODY" => body_chunk = Some(p),
                b"DLTA" => dlta = Some(p),
                b"ANHD" => anhd = Some(p),
                _ if frames.is_empty() => chunks.push((t, p.data.to_vec())),
                _ => {}
            }
            k = n;
        }
        let hr = header.ok_or_else(|| form.bad(0, "no BMHD before the first frame's body"))?;
        let h = bmhd(&hr)?;
        let (w, ht, depth) = (h.width, h.height, h.depth);
        let bpr = h.row_bytes();
        if frames.is_empty() {
            if h.masking != 0 || h.compression != 1 {
                return Err(hr.bad(
                    0,
                    format!("BMHD masking {}, compression {}", h.masking, h.compression),
                ));
            }
            let b = body_chunk.ok_or_else(|| form.bad(0, "the first frame has no BODY"))?;
            let buf = body(&b, &h)?;
            bufs = [buf.clone(), buf];
            frames.push(frame(&bufs[0], w, ht, depth, 0));
        } else {
            let a = anhd.ok_or_else(|| form.bad(0, format!("frame {} has no ANHD", frames.len())))?;
            let (op, reltime, interleave, bits) = (a.u8(0)?, a.u32(14)?, a.u8(18)?, a.u32(20)?);
            if op != 5 || !(interleave == 0 || interleave == 2) || bits != 0 {
                return Err(a.bad(
                    0,
                    format!(
                        "frame {}: ANHD operation {op}, interleave {interleave}, bits {bits:#x}",
                        frames.len()
                    ),
                ));
            }
            let d = dlta.ok_or_else(|| form.bad(0, format!("frame {} has no DLTA", frames.len())))?;
            // Frame n lands in buffer n % 2, which holds frame n - 2.
            let buf = &mut bufs[frames.len() % 2];
            apply_op5(&d, buf, bpr, ht as usize, depth as usize)?;
            frames.push(frame(buf, w, ht, depth, reltime));
        }
        at = next;
    }
    if frames.is_empty() {
        return Err(r.bad(0, "no frames"));
    }
    Ok(Anim {
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

/// ANIM operation 5 into a plane-major buffer: per plane a pointer (16 longs at the start; 0 =
/// unchanged), then per byte column an op count and ops: 0 = run (count, value), bit 7 set =
/// copy that many bytes, else skip that many rows.
fn apply_op5(r: &Reader, buf: &mut [u8], bpr: usize, h: usize, depth: usize) -> Result<(), ReadError> {
    for p in 0..16 {
        let ptr = r.u32(4 * p)? as usize;
        if ptr == 0 || p >= 8 {
            continue;
        }
        if p >= depth {
            return Err(r.bad(4 * p, format!("delta for plane {p} of {depth}")));
        }
        let plane = &mut buf[p * bpr * h..(p + 1) * bpr * h];
        let mut at = ptr;
        for col in 0..bpr {
            let overrun = |at: usize| r.bad(at, format!("delta runs off the bottom (plane {p}, column {col})"));
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
                        return Err(overrun(at));
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
                        return Err(overrun(at));
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
