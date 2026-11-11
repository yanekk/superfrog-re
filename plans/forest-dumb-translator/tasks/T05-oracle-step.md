# T05 — oracle-step

**Phase:** 0 · **Depends on:** — · **Weight:** light

## Goal

Give the reference processor a one-instruction step so each instruction kind can be compared alone, with its condition codes and cycles. `Cpu68k::call` runs only to a return sentinel.

## Design sections this implements

DESIGN §2.6.

## Files

- `toolkit/crates/amiga-oracle/src/routine.rs`, its tests, `toolkit/crates/amiga-oracle/README.md`. Names no game.

## Interface

```rust
pub struct Stepped { pub regs: Regs /* d, a, sr, pc */, pub writes: Vec<(u32, u8, u8)>, pub cycles: u32 }
impl Cpu68k {
    /// Runs exactly one instruction at `pc` from `regs`; hardware mode as set.
    pub fn step(&mut self, pc: u32, regs: Regs) -> Result<Stepped, CallError>;
}
```

`Regs` gains `usp`/`ssp` if Moira distinguishes them for `move usp`, `rte`; say so in the README.

## Tests

- [ ] `moveq #-1,d0`: d0, N set, Z clear, 4 cycles.
- [ ] `add.l d1,d0` with overflow: V and C, X follows C.
- [ ] `move.w d0,(a0)+`: writes two bytes, a0 += 2.
- [ ] A taken and an untaken `bne.s`: PC and cycles differ.
- [ ] An illegal opcode returns `IllegalInstruction`.

## Done when

- [ ] `Cpu68k::step` with its tests passes in `amiga-oracle`, labelled in the toolkit's smoke config.
- [ ] README documents it.
