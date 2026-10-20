# T06 — routine-oracle

**Phase:** 1 · **Depends on:** T04, T05 · **Weight:** medium

## Goal

Let a test call one routine of the original game in isolation: load the relocated executable and a
memory snapshot into Moira (vAmiga's 68000 core) with no chipset, set registers, run until the routine
returns, and get back registers and a memory diff. This is how each ported routine is checked against
its original on thousands of generated inputs in milliseconds, long before a whole level runs.

## Design sections this implements

DESIGN §4 (routine conformance), §3.2 (`sf-oracle`).

## Files

- `engine/crates/sf-oracle/shim/moira_shim.{h,cpp}` (built by `tools/setup/oracle.sh`)
- `engine/crates/sf-oracle/src/routine.rs`
- `engine/crates/sf-oracle/tests/routine.rs`

## Interface

```rust
pub struct Cpu68k;  // Moira with flat 24-bit memory
impl Cpu68k {
    pub fn new(mem: AmigaMemory) -> Cpu68k;                 // e.g. from Amiga::memory() at a sync point
    pub fn load_exe(&mut self, exe: &[u8], at: u32) -> HunkMap;  // relocates hunks like the game's loader
    pub fn call(&mut self, entry: u32, regs: Regs, max_instr: u64) -> Result<CallResult, CallError>;
}
pub struct CallResult { pub regs: Regs, pub writes: Vec<(u32, u8, u8)> /* addr, old, new */, pub instr: u64 }
pub enum CallError { Timeout, CustomChipAccess { pc: u32, addr: u32 }, IllegalInstruction { pc: u32 } }
```

A custom-chip access is an error rather than emulated, because a routine that touches hardware
belongs to the full-system oracle, and the error tells the porter so.

## Tests

- [ ] calling the ATN! routine at stage-1.5 `+0x2DA` on a chunk reproduces `sf-disk::atn` output (a
      first real routine, and a cross-check on T04).
- [ ] `max_instr` stops an infinite loop with `Timeout`.
- [ ] a write to `$DFF000`–`$DFF1FF` or `$BFxxxx` gives `CustomChipAccess` with the PC.
- [ ] relocation: an absolute reference in a loaded hunk points at the relocated target.
- [ ] two identical calls give identical results (no state leaks between calls).

## Done when

- [ ] The ATN! cross-check passes on all 103 chunks.
- [ ] Tests are in the test block.
- [ ] A short "how to differential-test a routine" section in `docs/RE-WORKBENCH.md` with one example.
