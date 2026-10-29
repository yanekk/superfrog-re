//! The diff picture beside a divergence. It shows the original's picture, which is derived from
//! the game's disks, so the caller's folder must be outside any repository.

use std::path::{Path, PathBuf};

use amiga_chips::{Picture, to_rgb12};
use amiga_oracle::Rgb12Frame;

use crate::compare::PixelMismatch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureDiff {
    /// original | engine | differences, side by side.
    pub path: PathBuf,
    pub mismatch: PixelMismatch,
}

/// Write `<dir>/<case>-frame<N>.png`: the original, the engine, and the original darkened with
/// every differing pixel in red.
pub fn write_diff_png(
    dir: &Path,
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
    let colours = to_rgb12(ours);
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
    let path = dir.join(format!("{case}-frame{frame}.png"));
    let written = std::fs::create_dir_all(dir)
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

/// The CRC-32 a PNG chunk carries (ISO 3309, reflected `$EDB88320`).
pub fn crc32<'a>(bytes: impl Iterator<Item = &'a u8>) -> u32 {
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

    /// A synthetic picture against a synthetic frame: the diff is three panels wide, with the
    /// differing pixel in red, written to the caller's folder.
    #[test]
    fn the_diff_png_is_written_to_the_callers_folder() {
        let dir = std::env::temp_dir().join(format!("amiga-conform-report-{}", std::process::id()));
        let ours = Picture {
            left: 0,
            top: 0x2c,
            width: 2,
            height: 1,
            pixels: vec![1, 1],
            palettes: vec![[0; 32]],
            splits: Vec::new(),
        };
        let theirs = Rgb12Frame {
            left: 0,
            top: 0x2c,
            width: 2,
            height: 1,
            pixels: vec![0, 0x0f0],
            hires_detail: false,
        };
        let m = crate::compare::picture(&ours, &theirs).unwrap();
        let d = write_diff_png(&dir, "case", 7, &ours, &theirs, m.clone());
        assert_eq!(d.path, dir.join("case-frame7.png"));
        assert_eq!(d.mismatch, m);
        let bytes = std::fs::read(&d.path).unwrap();
        assert!(bytes.starts_with(b"\x89PNG"));
        // IHDR's width: 2 + 4 + 2 + 4 + 2.
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 14);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
