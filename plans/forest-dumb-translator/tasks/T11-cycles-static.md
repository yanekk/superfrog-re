# T11 — cycles-static

**Phase:** 1 · **Depends on:** T06 · **Weight:** medium

## Goal

Count cycles in the generated code: each instruction adds its fixed cost (the 68000's base time plus its effective-address times) to a counter in `Regs`, and branches add their taken or not-taken time. The beam position, and so every raster wait, comes from this count (DESIGN §2.7).

## Design sections this implements

DESIGN §2.6, §2.7.

## Files

- `crates/pcode-rt/src/lib.rs` (`Regs::cycles: u64`, `tick(n)`), `crates/pcode-rt/src/cycles.rs` (new: the timing table by kind and EA mode).
- `tools/pcode_fix/cycles.py`, `tools/pcode_rust.py` (emit `r.tick(n)` per instruction; per branch on the taken and untaken path).

## Interface

```rust
pub fn ea_cycles(mode: Ea, size: Size) -> u32;        // the 68000's EA calculation times
pub fn base_cycles(kind: Kind) -> u32;                 // per census kind
impl Regs { pub fn cycles(&self) -> u64; pub fn tick(&mut self, n: u32); }
```

## Tests

- [ ] Unit: the timing table on the 68000's published examples for `move`, `add`, `lea`, `jsr`, `bcc` taken and not.
- [ ] `insn_check.py --columns cycles`: every kind whose time is not data-dependent equals Moira.

## Done when

- [ ] Every fixed-time census kind passes the cycles column; the data-dependent kinds are listed for T12.
- [ ] The generated tree still builds within the time T04 recorded, plus at most 20%.
