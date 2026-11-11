# T22 — machine-core

**Phase:** 4 · **Depends on:** T11, T14 · **Weight:** heavy

## Goal

The core of the machine the translated program runs on (DESIGN §2.8): memory laid out as the oracle's A500, the program's hunks at `HUNK_BASES`, a `Bus` that routes custom-register and CIA addresses to devices, and the beam position from counted cycles. The chips, interrupts, sound, input and OS come in T23 to T26 on this base.

## Design sections this implements

DESIGN §2.8, §3.1.

## Files

- `games/forest-dumb/engine/crates/pcode-machine/` (new crate, pure, names no game): `lib.rs`, `mem.rs`, `custom.rs`, `beam.rs`, `README.md`.
- Built on what exists (plan review, 2026-11-11): the hunk file is parsed and relocated with `amiga-disk`'s `hunk` module as `fd-conform/src/routine.rs` `load_program` does, and the beam position comes from `amiga-clock`'s `Clock` (`cpu`, `pos`); `beam.rs` only encodes VPOSR/VHPOSR from it. A gap is closed in the toolkit crate, naming no game, not by a second copy.
- `fd-conform/tests/purity.rs`: add `pcode-machine` to `PURE`. Workspace `Cargo.toml`. `smoke.toml` labels.

## Interface

```rust
pub struct Config { pub chip: u32, pub slow: u32, pub hunk_bases: Vec<u32>, pub frame_lines: u16 /* 313 */ }
pub struct Machine { /* memory, custom regs, beam, devices */ }
impl Machine {
    pub fn new(cfg: Config, program: &[u8] /* hunk file */) -> Result<Machine, LoadError>;
    pub fn from_snapshot(cfg: Config, mem: &[u8], custom: &[u16; 256]) -> Machine; // a case's sync point
    pub fn beam(&self, cycles: u64) -> (u16 /*line*/, u16 /*h*/);
}
impl pcode_rt::Bus for Machine { /* rb..wl; $dff000.. to custom, $bfd000/$bfe001 to CIA */ }
pub trait Device { fn read(&mut self, reg: u16, at: u64) -> u16; fn write(&mut self, reg: u16, v: u16, at: u64); }
```

VHPOSR and VPOSR read the beam at the current cycle count; `frame_lines` is the measured 313 (`docs/frame-length.md`).

## Tests

- [ ] Loading a made-up hunk file places hunks at the bases and applies its relocations.
- [ ] Chip and slow RAM sizes and mirrors as the oracle's configuration; an access outside memory is a `Stop` naming it.
- [ ] VHPOSR after N cycles gives the line and position a 313-line PAL frame implies (227 colour clocks a line).
- [ ] A write to a write-only register and a read of a read-only one behave as on the 68000 bus (the trial's last-word rule).

## Done when

- [ ] `pcode-machine` builds, is in `PURE`, and its tests pass in smoke (they need no disks).
- [ ] README states what is modelled and what is not (DMA contention).
