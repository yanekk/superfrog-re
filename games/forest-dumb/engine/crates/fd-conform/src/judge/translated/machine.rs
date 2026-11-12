//! A judged state on `pcode-machine` (translator T27, DESIGN §2.8, §4): the translated entry runs on
//! the machine the app runs, so its blits, beam waits and custom-register reads come from the
//! machine's models rather than [`super::Mem`]'s hand copy of the routine oracle's hardware mode.
//!
//! How a [`Setup`] is laid on the machine:
//!
//! - **Memory:** `Machine::from_snapshot` of the rig's memory (program, generator base, the state's
//!   pokes) as the A500 has it: chip RAM from 0, slow RAM from `$C00000`, 512 KB each. A poke the
//!   A500 has no RAM for is left out; an access there stops `outside-memory`, as on the machine.
//! - **The stack:** the rig calls the original on [`STACK`], in its flat 16 MB, where the A500 has
//!   no RAM. [`STACK_AREA`] is kept beside the machine ([`OnMachine`]), so a7 and every byte
//!   pushed are the original's; the rig does not compare that area.
//! - **Hardware reads the case gives** (`Setup::hw_reads`) are answered as the routine oracle answers
//!   them: a custom register's word from its two bytes (a byte not given reads 0), by a device
//!   attached to that register ([`CaseReads`]); a CIA's, by a device on that CIA. INTENAR and
//!   INTREQR, which no device may answer, are the snapshot's INTENA and INTREQ. VHPOSR given by a
//!   case is ignored, as the oracle ignores it.
//! - **The beam:** at line `Setup::beam`, slot 0, at cycle 0, so the first VHPOSR read gives the
//!   byte the original's first read gave; later reads follow the counted cycles, where the oracle's
//!   count one a read.
//! - **DMA:** [`DMACON`], the blitter alone: the routine oracle runs no copper, display or sound.
//!   Interrupts are all disabled, as the oracle takes none; CIA-A is at reset.
//!
//! Compared as on [`super::Mem`]: the registers, every RAM byte that differs from the snapshot
//! (slow RAM at its `$C00000` address), and the custom-register writes in order, the CPU's and any
//! the copper made.

use std::sync::Arc;

use amiga_oracle::{RETURN_SENTINEL, Regs};
use pcode_machine::custom::{self, Cia};
use pcode_machine::{Config, Device, Machine};
use pcode_rt::{Bus, Image, Stop};

use super::{Entry, Stops, fail, on_run_stack};
use crate::HUNK_BASES;
use crate::routine::{Effects, STACK, STACK_AREA, Setup};

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

/// The custom registers' snapshot and the devices `s.hw_reads` asks for.
fn lay_hardware(s: &Setup) -> ([u16; 256], CaseReads, [CaseReads; 2]) {
    let mut regs = [0u16; 256];
    regs[custom::DMACONR as usize >> 1] = DMACON;
    // VPOSR (LOF, a long frame) and VHPOSR: line `beam`, slot 0, at cycle 0.
    regs[custom::VPOSR as usize >> 1] = 0x8000;
    regs[custom::VHPOSR as usize >> 1] = (s.beam as u16) << 8;
    let (mut reads, mut cias) = (
        CaseReads::default(),
        [CaseReads::default(), CaseReads::default()],
    );
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
        } else if a & 0xFF_0000 == 0xBF_0000 {
            // A byte no CIA answers reads the bus on the machine, 0 on the oracle: left to the bus.
            if let Some(cia) = custom::cia_read8(a) {
                cias[cia as usize].set(custom::cia_reg(a), false, v);
            }
        }
    }
    (regs, reads, cias)
}

/// The machine with the rig's stack beside it: [`STACK_AREA`] is the oracle's flat memory, not the
/// A500's, so it is kept here and every other access goes to the machine.
struct OnMachine {
    m: Machine,
    stack: Vec<u8>,
}

impl OnMachine {
    fn stack_index(a: u32) -> Option<usize> {
        let a = a & 0xFF_FFFF;
        STACK_AREA
            .contains(&a)
            .then(|| (a - STACK_AREA.start) as usize)
    }
}

impl Bus for OnMachine {
    fn poll(&mut self, r: &mut pcode_rt::Regs, img: &Image) -> Result<(), Stop> {
        self.m.poll(r, img)
    }
    fn call_outside(&mut self, a: u32, r: &mut pcode_rt::Regs, img: &Image) -> Result<(), Stop> {
        self.m.call_outside(a, r, img)
    }
    fn rb(&mut self, a: u32) -> u8 {
        match Self::stack_index(a) {
            Some(i) => self.stack[i],
            None => self.m.rb(a),
        }
    }
    fn rw(&mut self, a: u32) -> u16 {
        match Self::stack_index(a) {
            Some(_) => (self.rb(a) as u16) << 8 | self.rb(a.wrapping_add(1)) as u16,
            None => self.m.rw(a),
        }
    }
    fn rl(&mut self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a.wrapping_add(2)) as u32
    }
    fn wb(&mut self, a: u32, v: u8) {
        match Self::stack_index(a) {
            Some(i) => self.stack[i] = v,
            None => self.m.wb(a, v),
        }
    }
    fn ww(&mut self, a: u32, v: u16) {
        match Self::stack_index(a) {
            Some(_) => {
                self.wb(a, (v >> 8) as u8);
                self.wb(a.wrapping_add(1), v as u8);
            }
            None => self.m.ww(a, v),
        }
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a.wrapping_add(2), v as u16);
    }
}

/// The machine a state starts on: `ram` with `s`'s pokes, the hardware [`lay_hardware`] lays.
pub fn machine_for(ram: &[u8], s: &Setup) -> Machine {
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
    let given: Vec<u16> = reads.words.iter().map(|&(r, _)| r).collect();
    for reg in given {
        m.attach(reg..reg + 2, Box::new(reads.clone()));
    }
    for (cia, dev) in [Cia::A, Cia::B].into_iter().zip(cias) {
        if !dev.words.is_empty() {
            m.attach_cia(cia, Box::new(dev));
        }
    }
    m
}

/// Run `run` from state `s` on the machine, as [`super::check_on`] runs it on [`super::Mem`].
pub(super) fn run(
    ram: &[u8],
    s: &Setup,
    run: Option<Entry>,
    at: &str,
    img: &Image,
    stops: &mut Stops,
) -> Effects {
    let m = machine_for(ram, s);
    // The machine writes ExecBase at $4 when built: memory before the routine is the machine's.
    let before = (m.chip().to_vec(), m.slow().to_vec());
    let mut bus = OnMachine {
        m,
        stack: vec![0; (STACK_AREA.end - STACK_AREA.start) as usize],
    };
    // The oracle's return address below its stack, a7 on it (`Cpu68k::call`), as on `Mem`.
    let sp = STACK - 4;
    bus.wl(sp, RETURN_SENTINEL);
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
    let mut writes = Vec::new();
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
        let (regs, mut reads, mut cias) = lay_hardware(&s);
        assert_eq!(regs[1], DMACON);
        assert_eq!((regs[2], regs[3]), (0x8000, 0x2C00));
        assert_eq!(regs[0x0F], 0x0040, "INTREQR is the snapshot's INTREQ");
        assert_eq!(reads.read(0x002, 0), 0x1234);
        assert_eq!(reads.read(0x00E, 0), 0x0056, "a byte not given reads 0");
        assert_eq!(reads.words.len(), 2, "VHPOSR is the beam's");
        assert_eq!(cias[Cia::A as usize].read(0, 0), 0x7F);
        assert_eq!(cias[Cia::B as usize].read(1, 0), 0x01);
        assert_eq!(cias[Cia::B as usize].read(0, 0), 0);
    }
}
