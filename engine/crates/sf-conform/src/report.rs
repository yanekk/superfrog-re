//! What a divergence says, and the diff PNG beside it.
//!
//! Diff images go to `SF_CONFORM_OUT`, else `<temp dir>/sf-conform/`: they show the original's
//! picture, which is derived from the disks, so they never go in the repository (DESIGN §2.3).

use std::fmt;
use std::path::PathBuf;

use sf_core::Picture;
use sf_core::layout::VarDef;
use sf_oracle::Rgb12Frame;

use crate::compare::PixelMismatch;

/// The first frame at which the engine and the original differ, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    /// Counted from the sync point.
    pub frame: u32,
    /// The first `LAYOUT` variable that differs: (its piece, the original's value, the engine's).
    pub first_var: Option<(VarDef, u32, u32)>,
    pub picture_diff: Option<PictureDiff>,
    pub paula_diff: Option<String>,
    /// The picture rendered from the original's memory at this frame, where it differs.
    pub render_diff: Option<PictureDiff>,
    /// The original's frame compared, counted from the sync point, when it is not `frame`: the
    /// original spent frames loading that the engine does at once (DESIGN §2.1).
    pub oracle_frame: Option<u32>,
    /// Paths of the original the engine reached and does not follow yet.
    pub unported: Vec<&'static str>,
    /// Every differing piece of the first differing variable (up to 40).
    pub var_detail: Vec<String>,
    /// The variables differed on the frame after a lag, where the runner would re-sync the
    /// engine, and the case forbids that (`resync = false`).
    pub resync: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureDiff {
    /// original | engine | differences, side by side.
    pub path: PathBuf,
    pub mismatch: PixelMismatch,
}

impl fmt::Display for Divergence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.oracle_frame {
            Some(o) => writeln!(
                f,
                "diverged at frame {} (the original's frame {o})",
                self.frame
            )?,
            None => writeln!(f, "diverged at frame {}", self.frame)?,
        }
        if self.resync {
            writeln!(
                f,
                "  after a lag; the runner would re-sync here, and the case sets resync = false"
            )?;
        }
        for u in &self.unported {
            writeln!(f, "  the engine reached a path it does not follow yet: {u}")?;
        }
        if let Some((d, theirs, ours)) = &self.first_var {
            writeln!(
                f,
                "  variable {} at ${:06x} ({:?}): original ${theirs:x}, engine ${ours:x}",
                d.name, d.addr, d.width
            )?;
            if self.var_detail.len() > 1 {
                writeln!(
                    f,
                    "    all of it that differs (original/engine): {}",
                    self.var_detail.join(" ")
                )?;
            }
        }
        for (what, p) in [
            ("picture", &self.picture_diff),
            ("render from the original's memory", &self.render_diff),
        ] {
            let Some(p) = p else { continue };
            if let Some((x, y, theirs, ours)) = p.mismatch.first {
                writeln!(
                    f,
                    "  {what}: first pixel ({x}, {y}): original ${theirs:03x}, engine ${ours:03x}"
                )?;
            }
            if let Some((a, b, n)) = p.mismatch.rows {
                writeln!(f, "  {what}: {n} pixels differ, on rows {a} to {b}")?;
            }
            if let Some((theirs, ours)) = &p.mismatch.window {
                writeln!(f, "  display window: original {theirs}, engine {ours}")?;
            }
            writeln!(f, "  diff image: {}", p.path.display())?;
        }
        if let Some(p) = &self.paula_diff {
            writeln!(f, "  paula: {p}")?;
        }
        Ok(())
    }
}

/// Where diff images go.
pub fn out_dir() -> PathBuf {
    std::env::var_os("SF_CONFORM_OUT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("sf-conform"))
}

/// Write `<out>/<case>-frame<N>.png`: the original, the engine, and the original darkened with
/// every differing pixel in red.
pub fn write_diff_png(
    case: &str,
    frame: u32,
    ours: &Picture,
    theirs: &Rgb12Frame,
    mismatch: PixelMismatch,
) -> PictureDiff {
    let (wt, wo) = (theirs.width as usize, ours.width as usize);
    let h = theirs.height.max(ours.height) as usize;
    let gap = 4;
    let width = wt + gap + wo + gap + wt;
    let mut rgb = vec![0x40u8; width * h * 3];
    let mut put = |x: usize, y: usize, c: u16| {
        let i = (y * width + x) * 3;
        rgb[i] = ((c >> 8 & 15) * 17) as u8;
        rgb[i + 1] = ((c >> 4 & 15) * 17) as u8;
        rgb[i + 2] = ((c & 15) * 17) as u8;
    };
    for y in 0..theirs.height {
        for x in 0..theirs.width {
            put(x as usize, y as usize, theirs.pixel(x, y));
        }
    }
    let colours = sf_core::to_rgb12(ours);
    let ours_at = |x: u16, y: u16| colours[y as usize * wo + x as usize];
    for y in 0..ours.height {
        for x in 0..ours.width {
            put(wt + gap + x as usize, y as usize, ours_at(x, y));
        }
    }
    for y in 0..theirs.height {
        for x in 0..theirs.width {
            let t = theirs.pixel(x, y);
            let differs = x >= ours.width || y >= ours.height || ours_at(x, y) != t;
            let c = if differs { 0xF00 } else { t >> 2 & 0x333 };
            put(wt + gap + wo + gap + x as usize, y as usize, c);
        }
    }
    let dir = out_dir();
    let path = dir.join(format!("{case}-frame{frame}.png"));
    let written = std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::write(&path, png(width as u32, h as u32, &rgb)));
    if let Err(e) = written {
        panic!("writing the diff image {}: {e}", path.display());
    }
    PictureDiff { path, mismatch }
}

/// An 8-bit RGB PNG.
pub fn png(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgb.len() + height as usize);
    for row in rgb.chunks_exact(width as usize * 3) {
        raw.push(0);
        raw.extend(row);
    }
    let mut ihdr = Vec::new();
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(
        &mut out,
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6),
    );
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    out.extend(kind);
    out.extend(data);
    let crc = crc32(kind.iter().chain(data));
    out.extend(crc.to_be_bytes());
}

fn crc32<'a>(bytes: impl Iterator<Item = &'a u8>) -> u32 {
    !bytes.fold(!0u32, |mut c, &b| {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ c >> 1
            } else {
                c >> 1
            };
        }
        c
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_png_check_value() {
        // The IEND chunk's CRC is fixed by the spec.
        assert_eq!(crc32(b"IEND".iter()), 0xAE42_6082);
        let p = png(1, 1, &[255, 0, 0]);
        assert!(p.starts_with(b"\x89PNG") && p.ends_with(&[0xAE, 0x42, 0x60, 0x82]));
    }
}
