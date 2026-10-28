# T10 — display-piece

**Phase:** 2 · **Depends on:** T01 · **Weight:** heavy

## Goal

Give the toolkit one display piece (owner, 2026-10-28; built from the existing renderer at plan
review): draw a `Picture` from bitplanes in chip memory and the copper's register writes, as Denise
shows a lores frame. Its core is lifted out of sf-core's renderer, which already tracks the bitplane
registers with timings measured against the original, so there is one Denise: sf-core's renderer
calls the moved core and keeps its Superfrog parts. The dry fit needs it to draw a picture from
toolkit crates alone, and game two's first screens can be looked at with it before any of its
renderer is ported.

## Design sections this implements

DESIGN §2.9, §2.3 (`learned on:` for the measured pipeline), §3.3.

## Files

- New: `toolkit/crates/amiga-chips/src/display.rs`, `lib.rs` (`pub mod display`).
- `engine/crates/sf-core/src/render/mod.rs` (`Planes`, `PIPELINE`, `HIRES_PIPELINE`, `index`,
  `index_hires`, `delay`, the pointer advance and the colour/`Split` apply move out; `Denise::walk`,
  `Mem`, `Under`, sprites, the level field, widescreen and `panel::fetch` stay and use them), and
  `render/panel.rs` if it reads `Planes`.

## Interface

```rust
// amiga_chips::display (pure)
pub struct Planes { pub fetch: copper::Fetch, pub bplcon1: u16, pub bplcon2: u16,
                    pub modulo: [i16; 2], pub pt: [u32; 6] }   // write(), delay(), index(), index_hires(), priority(), advance()
pub const PIPELINE: i32 = 17;        // learned on: docs/12's measurement
pub const HIRES_PIPELINE: i32 = 2;
pub fn draw(mem: &impl Ram, writes: &[copper::Write], start: copper::Fetch) -> Picture;
```

`draw` follows BPLCON0 (plane count, EHB at 6 planes; HIRES, HAM and dual playfield are out and
`draw` panics on them with a message), BPL1PT–BPL6PT, BPL1MOD/BPL2MOD, BPLCON1's delay,
COLOR00–31 as the writes leave them per line, and records a colour write inside a line as a
`Split`. Sprites are out. Exact shapes of the moved items follow today's; a rename is the worker's.

## Tests

- [ ] One plane, a known byte pattern: the pixels match bit for bit.
- [ ] Five planes: each pixel's index is the planes' bits combined in the right order.
- [ ] Six planes with EHB: indices 32–63 appear.
- [ ] Modulo: the second line starts `2 * words + mod` bytes on.
- [ ] A copper colour write on line 10 changes line 10's palette and not line 9's; one mid-line
      gives a `Split` at the right x, where sf-core's renderer would put it.
- [ ] BPLxPT rewritten mid-frame moves the fetch from that line on.
- [ ] HIRES or HAM panics with a message.
- [ ] sf-core's pictures unchanged: its render unit tests, sf-conform `--test beam`, `--test story
      -- --ignored` and `--test frog_cases` give T01's results.

## Done when

- [ ] `amiga_chips::display` holds the moved Denise core and `draw`, pure, no game name, with the
      synthetic tests above; sf-core's renderer uses it and keeps only Superfrog's parts.
- [ ] Test block green; DESIGN §4's per-task runs green.
