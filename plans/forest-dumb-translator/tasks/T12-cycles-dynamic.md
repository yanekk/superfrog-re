# T12 — cycles-dynamic

**Phase:** 1 · **Depends on:** T11 · **Weight:** heavy

## Goal

The cycles that depend on data: shifts and rotates by a count, `mulu`/`muls` (by the bits of the source), `divu`/`divs`, `dbcc` (taken, expired, condition true), `movem` (by register count), `scc`, `bset`-family by bit number, and exceptions (`trap`, `chk`, divide by zero). Computed at run time by helpers using Moira's rules.

## Design sections this implements

DESIGN §2.6.

## Files

- `crates/pcode-rt/src/cycles.rs` (dynamic helpers), `tools/pcode_fix/cycles.py`, tests.

## Interface

```rust
pub fn mulu_cycles(src: u16) -> u32;
pub fn muls_cycles(src: u16) -> u32;
pub fn divu_cycles(dividend: u32, divisor: u16) -> u32;
pub fn divs_cycles(dividend: i32, divisor: i16) -> u32;
pub fn shift_cycles(size: Size, count: u32) -> u32;
pub fn movem_cycles(dir: Dir, ea: Ea, size: Size, nregs: u32) -> u32;
```

## Tests

- [ ] `mulu` by 0, `$ffff`, `$5555`; `muls` by -1, `$aaaa`: equal Moira.
- [ ] `divu`/`divs` on 200 random pairs plus overflow and zero divisor: equal Moira (Moira's own divide timing is the reference).
- [ ] `dbcc` in all three outcomes; `movem` with 1, 8 and 16 registers.

## Done when

- [ ] Every census kind passes the cycles column.
- [ ] FINDINGS records any kind where Moira's timing is known to differ from a real 68000, with its source.
