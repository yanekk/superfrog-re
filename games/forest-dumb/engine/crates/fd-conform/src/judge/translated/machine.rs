//! A judged state on `pcode-machine` (translator T27, DESIGN §2.8, §4): the translated entry runs on
//! the machine the app runs, so its blits, beam waits and custom-register reads come from the
//! machine's models rather than [`super::Mem`]'s hand copy of the routine oracle's hardware mode.
//!
//! How a [`Setup`] is laid on the machine:
//!
//! - **Memory:** `Machine::from_snapshot` of the rig's memory (program, generator base, the state's
//!   pokes) as the A500 has it: chip RAM from 0, slow RAM from `$C00000`, 512 KB each.
//! - **Everything else is the oracle's flat memory** ([`OnMachine`]): the machine gets chip RAM
//!   (not its mirrors), slow RAM and the custom registers at `$DFF000`; the CIAs' `$BFxxxx` are the
//!   oracle's (below); any other address is a byte of the rig's flat
//!   16 MB ([`super::Mem`]), as the original saw it. The rig's [`STACK`] is there, so a7 and every
//!   byte pushed are the original's. The reason: generators give random pointers, which the
//!   original follows into flat memory and the A500 would follow into nothing, a chip RAM mirror
//!   or a custom register mirror; those are the rig's memory, not a hardware model.
//! - **Hardware reads the case gives** (`Setup::hw_reads`) are answered as the routine oracle answers
//!   them: a custom register's word from its two bytes (a byte not given reads 0), by a device
//!   attached to that register ([`CaseReads`]). INTENAR and
//!   INTREQR, which no device may answer, are the snapshot's INTENA and INTREQ. VHPOSR given by a
//!   case is ignored, as the oracle ignores it.
//! - **The beam:** at line `Setup::beam`, slot 0, at cycle 0, so the first VHPOSR read gives the
//!   byte the original's first read gave; later reads follow the counted cycles, where the oracle's
//!   count one a read.
//! - **DMA:** [`DMACON`], the blitter alone: the routine oracle runs no copper, display or sound.
//!   Interrupts are all disabled, as the oracle takes none.
//! - **The CIAs are the oracle's:** a byte read in `$BF0000..$BFFFFF` gives what the case gives,
//!   else 0, and a write there is dropped, as the routine oracle's hardware mode does. The machine's
//!   CIA-A model is not used: the oracle has none to judge it against (T25 and T26 check it on whole
//!   runs), a generated pointer into `$BFxxxx` would otherwise reach its timers, which stop the run,
//!   and a byte no CIA answers reads the data bus on the machine, 0 on the oracle.
//!
//! Compared as on [`super::Mem`]: the registers, every RAM byte that differs from the snapshot
//! (slow RAM at its `$C00000` address), and the custom-register writes in order, the CPU's and any
//! the copper made.

use std::sync::Arc;

use amiga_oracle::{RETURN_SENTINEL, Regs};
use pcode_machine::custom;
use pcode_machine::{Config, Device, Machine};
use pcode_rt::{Bus, Image, Stop};

use super::Mem;
use super::{Entry, Stops, fail, on_run_stack};
use crate::HUNK_BASES;
use crate::routine::{Effects, STACK, Setup};

/// The A500's chip RAM, and slow RAM at [`SLOW_BASE`] (`Config::a500`).
const CHIP: usize = 0x8_0000;
const SLOW_BASE: usize = 0xC0_0000;
const SLOW: usize = 0x8_0000;

/// DMACON on the judge's machine: DMAEN and BLTEN. The blitter runs, as the oracle's does at a
/// BLTSIZE write; copper, bitplane, sprite and audio DMA stay off, as the oracle has none of them.
pub const DMACON: u16 = 0x0240;

/// Chip RAM then slow RAM, as `Machine::from_snapshot` takes them, cut from the rig's flat memory.
pub fn ram(flat: &[u8]) -> Arc<Vec<u8>> {
    let mut m = Vec::with_capacity(CHIP + SLOW);
    m.extend_from_slice(&flat[..CHIP]);
    m.extend_from_slice(&flat[SLOW_BASE..SLOW_BASE + SLOW]);
    Arc::new(m)
}

/// Where flat address `a` is in [`ram`]'s image, if the A500 has RAM there.
fn ram_index(a: u32) -> Option<usize> {
    let a = (a & 0xFF_FFFF) as usize;
    if a < CHIP {
        Some(a)
    } else if (SLOW_BASE..SLOW_BASE + SLOW).contains(&a) {
        Some(CHIP + a - SLOW_BASE)
    } else {
        None
    }
}

/// The flat address of [`ram`]'s byte `i`.
fn flat_address(i: usize) -> u32 {
    if i < CHIP {
        i as u32
    } else {
        (SLOW_BASE + i - CHIP) as u32
    }
}

/// The hardware reads a case gives, as the routine oracle answers them: a register's word is its two
/// bytes, a byte not given reading 0. Attached to exactly the registers given.
#[derive(Clone, Default)]
pub struct CaseReads {
    words: Vec<(u16, u16)>,
}

impl Device for CaseReads {
    fn read(&mut self, reg: u16, _: u64) -> u16 {
        self.words
            .iter()
            .find(|&&(r, _)| r == reg)
            .map_or(0, |&(_, w)| w)
    }
    // The written word is the machine's (it keeps it and logs it); the case only answers reads.
    fn write(&mut self, _: u16, _: u16, _: u64) {}
}

impl CaseReads {
    fn set(&mut self, reg: u16, byte_hi: bool, v: u8) {
        let w = match self.words.iter_mut().find(|(r, _)| *r == reg) {
            Some((_, w)) => w,
            None => {
                self.words.push((reg, 0));
                &mut self.words.last_mut().expect("pushed").1
            }
        };
        *w = if byte_hi {
            *w & 0x00FF | (v as u16) << 8
        } else {
            *w & 0xFF00 | v as u16
        };
    }
}

/// The custom registers' snapshot, the device `s.hw_reads` asks for, and the CIA bytes it gives.
fn lay_hardware(s: &Setup) -> ([u16; 256], CaseReads, Vec<(u32, u8)>) {
    let mut regs = [0u16; 256];
    regs[custom::DMACONR as usize >> 1] = DMACON;
    // VPOSR (LOF, a long frame) and VHPOSR: line `beam`, slot 0, at cycle 0.
    regs[custom::VPOSR as usize >> 1] = 0x8000;
    regs[custom::VHPOSR as usize >> 1] = (s.beam as u16) << 8;
    let (mut reads, mut cias) = (CaseReads::default(), Vec::new());
    for &(a, v) in &s.hw_reads {
        let a = a & 0xFF_FFFF;
        if a & 0xFF_FE00 == 0xDF_F000 {
            let reg = (a & 0x1FE) as u16;
            let hi = a & 1 == 0;
            match reg {
                // The oracle's VHPOSR is its counter whatever the case sets.
                custom::VPOSR | custom::VHPOSR => {}
                // No device answers these: the snapshot's INTENA and INTREQ are what they read.
                0x01C | 0x01E => {
                    let w = &mut regs[reg as usize >> 1];
                    *w = if hi {
                        *w & 0x00FF | (v as u16) << 8
                    } else {
                        *w & 0xFF00 | v as u16
                    };
                }
                _ => reads.set(reg, hi, v),
            }
        } else if is_cia(a) {
            // A later byte for the same address wins, as in the oracle's map.
            cias.retain(|&(at, _)| at != a);
            cias.push((a, v));
        }
    }
    (regs, reads, cias)
}

/// The machine, and the rig's flat memory for every address outside the hardware windows the
/// oracle has and outside the A500's RAM.
struct OnMachine {
    m: Machine,
    flat: Mem,
    /// The CIA bytes the case gives, by address.
    cia: Vec<(u32, u8)>,
}

/// Whether the machine answers `a`: chip RAM below its mirrors, slow RAM and the custom registers'
/// `$DFF000` window.
fn on_machine(a: u32) -> bool {
    let a = (a & 0xFF_FFFF) as usize;
    a < CHIP || (SLOW_BASE..SLOW_BASE + SLOW).contains(&a) || a & 0xFF_FE00 == 0xDF_F000
}

/// The oracle's CIA window, `$BF0000..$BFFFFF`.
fn is_cia(a: u32) -> bool {
    a & 0xFF_0000 == 0xBF_0000
}

impl Bus for OnMachine {
    fn poll(&mut self, r: &mut pcode_rt::Regs, img: &Image) -> Result<(), Stop> {
        self.m.poll(r, img)
    }
    fn insn(&mut self, r: &mut pcode_rt::Regs, img: &Image) -> Result<(), Stop> {
        self.m.insn(r, img)
    }
    fn call_outside(&mut self, a: u32, r: &mut pcode_rt::Regs, img: &Image) -> Result<(), Stop> {
        self.m.call_outside(a, r, img)
    }
    fn sync(&mut self, r: &pcode_rt::Regs) {
        self.m.sync(r)
    }
    fn rb(&mut self, a: u32) -> u8 {
        let a = a & 0xFF_FFFF;
        if on_machine(a) {
            self.m.rb(a)
        } else if is_cia(a) {
            self.cia
                .iter()
                .find(|&&(at, _)| at == a)
                .map_or(0, |&(_, v)| v)
        } else {
            self.flat.ram(a)
        }
    }
    fn rw(&mut self, a: u32) -> u16 {
        if on_machine(a) {
            self.m.rw(a)
        } else {
            (self.rb(a) as u16) << 8 | self.rb(a.wrapping_add(1)) as u16
        }
    }
    fn rl(&mut self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a.wrapping_add(2)) as u32
    }
    fn wb(&mut self, a: u32, v: u8) {
        let a = a & 0xFF_FFFF;
        if on_machine(a) {
            self.m.wb(a, v)
        } else if !is_cia(a) {
            self.flat.wb(a, v)
        }
    }
    fn ww(&mut self, a: u32, v: u16) {
        if on_machine(a) {
            self.m.ww(a, v)
        } else {
            self.wb(a, (v >> 8) as u8);
            self.wb(a.wrapping_add(1), v as u8);
        }
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a.wrapping_add(2), v as u16);
    }
}

/// The machine a state starts on: `ram` with `s`'s pokes, the hardware [`lay_hardware`] lays.
fn machine_for(ram: &[u8], s: &Setup) -> (Machine, Vec<(u32, u8)>) {
    let mut mem = ram.to_vec();
    for (a, bytes) in &s.pokes {
        for (k, &b) in bytes.iter().enumerate() {
            if let Some(i) = ram_index(a.wrapping_add(k as u32)) {
                mem[i] = b;
            }
        }
    }
    let (regs, reads, cias) = lay_hardware(s);
    let mut m = Machine::from_snapshot(Config::a500(HUNK_BASES.to_vec()), &mem, &regs);
    // The machine puts its ExecBase at $4 (`os`); the original's memory holds the rig's bytes there,
    // and a routine indexing low memory must read what the original read.
    m.chip_mut()[..8].copy_from_slice(&mem[..8]);
    let given: Vec<u16> = reads.words.iter().map(|&(r, _)| r).collect();
    for reg in given {
        m.attach(reg..reg + 2, Box::new(reads.clone()));
    }
    (m, cias)
}

/// Run `run` from state `s` on the machine, as [`super::check_on`] runs it on [`super::Mem`].
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    ram: &[u8],
    flat: &Arc<Vec<u8>>,
    s: &Setup,
    run: Option<Entry>,
    at: &str,
    img: &Image,
    stops: &mut Stops,
) -> Effects {
    let (m, cia) = machine_for(ram, s);
    let before = (m.chip().to_vec(), m.slow().to_vec());
    // The oracle's return address below its stack, a7 on it (`Cpu68k::call`), as on `Mem`: written
    // before the snapshot, so it is no write of the routine's.
    let sp = STACK - 4;
    let mut flat_before = Mem::new(flat.clone(), &s.pokes);
    flat_before.wl(sp, RETURN_SENTINEL);
    let mut bus = OnMachine {
        m,
        flat: flat_before.clone(),
        cia,
    };
    let mut r = pcode_rt::Regs::new();
    for n in 0..8 {
        r.set_d(n, s.d[n]);
        r.set_a(n, s.a[n]);
    }
    r.set_a(7, sp);
    let ran = match run {
        Some(f) => on_run_stack(|| f(&mut r, &mut bus, img)),
        None => Err(Stop::Untranslated(at.to_string())),
    };
    if let Err(stop) = ran {
        fail(stops, stop);
    }
    let m = &bus.m;
    // The flat bytes are outside the machine's windows by `on_machine`, so no address is listed twice.
    let mut writes = bus.flat.changed_since(&flat_before);
    changed(m.chip(), &before.0, 0, &mut writes);
    changed(m.slow(), &before.1, CHIP, &mut writes);
    Effects {
        regs: Regs {
            d: std::array::from_fn(|n| r.d(n)),
            a: std::array::from_fn(|n| r.a(n)),
            ..Regs::default()
        },
        writes,
        hw_writes: m
            .custom_writes()
            .iter()
            .map(|&(_, reg, v)| (reg, v))
            .collect(),
    }
}

/// Every byte of `now` that differs from `was`, as (flat address, new byte), `now` being [`ram`]'s
/// bytes from index `from`. Compared a page at a time: a state changes a few pages of the megabyte.
fn changed(now: &[u8], was: &[u8], from: usize, out: &mut Vec<(u32, u8)>) {
    const PAGE: usize = 4096;
    for (p, (n, w)) in now.chunks(PAGE).zip(was.chunks(PAGE)).enumerate() {
        if n == w {
            continue;
        }
        for (i, (&b, &o)) in n.iter().zip(w).enumerate() {
            if b != o {
                out.push((flat_address(from + p * PAGE + i), b));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_index_and_flat_address_are_inverse_over_the_a500s_ram() {
        for a in [0, 0x7_FFFF, 0xC0_0000, 0xC7_FFFF] {
            assert_eq!(flat_address(ram_index(a).unwrap()), a);
        }
        for a in [0x8_0000, 0x7E_0000, 0xBF_FFFF, 0xC8_0000, 0xDF_F000] {
            assert_eq!(ram_index(a), None, "${a:06x}");
        }
    }

    #[test]
    fn the_machine_answers_the_a500s_ram_and_the_oracles_hardware_windows_only() {
        for a in [0, 0x7_FFFF, 0xC0_0000, 0xC7_FFFF, 0xDF_F000, 0xDF_F1FF] {
            assert!(on_machine(a), "${a:06x}");
        }
        // A chip RAM mirror, the rig's stack, a custom register mirror, the CIAs' $A00000 mirror,
        // and the oracle's CIA window, which the judge answers itself.
        for a in [
            0x8_0000, 0x7E_FFFC, 0xC8_0000, 0xDE_F000, 0xDF_F200, 0xA0_E001, 0xFF_FFFF, 0xBF_E001,
        ] {
            assert!(!on_machine(a), "${a:06x}");
        }
        assert!(is_cia(0xBF_1697) && is_cia(0xBF_E001) && !is_cia(0xA0_E001));
        assert!(on_machine(0xFF00_0004), "24 bits");
    }

    #[test]
    fn case_reads_become_words_on_their_registers_and_cias() {
        let s = Setup {
            hw_reads: vec![
                (0xDF_F002, 0x12),
                (0xDF_F003, 0x34),
                (0xDF_F00F, 0x56),
                (0xDF_F01F, 0x40),
                (0xDF_F006, 0x99),
                (0xBF_E001, 0x7F),
                (0xBF_D100, 0x01),
            ],
            beam: 0x2C,
            ..Setup::default()
        };
        let (regs, mut reads, cias) = lay_hardware(&s);
        assert_eq!(regs[1], DMACON);
        assert_eq!((regs[2], regs[3]), (0x8000, 0x2C00));
        assert_eq!(regs[0x0F], 0x0040, "INTREQR is the snapshot's INTREQ");
        assert_eq!(reads.read(0x002, 0), 0x1234);
        assert_eq!(reads.read(0x00E, 0), 0x0056, "a byte not given reads 0");
        assert_eq!(reads.words.len(), 2, "VHPOSR is the beam's");
        assert_eq!(cias, [(0xBF_E001, 0x7F), (0xBF_D100, 0x01)]);
    }
}
