# T24 — machine-chips

**Phase:** 4 · **Depends on:** T22 · **Weight:** heavy

## Goal

The blitter, the copper, the picture and the sprites on the machine, from the toolkit's models: a BLTSIZE write runs `amiga-chips::Blit` and keeps the blitter busy for `word_times` cycles (DMACONR's busy bit and the blitter-done interrupt follow); each frame the copper list runs with `copper::run_beam`, raising the copper interrupt at its line, and `display::draw` gives the picture.

## Design sections this implements

DESIGN §2.8.

## Files

- `crates/pcode-machine/src/{blitter,copper,video}.rs` (new). `amiga-chips` only if a gap is found, in a separate commit naming no game.

## Interface

```rust
impl Machine {
    pub fn end_frame(&mut self) -> Frame;   // picture, the frame's custom writes in order, copper interrupts raised
}
pub struct Frame { pub picture: amiga_chips::Picture, pub writes: Vec<(u64 /*cycle*/, u16, u16)> }
```

Line and fill blits are a stop (`unsupported: blit line`) unless T01's measurement shows the program uses them; then they are this task's.

## Tests

- [ ] A made-up area blit through register writes gives the same memory as `Blit` direct, and busy clears after `word_times`.
- [ ] A copper list with a WAIT and a MOVE to INTREQ raises the copper interrupt at that line.
- [ ] A made-up two-plane display draws the expected `Picture`.

## Done when

- [ ] Tests pass without disks (smoke); full: the title's first picture from a snapshot of the oracle at that frame equals the oracle's.
