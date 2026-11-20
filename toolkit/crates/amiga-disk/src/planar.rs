//! Planar bitmaps as the Amiga keeps them, decoded to one colour index per pixel.
//!
//! *Plane-major*: all of plane 0's rows, then plane 1's, each row `width / 8` bytes, most
//! significant bit leftmost. A pixel's colour index has plane `p` as bit `p`.

/// Colour indices, one byte per pixel, row-major.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    /// Bitplanes: indices are below `1 << depth`.
    pub depth: u8,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for Bitmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bitmap({}x{}x{})", self.width, self.height, self.depth)
    }
}

impl Bitmap {
    /// Bytes a plane-major bitmap of this size takes.
    pub fn planar_len(width: u16, height: u16, depth: u8) -> usize {
        width as usize / 8 * height as usize * depth as usize
    }

    /// Decode a plane-major bitmap from `data` (exactly [`Bitmap::planar_len`] bytes are read).
    pub fn from_planar(data: &[u8], width: u16, height: u16, depth: u8) -> Option<Bitmap> {
        let (w, h) = (width as usize, height as usize);
        let row = w / 8;
        if w % 8 != 0 || depth > 8 || data.len() < row * h * depth as usize {
            return None;
        }
        let mut pixels = vec![0u8; w * h];
        for p in 0..depth as usize {
            let plane = &data[p * row * h..(p + 1) * row * h];
            for (k, &b) in plane.iter().enumerate() {
                if b == 0 {
                    continue;
                }
                let base = k * 8;
                for bit in 0..8 {
                    if b & (0x80 >> bit) != 0 {
                        pixels[base + bit] |= 1 << p;
                    }
                }
            }
        }
        Some(Bitmap {
            width,
            height,
            depth,
            pixels,
        })
    }

    /// The plane-major bytes again: `from_planar(&b.to_planar(), ..) == b`.
    pub fn to_planar(&self) -> Vec<u8> {
        let (w, h) = (self.width as usize, self.height as usize);
        let row = w / 8;
        let mut out = vec![0u8; row * h * self.depth as usize];
        for p in 0..self.depth as usize {
            for (k, px) in self.pixels.iter().enumerate() {
                if px >> p & 1 != 0 {
                    out[p * row * h + k / 8] |= 0x80 >> (k % 8);
                }
            }
        }
        out
    }

    pub fn pixel(&self, x: u16, y: u16) -> u8 {
        self.pixels[y as usize * self.width as usize + x as usize]
    }
}
