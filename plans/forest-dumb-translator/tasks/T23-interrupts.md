# T23 — interrupts

**Phase:** 4 · **Depends on:** T22 · **Weight:** heavy

## Goal

Interrupts as the 68000 and Paula take them: INTENA, INTREQ and the SR mask; a pending, enabled interrupt taken at the next block start through `Bus::poll` (T14), by pushing the exception frame and calling the translated handler at the vector's address, so `rte` returns into the interrupted code. Play runs inside the copper interrupt (`level3_handler`), so this is what makes the game run.

## Design sections this implements

DESIGN §2.8 (interrupts).

## Files

- `crates/pcode-machine/src/irq.rs` (new), `custom.rs`, `lib.rs` (`poll`).
- `crates/pcode-rt`: `rte` emission confirmed against the pushed frame (`tools/pcode_fix/move_control.py` if it needs a change).

## Interface

```rust
impl Machine {
    pub fn raise(&mut self, bit: u16, at: u64);          // INTREQ bit set by a device at cycle `at`
    fn pending(&self, sr: u16) -> Option<u8 /*level*/>;   // INTENA & INTREQ & master, above the SR mask
}
// poll(): if pending, push PC and SR, set S and the mask, jump through the vector at $64 + 4*(level-1)
```

Vectors the program installs are in memory, so the handler is the translated function at that address; a vector into the OS is an `os` call (T26).

## Tests

- [ ] Made-up program: a foreground loop polling a flag the level 3 handler sets; the handler runs once per raised interrupt and `rte` resumes the loop.
- [ ] Masked by SR: not taken until the mask drops. Disabled in INTENA: not taken. INTREQ cleared by the handler: not re-taken.
- [ ] Nested: a level 3 request during a level 2 handler is taken; a level 2 during level 3 is not.

## Done when

- [ ] The tests pass on made-up programs; the README describes block-boundary delivery and why (DESIGN §2.8).
