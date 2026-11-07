# T02 — pcode-runtime

**Phase:** 1 · **Depends on:** T00 · **Weight:** medium

## Goal

Translated code needs a small library that carries out P-code's operations exactly, holds the 68000's registers as Ghidra lays them out, reaches memory through whatever the host provides, and stops loudly instead of guessing (DESIGN §2.5). This task builds it, game-free and pure, so the judge now and the app in the second plan can both run the same generated code. It depends on T00 only because it registers itself in files level 1 changed.

## Design sections this implements

DESIGN §2.5, §3.1, §3.3, §5.2 (the step cap).

## Files

- `games/forest-dumb/engine/crates/pcode-rt/` (new crate: `Cargo.toml`, `src/lib.rs`, `src/ops.rs`, tests in `src/`; names no game)
- `games/forest-dumb/engine/crates/fd-conform/tests/purity.rs` (add `pcode-rt` to `PURE`)
- `games/forest-dumb/engine/Cargo.toml` (an `opt-level` override for `pcode-rt`, since generated code runs millions of blocks per judge run)
- `games/forest-dumb/smoke.toml` (label)

## Interface

```rust
/// Memory as the host provides it. The judge implements it over its `Mem` (T04).
pub trait Bus {
    fn rb(&mut self, a: u32) -> u8;
    fn rw(&mut self, a: u32) -> u16;
    fn rl(&mut self, a: u32) -> u32;
    fn wb(&mut self, a: u32, v: u8);
    fn ww(&mut self, a: u32, v: u16);
    fn wl(&mut self, a: u32, v: u32);
}

/// Ghidra's 68000 register space, byte for byte, big-endian, sized from its language spec.
pub struct Regs { pub space: [u8; REG_SPACE] }
impl Regs {
    pub fn get(&self, off: usize, size: usize) -> u64;
    pub fn set(&mut self, off: usize, size: usize, v: u64);
    pub fn d(&self, n: usize) -> u32;  pub fn set_d(&mut self, n: usize, v: u32);
    pub fn a(&self, n: usize) -> u32;  pub fn set_a(&mut self, n: usize, v: u32);
}

/// Where the program sits, given at run time, and which bytes are translated instructions.
pub struct Image<'a> { pub bases: &'a [u32], pub code: &'a [(u32 /*hunk*/, u32 /*start*/, u32 /*end*/)] }
impl Image<'_> {
    pub fn h(&self, hunk: u32, off: u32) -> u32;
    pub fn hunk_of(&self, a: u32) -> Option<(u32, u32)>;   // absolute → (hunk, off)
    pub fn is_code(&self, a: u32, size: u32) -> bool;
}

pub enum Stop {
    Untranslated(String),            // "hN+$off", or "$xxxxxx" outside every hunk
    WriteIntoCode(String),           // "hN+$off"
    Unsupported { op: String, at: String },
    Hang,                            // the step cap, DESIGN §5.2
}

/// Loads and stores the generated code calls; a store into code returns Stop::WriteIntoCode.
pub fn load<B: Bus>(bus: &mut B, a: u32, size: usize) -> u64;
pub fn store<B: Bus>(bus: &mut B, img: &Image, a: u32, size: usize, v: u64) -> Result<(), Stop>;

/// P-code ops on values of 1, 2, 4 or 8 bytes held in u64 (ops.rs): int_add, int_sub, int_carry,
/// int_scarry, int_sborrow, int_less, int_sless, int_equal, int_2comp, int_negate, int_zext,
/// int_sext, int_mult, int_div, int_sdiv, int_rem, int_srem, int_left, int_right, int_sright,
/// int_and, int_or, int_xor, bool_negate, bool_and, bool_or, bool_xor, subpiece, piece, popcount.
pub mod ops;
pub const STEP_CAP: u64 = 10_000_000;
```

Word and long accesses are big-endian through `Bus`; `load`/`store` split an 8-byte access into two longs. Division by zero is not trapped here (the 68000's trap is stage 1's); the op returns 0 and the caller's P-code decides.

## Tests

- [ ] Every op at every size it takes, against hand-computed values, including carry out of the top bit, signed overflow both ways, shifts by 0, by the size and past it, and `sext` of the top bit.
- [ ] `Regs`: `d(n)`/`a(n)` agree with `get` at Ghidra's offsets; a byte write to `d0`'s low byte leaves the rest.
- [ ] `store` into a code range at its first byte, last byte and one byte before it: two stops, one write.
- [ ] `Image::h` and `hunk_of` round-trip; an address outside every hunk is `None`.
- [ ] `purity.rs` passes with `pcode-rt` in `PURE`.

## Done when

- [ ] `cargo test -q -p pcode-rt` and `-p fd-conform --test purity` pass; the crate has no dependencies and names no game.
- [ ] Each test file is labelled in `smoke.toml`.
