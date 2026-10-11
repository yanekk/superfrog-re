//! One emulated A500 running the original, stepped a frame at a time.

use sf_core::input::FrameInput;
use sf_disk::{DiskSet, DiskSlot};
use sf_paula::PaulaWrite;

use crate::config::{OracleConfig, OracleError, disk};
use crate::ffi::{self, ErrBuf, SfAmiga};

/// Super-hires entries per lores pixel in the core's texture (shim.h).
const PER_LORES: usize = 4;

/// Lores position of a texture row's first entry: Denise's horizontal counter starts each line at
/// HBLANK_MIN (colour clock 0x12) times 2 (vAmiga `Denise::updateBorderBuffer`, lines 9 onward).
const FIRST_H: usize = 0x12 * 2;

/// The picture inside the display window, one entry per lores pixel, as 12-bit OCS colour
/// `0x0RGB`. The window is the one DIWSTRT/DIWSTOP give at the end of the frame (OCS rules);
/// `left` and `top` are its position in DIWSTRT's units (lores pixel, raster line).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Rgb12Frame {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
    /// Row-major, `width * height` entries.
    pub pixels: Vec<u16>,
    /// Some lores pixel's two hires halves differed: the frame has hires detail this lores
    /// capture drops. False on every frame Superfrog has been seen to draw so far.
    pub hires_detail: bool,
}

impl Rgb12Frame {
    pub fn pixel(&self, x: u16, y: u16) -> u16 {
        self.pixels[y as usize * self.width as usize + x as usize]
    }
}

/// What one frame produced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FrameCapture {
    pub picture: Rgb12Frame,
    /// Paula register writes made during the frame, in order (DESIGN §2.1 (2)).
    pub paula: Vec<PaulaWrite>,
    /// Per-channel DAC taps. Always empty: T00 found none to tap (the core mixes in floating
    /// point for the host); the mixer is judged on `paula` instead.
    pub audio: Vec<i16>,
}

/// Chip RAM (at 0x000000) and slow RAM (at 0xC00000), copied.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct AmigaMemory {
    pub chip: Vec<u8>,
    pub slow: Vec<u8>,
}

/// Where slow ("ranger") RAM sits on the A500.
pub const SLOW_BASE: u32 = 0xC0_0000;

impl AmigaMemory {
    /// The byte at a CPU address, or None outside chip and slow RAM.
    pub fn byte(&self, addr: u32) -> Option<u8> {
        let a = addr as usize;
        if a < self.chip.len() {
            Some(self.chip[a])
        } else if addr >= SLOW_BASE && ((addr - SLOW_BASE) as usize) < self.slow.len() {
            Some(self.slow[(addr - SLOW_BASE) as usize])
        } else {
            None
        }
    }

    /// The big-endian word at `addr`.
    pub fn word(&self, addr: u32) -> Option<u16> {
        Some(u16::from_be_bytes([self.byte(addr)?, self.byte(addr + 1)?]))
    }

    /// The big-endian long at `addr`.
    pub fn long(&self, addr: u32) -> Option<u32> {
        Some((self.word(addr)? as u32) << 16 | self.word(addr + 2)? as u32)
    }

    /// A fast 64-bit digest of both RAMs (not cryptographic; for comparing runs).
    pub fn hash(&self) -> u64 {
        hash_bytes(&self.slow, hash_bytes(&self.chip, 0x5346_4f52_4143_4c45))
    }
}

/// A word-at-a-time multiply-rotate hash: about 0.1 ms per megabyte.
pub(crate) fn hash_bytes(data: &[u8], seed: u64) -> u64 {
    const K: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut h = seed ^ (data.len() as u64).wrapping_mul(K);
    let (words, rest) = data.as_chunks::<8>();
    for w in words {
        h = (h ^ u64::from_le_bytes(*w)).wrapping_mul(K).rotate_left(29);
    }
    for &b in rest {
        h = (h ^ b as u64).wrapping_mul(K).rotate_left(29);
    }
    h ^ (h >> 32)
}

/// The whole machine at one frame boundary, with the frame number and the input then held.
#[derive(Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub frame: u32,
    pub input: FrameInput,
    /// A vAmiga snapshot of the core this crate links (`crate::BUILD`).
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snapshot")
            .field("frame", &self.frame)
            .field("len", &self.bytes.len())
            .finish()
    }
}

/// One emulator instance: an A500 (OCS, 512K chip + 512K slow, no clock, RAM zeroed, PAL) with
/// the owner's Kickstart and disk 1 in df0. Deterministic: the same inputs from the same start give
/// the same frames (FINDINGS 2026-10-11).
pub struct Amiga {
    raw: *mut SfAmiga,
    disks: DiskSet,
    max_frames: u32,
    frame: u32,
    input: FrameInput,
}

// The shim keeps no state shared between instances; an instance may move between threads.
unsafe impl Send for Amiga {}

impl Drop for Amiga {
    fn drop(&mut self) {
        unsafe { ffi::sf_amiga_free(self.raw) };
    }
}

impl Amiga {
    /// Read the ROM and disks (read-only), insert disk 1 and switch on. Frame 0 is next.
    pub fn power_on(cfg: &OracleConfig) -> Result<Amiga, OracleError> {
        let (rom, disks) = cfg.load()?;
        let mut err = ErrBuf::new();
        let raw = unsafe { ffi::sf_amiga_new(rom.as_ptr(), rom.len(), err.ptr(), ErrBuf::LEN) };
        if raw.is_null() {
            return Err(OracleError::Setup(err.message()));
        }
        let mut amiga = Amiga {
            raw,
            disks,
            max_frames: cfg.max_frames,
            frame: 0,
            input: FrameInput::NONE,
        };
        amiga
            .insert(0, DiskSlot::D1)
            .map_err(|e| OracleError::Setup(e.to_string()))?;
        let rc = unsafe { ffi::sf_amiga_power_on(amiga.raw, err.ptr(), ErrBuf::LEN) };
        if rc != 0 {
            return Err(OracleError::Setup(err.message()));
        }
        Ok(amiga)
    }

    /// Run exactly one PAL frame with `input` held, or refuse once `max_frames` frames have run.
    pub fn step(&mut self, input: FrameInput) -> Result<FrameCapture, OracleError> {
        if self.frame >= self.max_frames {
            return Err(OracleError::FrameCap {
                max_frames: self.max_frames,
            });
        }
        let keys = input.keys.to_bytes();
        let mut err = ErrBuf::new();
        let rc = unsafe {
            ffi::sf_amiga_step(
                self.raw,
                input.joy.bits() as u32,
                keys.as_ptr(),
                err.ptr(),
                ErrBuf::LEN,
            )
        };
        if rc != 0 {
            return Err(OracleError::Emulator(format!(
                "frame {}: {}",
                self.frame,
                err.message()
            )));
        }
        self.frame += 1;
        self.input = input;
        Ok(FrameCapture {
            picture: self.picture(),
            paula: self.paula(),
            audio: Vec::new(),
        })
    }

    /// Frames run since power-on (also counting those before a restored snapshot).
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// The frame cap this instance enforces.
    pub fn max_frames(&self) -> u32 {
        self.max_frames
    }

    /// Chip and slow RAM as they are now.
    pub fn memory(&self) -> AmigaMemory {
        unsafe {
            let (mut c, mut s) = (0usize, 0usize);
            let chip = ffi::sf_amiga_chip(self.raw, &mut c);
            let slow = ffi::sf_amiga_slow(self.raw, &mut s);
            AmigaMemory {
                chip: std::slice::from_raw_parts(chip, c).to_vec(),
                slow: if slow.is_null() {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(slow, s).to_vec()
                },
            }
        }
    }

    /// [`AmigaMemory::hash`] of the RAM as it is now, without copying it.
    pub fn memory_hash(&self) -> u64 {
        unsafe {
            let (mut c, mut s) = (0usize, 0usize);
            let chip = std::slice::from_raw_parts(ffi::sf_amiga_chip(self.raw, &mut c), c);
            let slow = ffi::sf_amiga_slow(self.raw, &mut s);
            let slow = if slow.is_null() {
                &[][..]
            } else {
                std::slice::from_raw_parts(slow, s)
            };
            hash_bytes(slow, hash_bytes(chip, 0x5346_4f52_4143_4c45))
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut err = ErrBuf::new();
        let mut len = 0usize;
        let buf = unsafe { ffi::sf_amiga_snapshot(self.raw, &mut len, err.ptr(), ErrBuf::LEN) };
        assert!(
            !buf.is_null(),
            "vAmiga could not take a snapshot: {}",
            err.message()
        );
        let bytes = unsafe { std::slice::from_raw_parts(buf, len).to_vec() };
        unsafe { ffi::sf_buffer_free(buf) };
        Snapshot {
            frame: self.frame,
            input: self.input,
            bytes,
        }
    }

    /// Return to a snapshot of this build. Panics on a snapshot the core refuses, since one this
    /// crate made always loads.
    pub fn restore(&mut self, s: &Snapshot) {
        let keys = s.input.keys.to_bytes();
        let mut err = ErrBuf::new();
        let rc = unsafe {
            ffi::sf_amiga_restore(
                self.raw,
                s.bytes.as_ptr(),
                s.bytes.len(),
                s.input.joy.bits() as u32,
                keys.as_ptr(),
                err.ptr(),
                ErrBuf::LEN,
            )
        };
        assert_eq!(rc, 0, "vAmiga refused a snapshot: {}", err.message());
        self.frame = s.frame;
        self.input = s.input;
    }

    /// Swap the disk in `drive` for `slot`'s, as a player answering the game's prompt does. The
    /// A500 has one drive; anything but 0 panics. The core ejects first and inserts after the
    /// drive's swap delay, as the hardware would see it.
    pub fn insert_disk(&mut self, drive: u8, slot: DiskSlot) {
        if let Err(e) = self.insert(drive, slot) {
            panic!("{e}");
        }
    }

    fn insert(&mut self, drive: u8, slot: DiskSlot) -> Result<(), OracleError> {
        if drive != 0 {
            return Err(OracleError::NoDrive(drive));
        }
        let adf = disk(&self.disks, slot);
        let mut err = ErrBuf::new();
        let rc = unsafe {
            ffi::sf_amiga_insert_disk(
                self.raw,
                drive as i32,
                adf.as_ptr(),
                adf.len(),
                err.ptr(),
                ErrBuf::LEN,
            )
        };
        if rc != 0 {
            return Err(OracleError::Emulator(err.message()));
        }
        Ok(())
    }

    fn paula(&self) -> Vec<PaulaWrite> {
        let mut ptr = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_paula_writes(self.raw, &mut ptr) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, n) }
            .iter()
            .map(|w| PaulaWrite {
                reg: w.reg,
                value: w.value,
                hpos: w.hpos,
                vpos: w.vpos,
            })
            .collect()
    }

    fn picture(&self) -> Rgb12Frame {
        let (mut stride, mut rows) = (0usize, 0usize);
        let (mut strt, mut stop) = (0u16, 0u16);
        let tex = unsafe {
            ffi::sf_amiga_diw(self.raw, &mut strt, &mut stop);
            let p = ffi::sf_amiga_texture(self.raw, &mut stride, &mut rows);
            std::slice::from_raw_parts(p, stride * rows)
        };
        crop(tex, stride, rows, strt, stop)
    }
}

/// The display window of a texture (shim.h's layout), from OCS DIWSTRT/DIWSTOP: start H8 = 0,
/// V8 = 0; stop H8 = 1, V8 = NOT V7. Lores position h is texture entry `4 * (h - FIRST_H)`.
pub(crate) fn crop(
    tex: &[u32],
    stride: usize,
    rows: usize,
    diwstrt: u16,
    diwstop: u16,
) -> Rgb12Frame {
    let (h0, v0) = ((diwstrt & 0xFF) as usize, (diwstrt >> 8) as usize);
    let h1 = ((diwstop & 0xFF) | 0x100) as usize;
    let v1 = ((diwstop >> 8) | if diwstop & 0x8000 == 0 { 0x100 } else { 0 }) as usize;
    let h0 = h0.max(FIRST_H);
    let h1 = h1.min(stride / PER_LORES + FIRST_H);
    let v1 = v1.min(rows);
    let (w, h) = (h1.saturating_sub(h0), v1.saturating_sub(v0));
    let mut pixels = Vec::with_capacity(w * h);
    let mut hires_detail = false;
    for y in v0..v0 + h {
        let row = &tex[y * stride..(y + 1) * stride];
        for x in h0..h0 + w {
            let i = (x - FIRST_H) * PER_LORES;
            hires_detail |= row[i] != row[i + 2];
            pixels.push(rgb12(row[i]));
        }
    }
    Rgb12Frame {
        left: h0 as u16,
        top: v0 as u16,
        width: w as u16,
        height: h as u16,
        pixels,
        hires_detail,
    }
}

/// 0xAARRGGBB with 8-bit channels (the 4-bit value times 17) to 0x0RGB.
fn rgb12(argb: u32) -> u16 {
    (((argb >> 20) & 0xF) << 8 | ((argb >> 12) & 0xF) << 4 | ((argb >> 4) & 0xF)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb12_takes_the_top_nibble_of_each_channel() {
        assert_eq!(rgb12(0xFF_FF_88_00), 0xF80);
        assert_eq!(rgb12(0xFF_11_22_33), 0x123);
    }

    #[test]
    fn crop_follows_the_ocs_window_rules() {
        // A tiny texture: 8 lores columns (32 entries) from h = 0x24, 4 rows. Each pixel encodes
        // its column in red and its row in blue.
        let (stride, rows) = (32, 4);
        let mut tex = vec![0xFF00_0000u32; stride * rows];
        for y in 0..rows {
            for x in 0..8 {
                for k in 0..4 {
                    tex[y * stride + x * 4 + k] =
                        0xFF00_0000 | ((x as u32) << 20) | ((y as u32) << 4);
                }
            }
        }
        // DIWSTRT V=1 H=0x26; DIWSTOP V=3 (bit 15 clear -> V8 set -> 0x103, clamped to 4 rows),
        // H=0x105 -> clamped to the texture's last column, 0x2B.
        let f = crop(&tex, stride, rows, 0x0126, 0x0305);
        assert_eq!((f.left, f.top, f.width, f.height), (0x26, 1, 6, 3));
        assert_eq!(f.pixel(0, 0), 0x201);
        assert_eq!(f.pixel(5, 2), 0x703);
        assert!(!f.hires_detail);
        tex[stride + 2 * 4 + 2] = 0xFFFF_FFFF;
        assert!(crop(&tex, stride, rows, 0x0126, 0x0305).hires_detail);
        // a window opening left of the texture starts at its first column
        assert_eq!(crop(&tex, stride, rows, 0x0102, 0x0305).left, 0x24);
        // the usual PAL window, 0x2C81..0x2CC1, is 320 x 256
        let big = vec![0u32; 1824 * 313];
        let f = crop(&big, 1824, 313, 0x2C81, 0x2CC1);
        assert_eq!((f.left, f.top, f.width, f.height), (0x81, 0x2C, 320, 256));
    }

    #[test]
    fn memory_reads_big_endian_at_chip_and_slow_addresses() {
        let mut m = AmigaMemory {
            chip: vec![0; 16],
            slow: vec![0; 16],
        };
        m.chip[4..8].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        m.slow[2..4].copy_from_slice(&[0xAB, 0xCD]);
        assert_eq!(m.long(4), Some(0x1234_5678));
        assert_eq!(m.word(SLOW_BASE + 2), Some(0xABCD));
        assert_eq!(m.byte(16), None);
        assert_eq!(m.byte(SLOW_BASE + 16), None);
        let h = m.hash();
        m.slow[15] = 1;
        assert_ne!(h, m.hash());
    }
}
