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
    /// Every new value a channel's DAC took during the frame, in order, as [`AudioTap`]s packed
    /// four `i16`s each (`channel, vpos, hpos, sample * volume`); read them with
    /// [`FrameCapture::taps`]. The core patch 0004 reports them (T10).
    pub audio: Vec<i16>,
}

/// One new value on a channel's DAC: from beam position (`vpos`, `hpos`) the channel outputs
/// `value` = sample byte times volume (0..=64), until its next tap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioTap {
    pub channel: u8,
    pub vpos: u16,
    pub hpos: u16,
    pub value: i16,
}

impl FrameCapture {
    /// The DAC taps packed in [`FrameCapture::audio`].
    pub fn taps(&self) -> impl Iterator<Item = AudioTap> + '_ {
        self.audio.chunks_exact(4).map(|t| AudioTap {
            channel: t[0] as u8,
            vpos: t[1] as u16,
            hpos: t[2] as u16,
            value: t[3],
        })
    }
}

/// An interrupt the CPU took ([`Amiga::irqs`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Irq {
    pub level: u8,
    /// It began in the next frame: the step runs on into the next frame's start.
    pub next: bool,
    pub vpos: u16,
    pub hpos: u16,
}

/// A custom-register write the CPU made ([`Amiga::cpu_writes`]): the register's offset from
/// 0xDFF000, the value, and the beam (line, DMA cycle) as it was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuWrite {
    pub reg: u16,
    pub value: u16,
    pub vpos: u16,
    pub hpos: u16,
}

/// What a [`BlitEvent`] marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlitEventKind {
    /// The blit begins (vAmiga's `Blitter::beginBlit`).
    Begin,
    /// Its busy flag clears: BLTBUSY reads 0 from here (`Blitter::clearBusyFlag`).
    Free,
    /// It ends (`Blitter::endBlit`).
    End,
}

/// A blitter event ([`Amiga::blit_events`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlitEvent {
    pub kind: BlitEventKind,
    /// It happened in the next frame: the step runs on into the next frame's start.
    pub next: bool,
    pub vpos: u16,
    pub hpos: u16,
}

/// The rate of [`Amiga::mixed_audio`].
pub const MIX_RATE: u32 = ffi::SF_AUDIO_RATE;

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

/// What the mixer's DMA reads: chip RAM only, as Agnus sees it (an address past the RAM reads 0).
impl sf_paula::ChipMemory for AmigaMemory {
    fn chip_word(&self, addr: u32) -> u16 {
        let a = addr as usize;
        match self.chip.get(a..a + 2) {
            Some(w) => u16::from_be_bytes([w[0], w[1]]),
            None => 0,
        }
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

/// The 68000's registers at a frame boundary (after [`Amiga::step`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CpuRegs {
    pub d: [u32; 8],
    /// `a[7]` is the active stack pointer (SSP in supervisor mode, else USP).
    pub a: [u32; 8],
    /// The address of the instruction being executed.
    pub pc: u32,
    pub sr: u16,
    pub usp: u32,
    pub ssp: u32,
}

impl CpuRegs {
    /// The interrupt priority mask in SR (0..7).
    pub fn ipl(&self) -> u8 {
        ((self.sr >> 8) & 7) as u8
    }

    /// Supervisor mode.
    pub fn supervisor(&self) -> bool {
        self.sr & 0x2000 != 0
    }
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

/// Where the CPU was as a raster line ended ([`Amiga::line_samples`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSample {
    pub vpos: u16,
    pub hpos: u16,
    /// The instruction last executed.
    pub pc: u32,
    pub sr: u16,
    /// The active stack pointer, and the longs from it up.
    pub sp: u32,
    pub stack: [u32; ffi::SF_STACK_LONGS],
    /// With [`Amiga::set_dma_profile`]: the line's DMA slots' owners (vAmiga's `BusOwner`).
    pub owners: [u8; 228],
}

/// One emulator instance: an A500 (OCS, 512K chip + 512K slow, no clock, RAM zeroed, PAL) with
/// the owner's Kickstart and disk 1 (or [`OracleConfig::boot`]'s) in df0. Deterministic: the same inputs from the same start give
/// the same frames (FINDINGS 2026-10-11).
pub struct Amiga {
    raw: *mut SfAmiga,
    disks: DiskSet,
    max_frames: u32,
    drives: u8,
    /// The disk last put in df0 (a restored snapshot does not change it).
    df0: DiskSlot,
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
    /// Read the ROM and disks (read-only), insert the boot disk ([`OracleConfig::boot`]) and switch on. Frame 0 is next.
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
            drives: cfg.drives,
            df0: cfg.boot,
            frame: 0,
            input: FrameInput::NONE,
        };
        for d in 1..cfg.drives {
            let rc = unsafe {
                ffi::sf_amiga_connect_drive(amiga.raw, d as i32, 1, err.ptr(), ErrBuf::LEN)
            };
            if rc != 0 {
                return Err(OracleError::Setup(err.message()));
            }
        }
        amiga
            .insert(0, cfg.boot)
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
            audio: self.taps(),
        })
    }

    /// The blits the last step started, in order: BLTSIZE as written and the beam's (line,
    /// slot) then. Always recorded (a BLTSIZE write starts a blit). Conformance anchors the
    /// engine's blits at these (`Game::anchor_blits`, T36): each of the engine's starts where the
    /// original started the next one of its size.
    pub fn blit_starts(&self) -> Vec<(u16, u16, u16)> {
        let mut p = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_blit_starts(self.raw, &mut p) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(p, n) }
            .iter()
            .map(|w| (w.value, w.vpos, w.hpos))
            .collect()
    }

    /// Sample the CPU at the end of every raster line from the next step on (or stop): test
    /// tooling that measures where the original's CPU time goes ([`Amiga::line_samples`]: the
    /// PC, the status register and the top of the stack, from which the running routine and its
    /// callers are read; the `profile` and `fit` examples of `sf-conform`, T36). The run itself
    /// is unchanged: the end-of-line trap pauses the core between two instructions and it goes
    /// on from there; no trap is set on the frame's last four lines, so a pause cannot fall
    /// between the frame's end and the vertical sync.
    pub fn set_profile(&mut self, on: bool) {
        unsafe { ffi::sf_amiga_set_profile(self.raw, on as std::os::raw::c_int) };
    }

    /// [`Amiga::set_profile`], and each line's DMA slot owners too ([`LineSample::owners`]): who
    /// had the bus in each of its slots (vAmiga's `BusOwner`: CPU, bitplane, blitter, copper,
    /// …), to measure what the display and the blitter take from the CPU on a line. The owners are
    /// the reference emulator's DMA debugger's per-line copy, a private member the shim reaches
    /// through the explicit-instantiation access trick (`shim.cpp`, `Rob`/`OwnersTag`: an
    /// explicit template instantiation may name a private member). The debugger is on meanwhile
    /// (X-ray DMA mode), so it draws over the picture: for timing studies only, never in a
    /// comparison. Stopped with `set_profile(false)`.
    pub fn set_dma_profile(&mut self) {
        unsafe { ffi::sf_amiga_set_profile(self.raw, 2) };
    }

    /// The last step's line samples, while profiling is on ([`Amiga::set_profile`]).
    pub fn line_samples(&self) -> Vec<LineSample> {
        let mut p = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_line_samples(self.raw, &mut p) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(p, n) }
            .iter()
            .map(|s| LineSample {
                vpos: s.vpos,
                hpos: s.hpos,
                pc: s.pc,
                sr: s.sr as u16,
                sp: s.sp,
                stack: s.stack,
                owners: s.owners,
            })
            .collect()
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

    /// `len` bytes of chip or slow RAM from `addr` (0 outside RAM), without copying the rest.
    pub fn peek(&self, addr: u32, len: usize) -> Vec<u8> {
        unsafe {
            let (mut c, mut s) = (0usize, 0usize);
            let chip = ffi::sf_amiga_chip(self.raw, &mut c);
            let slow = ffi::sf_amiga_slow(self.raw, &mut s);
            (0..len as u32)
                .map(|k| {
                    let a = (addr + k) as usize & 0xFF_FFFF;
                    if a < c {
                        *chip.add(a)
                    } else if a >= 0xC0_0000 && a - 0xC0_0000 < s && !slow.is_null() {
                        *slow.add(a - 0xC0_0000)
                    } else {
                        0
                    }
                })
                .collect()
        }
    }

    /// Write `bytes` into chip or slow RAM at `addr`, between frames: how a conformance case
    /// freezes what the engine does not model yet (patching the original's own code). Bytes
    /// outside RAM are dropped.
    pub fn poke(&mut self, addr: u32, bytes: &[u8]) {
        unsafe { ffi::sf_amiga_poke(self.raw, addr, bytes.as_ptr(), bytes.len()) }
    }

    /// The CPU's registers as they are now: at the end of the last frame, just before the next
    /// vertical blank's interrupt is taken.
    pub fn cpu(&self) -> CpuRegs {
        let mut r = ffi::SfCpuRegs::default();
        unsafe { ffi::sf_amiga_cpu(self.raw, &mut r) };
        CpuRegs {
            d: r.d,
            a: r.a,
            pc: r.pc,
            sr: r.sr as u16,
            usp: r.usp,
            ssp: r.ssp,
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

    /// Write `bytes` at `addr` in chip or slow RAM before the next frame. Not a player's input:
    /// only for a sync point the original offers no input route to (docs/12 "Reaching every
    /// level"). Panics unless every byte is chip or slow RAM as the CPU sees it now.
    pub fn write_memory(&mut self, addr: u32, bytes: &[u8]) {
        let mut err = ErrBuf::new();
        let rc = unsafe {
            ffi::sf_amiga_write(
                self.raw,
                addr,
                bytes.as_ptr(),
                bytes.len(),
                err.ptr(),
                ErrBuf::LEN,
            )
        };
        assert_eq!(rc, 0, "vAmiga refused a memory write: {}", err.message());
    }

    /// Swap the disk in `drive` for `slot`'s, as a player answering the game's prompt does. A
    /// drive the set-up did not connect ([`OracleConfig::drives`]) panics. The core ejects first and inserts after the
    /// drive's swap delay, as the hardware would see it.
    pub fn insert_disk(&mut self, drive: u8, slot: DiskSlot) {
        if let Err(e) = self.insert(drive, slot) {
            panic!("{e}");
        }
    }

    /// Whether disk 1 is the disk last put in df0.
    pub fn disk1_in(&self) -> bool {
        self.df0 == DiskSlot::D1
    }

    fn insert(&mut self, drive: u8, slot: DiskSlot) -> Result<(), OracleError> {
        if drive >= self.drives {
            return Err(OracleError::NoDrive(drive));
        }
        if drive == 0 {
            self.df0 = slot;
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

    /// vAmiga's own stereo mix of the last frame at [`MIX_RATE`], `[left, right]` in -1..1:
    /// channels 0/3 left and 1/2 right, no filter, nearest-sample resampling (shim.h). It fades in
    /// over the first 10 000 samples after power-on.
    pub fn mixed_audio(&self) -> Vec<[f32; 2]> {
        let mut ptr = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_audio_out(self.raw, &mut ptr) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, 2 * n) }
            .chunks_exact(2)
            .map(|p| [p[0], p[1]])
            .collect()
    }

    /// Lines in the frame the last step ran: 313 (long) or 312 (short). Superfrog's frames are not
    /// all long: Agnus alternates them while BPLCON0's LACE bit is set and keeps the last kind when
    /// it is cleared (FINDINGS 2026-10-11, T10).
    pub fn frame_lines(&self) -> u32 {
        unsafe { ffi::sf_amiga_frame_lines(self.raw) as u32 }
    }

    fn taps(&self) -> Vec<i16> {
        let mut ptr = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_audio_taps(self.raw, &mut ptr) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, n) }
            .iter()
            .flat_map(|t| {
                [
                    t.channel as i16,
                    t.vpos as i16,
                    t.hpos as i16,
                    t.sample as i16 * t.volume as i16,
                ]
            })
            .collect()
    }

    /// Every custom-register write the CPU made during the last step, in order, with the beam as
    /// each was made (the Copper's are not kept).
    pub fn cpu_writes(&self) -> Vec<CpuWrite> {
        let mut ptr = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_cpu_writes(self.raw, &mut ptr) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, n) }
            .iter()
            .map(|w| CpuWrite {
                reg: w.reg,
                value: w.value,
                vpos: w.vpos,
                hpos: w.hpos,
            })
            .collect()
    }

    /// The blitter's events during the last step, in order: each blit's begin, busy flag clearing
    /// and end, with the beam (line, DMA cycle) then (core patch 0005).
    pub fn blit_events(&self) -> Vec<BlitEvent> {
        let mut ptr = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_blit_events(self.raw, &mut ptr) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, n) }
            .iter()
            .map(|e| BlitEvent {
                kind: match e.event {
                    0 => BlitEventKind::Begin,
                    1 => BlitEventKind::Free,
                    _ => BlitEventKind::End,
                },
                next: e.next != 0,
                vpos: e.vpos,
                hpos: e.hpos,
            })
            .collect()
    }

    /// How many frames the last step ran: Agnus's frame count at its end minus at its start. A
    /// step runs until the CPU ends an instruction past the frame's end, so a blit holding the CPU
    /// can carry it into the next frame (FINDINGS 2026-10-13, T46); normally 1.
    pub fn frames_run(&self) -> u32 {
        unsafe { ffi::sf_amiga_frames_run(self.raw) as u32 }
    }

    /// From the next step on, copy chip and slow RAM at the CPU's first write of `value` to the
    /// custom register `reg` (its offset from `$DFF000`) in each step ([`Amiga::watched_memory`]);
    /// `None` stops it.
    pub fn watch_write(&mut self, write: Option<(u16, u16)>) {
        let (reg, value) = write.map_or((0xffff, 0), |(r, v)| (r as u32, v));
        unsafe { ffi::sf_amiga_watch_write(self.raw, reg, value) };
    }

    /// RAM as it was at the watched write ([`Amiga::watch_write`]) in the last step, if the CPU
    /// made it.
    pub fn watched_memory(&self) -> Option<AmigaMemory> {
        let (mut chip, mut slow) = (std::ptr::null(), std::ptr::null());
        let (mut c, mut s) = (0usize, 0usize);
        let hit =
            unsafe { ffi::sf_amiga_watched_memory(self.raw, &mut chip, &mut c, &mut slow, &mut s) };
        if hit == 0 {
            return None;
        }
        unsafe {
            Some(AmigaMemory {
                chip: std::slice::from_raw_parts(chip, c).to_vec(),
                slow: if s == 0 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(slow, s).to_vec()
                },
            })
        }
    }

    /// The interrupts the CPU took during the last step, in order: their level and the beam position
    /// (line, DMA cycle) as each exception began (core patch 0005).
    pub fn irqs(&self) -> Vec<Irq> {
        let mut ptr = std::ptr::null();
        let n = unsafe { ffi::sf_amiga_irqs(self.raw, &mut ptr) };
        if n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, n) }
            .iter()
            .map(|i| Irq {
                level: i.level,
                next: i.next != 0,
                vpos: i.vpos,
                hpos: i.hpos,
            })
            .collect()
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
