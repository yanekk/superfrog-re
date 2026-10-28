# T10 — display-piece

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Write the one new display piece (owner, 2026-10-28): draw a `Picture` from bitplanes in chip memory
and the copper's register writes, as Denise shows a lores frame. The dry fit needs it to draw a
picture from toolkit crates alone, and game two's first screens can be looked at with it before any
of its renderer is ported. Superfrog's renderer does not use it.

## Design sections this implements

DESIGN §2.9.

## Files

- New: `toolkit/crates/amiga-chips/src/display.rs`, `lib.rs` (`pub mod display`).

## Interface

```rust
pub struct Window { pub diwstrt: u16, pub diwstop: u16, pub ddfstrt: u16, pub ddfstop: u16 }
pub fn draw(mem: &impl Ram, writes: &[copper::Write], window: Window) -> Picture;
```

It follows BPLCON0 (plane count, EHB at 6 planes; HIRES, HAM and dual playfield are out and
`draw` panics on them with a message), BPL1PT–BPL6PT, BPL1MOD/BPL2MOD, COLOR00–31 as the writes
leave them per line, and records a colour write inside a line as a `Split`. Sprites are out.

## Tests

- [ ] One plane, a known byte pattern: the pixels match bit for bit.
- [ ] Five planes: each pixel's index is the planes' bits combined in the right order.
- [ ] Six planes with EHB: indices 32–63 appear.
- [ ] Modulo: the second line starts `width/8 + mod` bytes on.
- [ ] A copper colour write on line 10 changes line 10's palette and not line 9's; one mid-line
      gives a `Split` at the right x.
- [ ] BPLxPT rewritten mid-frame moves the fetch from that line on.
- [ ] HIRES or HAM panics with a message.

## Done when

- [ ] `amiga_chips::display::draw` exists with the tests above, pure, no game name.
- [ ] Test block green.
