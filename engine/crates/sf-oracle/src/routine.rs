//! The routine oracle (DESIGN §4, routine conformance): one routine of the original, run alone on
//! Moira (vAmiga's 68000 core) over a flat 24-bit memory with no chipset.
//!
//! A test fills a [`Cpu68k`]'s memory (from [`crate::Amiga::memory`] at a sync point, an executable
//! loaded with [`Cpu68k::load_exe`], its own bytes), then [`Cpu68k::call`]s a routine with chosen
//! registers and gets back the registers and every byte the routine changed. Each call starts from
//! the same memory: what a call writes is undone before it returns.
//!
//! The routine returns to [`RETURN_SENTINEL`], pushed as its return address: on an A500 nothing is
//! mapped there, so no routine has a reason to touch it.

use std::fmt;
use std::ptr::NonNull;

use crate::amiga::{AmigaMemory, SLOW_BASE};
use crate::ffi::{self, SfCallStatus, SfCpu, SfRegs};

/// The return address [`Cpu68k::call`] pushes; the call ends when the PC reaches it.
pub const RETURN_SENTINEL: u32 = 0xF0_0000;

/// Bytes of memory: every 24-bit address.
pub const MEM_SIZE: usize = ffi::SF_CPU_MEM_SIZE;

/// The 68000's registers a routine sees and leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Regs {
    pub d: [u32; 8],
    /// `a[7]` is the stack pointer `sr` selects (supervisor or user).
    pub a: [u32; 8],
    pub sr: u16,
}

impl Default for Regs {
    /// All zero, in supervisor mode with interrupts masked (`sr = 0x2700`), as the game runs.
    fn default() -> Regs {
        Regs {
            d: [0; 8],
            a: [0; 8],
            sr: 0x2700,
        }
    }
}

/// What a routine did.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallResult {
    /// The registers when it returned (`a[7]` is back above the return address).
    pub regs: Regs,
    /// Every byte it changed, by address: `(addr, old, new)`. A byte written with the value it
    /// already held is not here, nor is the return address the call pushed.
    pub writes: Vec<(u32, u8, u8)>,
    /// Instructions executed, its final `rts` included.
    pub instr: u64,
    /// CPU cycles it took on Moira's clock (from 0 at the call), its final `rts` included. Moira's
    /// own timing, with no bus contention: test tooling for calibrating the engine's CPU time.
    pub cycles: u64,
    /// In hardware mode ([`Cpu68k::set_hardware`]): every custom-chip write, in order, as
    /// `(offset from $DFF000, word)`. Empty otherwise.
    pub hw_writes: Vec<(u16, u16)>,
}

impl CallResult {
    /// `len` bytes at `addr` as the routine left them: `cpu`'s memory with [`Self::writes`] applied.
    pub fn bytes_after(&self, cpu: &Cpu68k, addr: u32, len: usize) -> Vec<u8> {
        let mut out = cpu.read(addr, len).to_vec();
        let start = self.writes.partition_point(|w| w.0 < addr);
        for &(a, _, new) in &self.writes[start..] {
            if (a - addr) as usize >= len {
                break;
            }
            out[(a - addr) as usize] = new;
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallError {
    /// `max_instr` instructions ran and the routine had not returned.
    Timeout,
    /// The instruction at `pc` read or wrote `$DFF000`–`$DFF1FF` or `$BFxxxx`. Such a routine
    /// belongs to the full-system oracle ([`crate::Amiga`]). The write was not performed.
    CustomChipAccess { pc: u32, addr: u32 },
    /// The instruction at `pc` is illegal (vector 4) or an unimplemented A/F-line opcode (10, 11).
    IllegalInstruction { pc: u32 },
    /// Any other exception: address or bus error (2, 3), division by zero (5), `chk`, `trapv`,
    /// privilege violation, trace, `trap #n` (32–47). The game installs its own handlers, which
    /// this runner does not model.
    Exception { pc: u32, vector: u16 },
    /// The CPU halted (a fault while taking an exception).
    Halted { pc: u32 },
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            CallError::Timeout => write!(
                f,
                "the routine did not return within max_instr instructions"
            ),
            CallError::CustomChipAccess { pc, addr } => write!(
                f,
                "the instruction at ${pc:06x} touches hardware register ${addr:06x}: run this routine on \
                 the full-system oracle (sf_oracle::Amiga)"
            ),
            CallError::IllegalInstruction { pc } => write!(f, "illegal instruction at ${pc:06x}"),
            CallError::Exception { pc, vector } => {
                write!(f, "exception vector {vector} taken at ${pc:06x}")
            }
            CallError::Halted { pc } => write!(f, "the CPU halted at ${pc:06x}"),
        }
    }
}

impl std::error::Error for CallError {}

/// What kind of hunk the loader filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HunkKind {
    Code,
    Data,
    Bss,
    /// Declared in the header but not in the file: room is reserved, nothing is written
    /// (docs/RE-WORKBENCH.md, `d1_c03` and `d3_c04`).
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hunk {
    pub base: u32,
    /// The size the header declares, in bytes.
    pub size: u32,
    pub kind: HunkKind,
}

/// Where [`Cpu68k::load_exe`] put each hunk.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HunkMap {
    pub hunks: Vec<Hunk>,
}

impl HunkMap {
    /// The address of byte `offset` of hunk `hunk`: how `symbols.toml`'s `hN+$off` becomes an address.
    pub fn addr(&self, hunk: usize, offset: u32) -> u32 {
        self.hunks[hunk].base + offset
    }

    /// The first address after the last hunk.
    pub fn end(&self) -> u32 {
        self.hunks.last().map_or(0, |h| h.base + h.size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HunkError {
    /// The file does not start with `HUNK_HEADER` (0x3F3).
    NotAnExecutable,
    /// The file ends inside a block.
    Truncated { at: usize },
    /// A block the game's loader would run wild on: an unknown type, a zero or oversized length, a
    /// relocation to a hunk that does not exist or outside its hunk.
    Malformed { at: usize, why: &'static str },
    /// The hunks do not fit below 16 MB from the load address.
    TooBig,
}

impl fmt::Display for HunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HunkError::NotAnExecutable => write!(f, "not a hunk executable (no 0x3F3 header)"),
            HunkError::Truncated { at } => write!(f, "hunk executable cut short at byte {at}"),
            HunkError::Malformed { at, why } => write!(f, "hunk executable byte {at}: {why}"),
            HunkError::TooBig => write!(f, "hunk executable does not fit in 24-bit memory"),
        }
    }
}

impl std::error::Error for HunkError {}

/// Moira with flat 24-bit memory (16 MB, zeroed, everything RAM except the custom chips and CIAs).
pub struct Cpu68k {
    raw: NonNull<SfCpu>,
}

// The shim object is used from one thread at a time, through &mut self.
unsafe impl Send for Cpu68k {}

impl Drop for Cpu68k {
    fn drop(&mut self) {
        unsafe { ffi::sf_cpu_free(self.raw.as_ptr()) }
    }
}

impl Cpu68k {
    /// A CPU whose memory holds `mem`'s chip RAM at 0 and slow RAM at `$C00000`; the rest is zero.
    pub fn new(mem: AmigaMemory) -> Cpu68k {
        let raw = NonNull::new(unsafe { ffi::sf_cpu_new() }).expect("allocating the routine CPU");
        let mut cpu = Cpu68k { raw };
        cpu.write(0, &mem.chip);
        cpu.write(SLOW_BASE, &mem.slow);
        cpu
    }

    /// The whole memory, `MEM_SIZE` bytes.
    pub fn memory(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(ffi::sf_cpu_mem(self.raw.as_ptr()), MEM_SIZE) }
    }

    pub fn memory_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(ffi::sf_cpu_mem(self.raw.as_ptr()), MEM_SIZE) }
    }

    /// `len` bytes at `addr`. Panics past 16 MB.
    pub fn read(&self, addr: u32, len: usize) -> &[u8] {
        &self.memory()[addr as usize..addr as usize + len]
    }

    /// Put `bytes` at `addr`. Panics past 16 MB.
    pub fn write(&mut self, addr: u32, bytes: &[u8]) {
        self.memory_mut()[addr as usize..addr as usize + bytes.len()].copy_from_slice(bytes);
    }

    /// Load a hunk executable at `at` as the game's own loader does (`hunk_load`, stage 1.5
    /// `+0x43A`): hunks back to back from `at` at their declared sizes, `HUNK_CODE`/`HUNK_DATA`
    /// copied and the rest of the hunk cleared, `HUNK_BSS` cleared for its own length,
    /// `HUNK_RELOC32` applied by adding the target hunk's base. Loading ends after one `HUNK_END`
    /// per declared hunk, at a 0x3ED–0x3F1 block, or at the end of the file (where the game's
    /// loader reads on past it; T07).
    pub fn load_exe(&mut self, exe: &[u8], at: u32) -> Result<HunkMap, HunkError> {
        let mut r = Reader { exe, pos: 0 };
        if r.long()? != 0x3F3 {
            return Err(HunkError::NotAnExecutable);
        }
        loop {
            let n = r.long()?;
            if n == 0 {
                break;
            }
            r.skip(n)?;
        }
        let count = r.long()? as usize;
        if count == 0 || count > 0x10000 {
            return Err(HunkError::Malformed {
                at: r.pos - 4,
                why: "hunk count",
            });
        }
        r.skip(2)?;
        let mut hunks = Vec::with_capacity(count);
        let mut base = at as u64;
        for _ in 0..count {
            // The loader multiplies the whole long by four, so the memory flags fall off the top.
            let size = r.long()?.wrapping_mul(4);
            hunks.push(Hunk {
                base: base as u32,
                size,
                kind: HunkKind::Absent,
            });
            base += size as u64;
        }
        if base > MEM_SIZE as u64 {
            return Err(HunkError::TooBig);
        }

        let mut cur = 0;
        while r.pos < exe.len() {
            let at = r.pos;
            match r.long()? {
                // HUNK_UNIT, HUNK_NAME: skipped.
                0x3E7 | 0x3E8 => {
                    let n = r.long()?;
                    r.skip(n)?;
                }
                kind @ (0x3E9 | 0x3EA) => {
                    let h = hunk(&mut hunks, cur, at)?;
                    let n = r.long()?;
                    if n == 0 || n as u64 * 4 > h.size as u64 {
                        return Err(HunkError::Malformed {
                            at,
                            why: "code/data length",
                        });
                    }
                    let body = r.bytes(n as usize * 4)?;
                    let (b, size) = (h.base, h.size);
                    h.kind = if kind == 0x3E9 {
                        HunkKind::Code
                    } else {
                        HunkKind::Data
                    };
                    self.write(b, body);
                    self.memory_mut()[(b + n * 4) as usize..(b + size) as usize].fill(0);
                }
                0x3EB => {
                    let h = hunk(&mut hunks, cur, at)?;
                    let n = r.long()?;
                    if n == 0 || h.base as u64 + n as u64 * 4 > MEM_SIZE as u64 {
                        return Err(HunkError::Malformed {
                            at,
                            why: "bss length",
                        });
                    }
                    h.kind = HunkKind::Bss;
                    let b = h.base as usize;
                    self.memory_mut()[b..b + n as usize * 4].fill(0);
                }
                0x3EC => {
                    let h = *hunk(&mut hunks, cur, at)?;
                    loop {
                        let n = r.long()?;
                        if n == 0 {
                            break;
                        }
                        let target = r.long()? as usize;
                        let delta = hunks
                            .get(target)
                            .ok_or(HunkError::Malformed {
                                at,
                                why: "relocation to a missing hunk",
                            })?
                            .base;
                        for _ in 0..n {
                            let off = r.long()?;
                            if off as u64 + 4 > h.size as u64 {
                                return Err(HunkError::Malformed {
                                    at,
                                    why: "relocation outside its hunk",
                                });
                            }
                            let a = (h.base + off) as usize;
                            let m = self.memory_mut();
                            let v = u32::from_be_bytes([m[a], m[a + 1], m[a + 2], m[a + 3]]);
                            m[a..a + 4].copy_from_slice(&v.wrapping_add(delta).to_be_bytes());
                        }
                    }
                }
                // HUNK_RELOC16, RELOC8, EXT, SYMBOL, DEBUG: the game's loader stops here.
                0x3ED..=0x3F1 => break,
                0x3F2 => {
                    cur += 1;
                    if cur == count {
                        break;
                    }
                }
                _ => {
                    return Err(HunkError::Malformed {
                        at,
                        why: "unknown block type",
                    });
                }
            }
        }
        Ok(HunkMap { hunks })
    }

    /// Hardware mode, off by default: for routines that drive the custom chips themselves (the
    /// sound driver, T11; the level's blits, T14). A custom-chip write is recorded in
    /// [`CallResult::hw_writes`] instead of ending the call with [`CallError::CustomChipAccess`]; a
    /// byte write is recorded as that byte on both halves. A write to BLTSIZE runs the blit there
    /// and then, in area mode (channels, shifts, masks, minterm, descending; no line mode or fill),
    /// its writes journalled with the CPU's; the blitter's registers start each call at 0. A CPU
    /// word read of a write-only custom register writes the data bus's last word into it, as the
    /// full-system reference does (vAmiga `peekCustomFaulty16`): `clr.w` of a volume register
    /// writes the next instruction's opcode, then 0. VHPOSR (`$DFF006`) steps on every read so raster waits end; other custom
    /// registers and the CIAs read 0, and CIA writes are dropped. Nothing else of the chipset is
    /// modelled: a routine that waits on DMA or an interrupt still times out.
    pub fn set_hardware(&mut self, on: bool) {
        unsafe { ffi::sf_cpu_set_hw(self.raw.as_ptr(), on as i32) }
    }

    /// In hardware mode, make a read of the byte at `addr` (a custom register or a CIA's) return
    /// `value` from now on: the joystick (`JOY1DAT`, `$DFF00C`), the fire buttons (CIA-A PRA,
    /// `$BFE001`). VHPOSR keeps stepping; every other hardware byte reads 0.
    pub fn set_hardware_read(&mut self, addr: u32, value: u8) {
        unsafe { ffi::sf_cpu_set_hw_read(self.raw.as_ptr(), addr, value) }
    }

    /// Put a call's memory changes back: what [`Cpu68k::call`] undid, for a caller that runs a
    /// routine many times over and wants each run to see the last one's effects. Writes for which
    /// `keep(addr)` is false (the call's own stack, say) are left out.
    pub fn apply(&mut self, result: &CallResult, keep: impl Fn(u32) -> bool) {
        for &(a, _, new) in &result.writes {
            if keep(a) {
                self.memory_mut()[a as usize] = new;
            }
        }
    }

    /// Run the routine at `entry` with `regs` until it returns, at most `max_instr` instructions.
    ///
    /// The CPU is reset, [`RETURN_SENTINEL`] is pushed below `regs.a[7]`, and the routine starts
    /// there. Memory is put back as it was before the call returns, whatever the outcome.
    pub fn call(
        &mut self,
        entry: u32,
        regs: Regs,
        max_instr: u64,
    ) -> Result<CallResult, CallError> {
        // The CPU gets all 32 bits of the stack pointer; memory sees the low 24, as the bus does.
        let sp_reg = regs.a[7].wrapping_sub(4);
        let sp = sp_reg & (MEM_SIZE as u32 - 1);
        assert!(
            sp as usize + 4 <= MEM_SIZE,
            "the stack pointer must leave room below it for the return address"
        );
        let saved: [u8; 4] = self.read(sp, 4).try_into().unwrap();
        self.write(sp, &RETURN_SENTINEL.to_be_bytes());

        let mut r = SfRegs {
            d: regs.d,
            a: regs.a,
            sr: regs.sr,
        };
        r.a[7] = sp_reg;
        let mut st = SfCallStatus::default();
        unsafe {
            ffi::sf_cpu_call(
                self.raw.as_ptr(),
                entry,
                &mut r,
                RETURN_SENTINEL,
                max_instr,
                &mut st,
            )
        };

        let mut writes = Vec::new();
        let mut hw_writes = Vec::new();
        if st.status == ffi::SF_CALL_RETURNED {
            let mut hw = std::ptr::null();
            let n = unsafe { ffi::sf_cpu_hw_writes(self.raw.as_ptr(), &mut hw) };
            if n > 0 {
                hw_writes = unsafe { std::slice::from_raw_parts(hw, n) }
                    .iter()
                    .map(|w| (w.reg, w.value))
                    .collect();
            }
            let (mut addrs, mut olds) = (std::ptr::null(), std::ptr::null());
            let n = unsafe { ffi::sf_cpu_journal(self.raw.as_ptr(), &mut addrs, &mut olds) };
            if n > 0 {
                let (addrs, olds) = unsafe {
                    (
                        std::slice::from_raw_parts(addrs, n),
                        std::slice::from_raw_parts(olds, n),
                    )
                };
                let mem = self.memory();
                writes = addrs
                    .iter()
                    .zip(olds)
                    .filter(|&(&a, &old)| mem[a as usize] != old)
                    .map(|(&a, &old)| (a, old, mem[a as usize]))
                    .collect();
                writes.sort_unstable_by_key(|w| w.0);
            }
        }
        unsafe { ffi::sf_cpu_rollback(self.raw.as_ptr()) };
        self.write(sp, &saved);

        match st.status {
            ffi::SF_CALL_RETURNED => Ok(CallResult {
                regs: Regs {
                    d: r.d,
                    a: r.a,
                    sr: r.sr,
                },
                writes,
                instr: st.instr,
                cycles: st.cycles,
                hw_writes,
            }),
            ffi::SF_CALL_TIMEOUT => Err(CallError::Timeout),
            ffi::SF_CALL_CUSTOM => Err(CallError::CustomChipAccess {
                pc: st.pc,
                addr: st.addr,
            }),
            ffi::SF_CALL_EXCEPTION if matches!(st.vector, 4 | 10 | 11) => {
                Err(CallError::IllegalInstruction { pc: st.pc })
            }
            ffi::SF_CALL_EXCEPTION => Err(CallError::Exception {
                pc: st.pc,
                vector: st.vector,
            }),
            ffi::SF_CALL_HALTED => Err(CallError::Halted { pc: st.pc }),
            other => unreachable!("moira_shim returned status {other}"),
        }
    }
}

fn hunk(hunks: &mut [Hunk], cur: usize, at: usize) -> Result<&mut Hunk, HunkError> {
    hunks.get_mut(cur).ok_or(HunkError::Malformed {
        at,
        why: "more hunks than the header declares",
    })
}

struct Reader<'a> {
    exe: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], HunkError> {
        let s = self
            .exe
            .get(self.pos..self.pos.saturating_add(n))
            .ok_or(HunkError::Truncated { at: self.pos })?;
        self.pos += n;
        Ok(s)
    }

    fn long(&mut self) -> Result<u32, HunkError> {
        Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn skip(&mut self, longs: u32) -> Result<(), HunkError> {
        self.bytes(longs as usize * 4).map(|_| ())
    }
}
