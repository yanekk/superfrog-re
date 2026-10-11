//! The shim's C API (`shim/shim.h`), as declared there.

use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct SfAmiga {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SfCustomWrite {
    pub reg: u16,
    pub value: u16,
    pub vpos: u16,
    pub hpos: u16,
}

unsafe extern "C" {
    pub fn sf_amiga_new(
        rom: *const u8,
        rom_len: usize,
        err: *mut c_char,
        err_len: usize,
    ) -> *mut SfAmiga;
    pub fn sf_amiga_free(a: *mut SfAmiga);
    pub fn sf_amiga_insert_disk(
        a: *mut SfAmiga,
        drive: c_int,
        adf: *const u8,
        len: usize,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
    pub fn sf_amiga_power_on(a: *mut SfAmiga, err: *mut c_char, err_len: usize) -> c_int;
    pub fn sf_amiga_step(
        a: *mut SfAmiga,
        joy: u32,
        keys: *const u8,
        err: *mut c_char,
        err_len: usize,
    ) -> c_int;
    pub fn sf_amiga_paula_writes(a: *const SfAmiga, out: *mut *const SfCustomWrite) -> usize;
    pub fn sf_amiga_texture(a: *const SfAmiga, stride: *mut usize, rows: *mut usize) -> *const u32;
    pub fn sf_amiga_diw(a: *const SfAmiga, diwstrt: *mut u16, diwstop: *mut u16);
    pub fn sf_amiga_chip(a: *const SfAmiga, len: *mut usize) -> *const u8;
    pub fn sf_amiga_slow(a: *const SfAmiga, len: *mut usize) -> *const u8;
    pub fn sf_amiga_snapshot(
        a: *mut SfAmiga,
        len: *mut usize,
        err: *mut c_char,
        err_len: usize,
    ) -> *mut u8;
    pub fn sf_buffer_free(buf: *mut u8);
    pub fn sf_amiga_restore(
        a: *mut SfAmiga,
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
pub struct SfCpu {
    _private: [u8; 0],
}

pub const SF_CPU_MEM_SIZE: usize = 0x100_0000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SfRegs {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub sr: u16,
}

pub const SF_CALL_RETURNED: u32 = 0;
pub const SF_CALL_TIMEOUT: u32 = 1;
pub const SF_CALL_CUSTOM: u32 = 2;
pub const SF_CALL_EXCEPTION: u32 = 3;
pub const SF_CALL_HALTED: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SfCallStatus {
    pub status: u32,
    pub pc: u32,
    pub addr: u32,
    pub vector: u16,
    pub instr: u64,
}

unsafe extern "C" {
    pub fn sf_cpu_new() -> *mut SfCpu;
    pub fn sf_cpu_free(c: *mut SfCpu);
    pub fn sf_cpu_mem(c: *mut SfCpu) -> *mut u8;
    pub fn sf_cpu_call(
        c: *mut SfCpu,
        entry: u32,
        regs: *mut SfRegs,
        sentinel: u32,
        max_instr: u64,
        out: *mut SfCallStatus,
    );
    pub fn sf_cpu_journal(c: *const SfCpu, addrs: *mut *const u32, olds: *mut *const u8) -> usize;
    pub fn sf_cpu_rollback(c: *mut SfCpu);
}
