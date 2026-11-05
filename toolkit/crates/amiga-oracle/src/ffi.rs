//! The shim's C API (`shim/shim.h`), as declared there.

use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct RawAmiga {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawCustomWrite {
    pub reg: u16,
    pub value: u16,
    pub vpos: u16,
    pub hpos: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawCpuRegs {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub pc: u32,
    pub sr: u32,
    pub usp: u32,
    pub ssp: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawAudioTap {
    pub channel: u8,
    pub sample: i8,
    pub volume: u16,
    pub vpos: u16,
    pub hpos: u16,
}

/// shim.h's AMIGA_STACK_LONGS.
pub const AMIGA_STACK_LONGS: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawLineSample {
    pub vpos: u16,
    pub hpos: u16,
    pub pc: u32,
    pub sr: u32,
    pub sp: u32,
    pub stack: [u32; AMIGA_STACK_LONGS],
    pub owners: [u8; 228],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawIrq {
    pub level: u8,
    pub next: u8,
    pub vpos: u16,
    pub hpos: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawBlitEvent {
    pub event: u8,
    pub next: u8,
    pub vpos: u16,
    pub hpos: u16,
}

/// shim.h's AMIGA_AUDIO_RATE.
pub const AMIGA_AUDIO_RATE: u32 = 48_000;

unsafe extern "C" {
    pub fn amiga_new(
        rom: *const u8,
        rom_len: usize,
        err: *mut c_char,
        err_len: usize,
    ) -> *mut RawAmiga;
    pub fn amiga_free(a: *mut RawAmiga);
    pub fn amiga_insert_disk(
        a: *mut RawAmiga,
        drive: c_int,
        adf: *const u8,
        len: usize,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
    pub fn amiga_eject_disk(
        a: *mut RawAmiga,
        drive: c_int,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
    pub fn amiga_write(
        a: *mut RawAmiga,
        addr: u32,
        bytes: *const u8,
        len: usize,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
    pub fn amiga_connect_drive(
        a: *mut RawAmiga,
        drive: c_int,
        connected: c_int,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
    pub fn amiga_power_on(a: *mut RawAmiga, err: *mut c_char, err_len: usize) -> c_int;
    pub fn amiga_step(
        a: *mut RawAmiga,
        joy: u32,
        keys: *const u8,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
    pub fn amiga_paula_writes(a: *const RawAmiga, out: *mut *const RawCustomWrite) -> usize;
    pub fn amiga_audio_taps(a: *const RawAmiga, out: *mut *const RawAudioTap) -> usize;
    pub fn amiga_irqs(a: *const RawAmiga, out: *mut *const RawIrq) -> usize;
    pub fn amiga_frame_lines(a: *const RawAmiga) -> c_int;
    pub fn amiga_cpu_writes(a: *const RawAmiga, out: *mut *const RawCustomWrite) -> usize;
    pub fn amiga_blit_events(a: *const RawAmiga, out: *mut *const RawBlitEvent) -> usize;
    pub fn amiga_frames_run(a: *const RawAmiga) -> c_int;
    pub fn amiga_watch_write(a: *mut RawAmiga, reg: u32, value: u16);
    pub fn amiga_watched_memory(
        a: *const RawAmiga,
        chip: *mut *const u8,
        chip_len: *mut usize,
        slow: *mut *const u8,
        slow_len: *mut usize,
    ) -> c_int;
    pub fn amiga_audio_out(a: *const RawAmiga, out: *mut *const f32) -> usize;
    pub fn amiga_texture(a: *const RawAmiga, stride: *mut usize, rows: *mut usize) -> *const u32;
    pub fn amiga_diw(a: *const RawAmiga, diwstrt: *mut u16, diwstop: *mut u16);
    pub fn amiga_chip(a: *const RawAmiga, len: *mut usize) -> *const u8;
    pub fn amiga_slow(a: *const RawAmiga, len: *mut usize) -> *const u8;
    pub fn amiga_cpu(a: *const RawAmiga, out: *mut RawCpuRegs);
    pub fn amiga_blit_starts(a: *const RawAmiga, out: *mut *const RawCustomWrite) -> usize;
    pub fn amiga_set_profile(a: *mut RawAmiga, on: c_int);
    pub fn amiga_watch_pcs(a: *mut RawAmiga, pcs: *const u32, n: usize);
    pub fn amiga_pc_hits(a: *const RawAmiga, pcs: *mut *const u32, counts: *mut *const u32) -> usize;
    pub fn amiga_line_samples(a: *const RawAmiga, out: *mut *const RawLineSample) -> usize;
    pub fn amiga_poke(a: *mut RawAmiga, addr: u32, bytes: *const u8, len: usize);
    pub fn amiga_snapshot(
        a: *mut RawAmiga,
        len: *mut usize,
        err: *mut c_char,
        err_len: usize,
    ) -> *mut u8;
    pub fn amiga_buffer_free(buf: *mut u8);
    pub fn amiga_restore(
        a: *mut RawAmiga,
        snap: *const u8,
        len: usize,
        joy: u32,
        keys: *const u8,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
}

/// A buffer for the shim's error message, and the message once it is filled.
pub struct ErrBuf([c_char; 512]);

impl ErrBuf {
    pub fn new() -> ErrBuf {
        ErrBuf([0; 512])
    }

    pub fn ptr(&mut self) -> *mut c_char {
        self.0.as_mut_ptr()
    }

    pub const LEN: usize = 512;

    pub fn message(&self) -> String {
        let bytes: Vec<u8> = self
            .0
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

// The routine runner (`shim/moira_shim.h`).

#[repr(C)]
pub struct RawCpu {
    _private: [u8; 0],
}

pub const AMIGA_CPU68K_MEM_SIZE: usize = 0x100_0000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawRegs {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub sr: u16,
}

pub const AMIGA_CALL_RETURNED: u32 = 0;
pub const AMIGA_CALL_TIMEOUT: u32 = 1;
pub const AMIGA_CALL_CUSTOM: u32 = 2;
pub const AMIGA_CALL_EXCEPTION: u32 = 3;
pub const AMIGA_CALL_HALTED: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawCallStatus {
    pub status: u32,
    pub pc: u32,
    pub addr: u32,
    pub vector: u16,
    pub instr: u64,
    pub cycles: u64,
}

unsafe extern "C" {
    pub fn amiga_cpu68k_new() -> *mut RawCpu;
    pub fn amiga_cpu68k_free(c: *mut RawCpu);
    pub fn amiga_cpu68k_mem(c: *mut RawCpu) -> *mut u8;
    pub fn amiga_cpu68k_call(
        c: *mut RawCpu,
        entry: u32,
        regs: *mut RawRegs,
        sentinel: u32,
        max_instr: u64,
        out: *mut RawCallStatus,
    );
    pub fn amiga_cpu68k_journal(c: *const RawCpu, addrs: *mut *const u32, olds: *mut *const u8) -> usize;
    pub fn amiga_cpu68k_rollback(c: *mut RawCpu);
    pub fn amiga_cpu68k_set_hw(c: *mut RawCpu, on: c_int);
    pub fn amiga_cpu68k_set_hw_read(c: *mut RawCpu, addr: u32, value: u8);
    pub fn amiga_cpu68k_hw_writes(c: *const RawCpu, out: *mut *const RawHwWrite) -> usize;
    pub fn amiga_cpu68k_set_trace(c: *mut RawCpu, on: c_int);
    pub fn amiga_cpu68k_branches(c: *const RawCpu, out: *mut *const RawBranch) -> usize;
    pub fn amiga_cpu68k_reads(c: *const RawCpu, out: *mut *const u32) -> usize;
    pub fn amiga_cpu68k_insn_len(c: *const RawCpu, addr: u32) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawBranch {
    pub at: u32,
    pub not_taken: u8,
    pub taken: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawHwWrite {
    pub reg: u16,
    pub value: u16,
}
