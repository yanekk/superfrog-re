# T13 — renderer

**Phase:** 2 · **Depends on:** T08, T12 · **Weight:** heavy

## Goal

Draw the frame the original displays from the engine's state alone: the scrolling 5-bitplane tile
playfield, bobs and hardware sprites in the original's priority order, the status panel, and the
per-line palette changes the copper makes. Not a blitter emulation: a software renderer that produces
the same pixels. It also takes the widescreen view parameter, drawing extra level columns from the
same state (DESIGN §2.7).

## Design sections this implements

DESIGN §2.1 (picture), §2.7, §3.3 (`render`).

## Files

- `engine/crates/sf-core/src/render/{mod.rs,playfield.rs,objects.rs,panel.rs,copper.rs,view.rs}`
- `engine/crates/sf-core/tests/render_vs_oracle.rs`
- `docs/12-display.md` — display window, scroll registers, sprite/bob composition, copper lists.

## Interface

```rust
pub struct Picture { pub width: u16, pub height: u16, pub pixels: Vec<u8> /* colour index */, pub line_palettes: Vec<[u16; 32]> /* 12-bit, per line */ }
pub struct View { pub extra_columns_each_side: u16 }   // 0 = original
impl Game { pub fn render(&self, view: View) -> Picture }
pub fn to_rgb12(p: &Picture) -> Vec<u16>
```

## Tests

- [ ] for 200 sync points spread over all 29 levels (transplanted from oracle memory, no stepping),
      `render(View{0})` equals the oracle frame pixel for pixel.
- [ ] scrolling: a scripted horizontal and vertical scroll through W1-1, every frame equal.
- [ ] widescreen: centre 320 columns of `render(View{n})` equal `render(View{0})` for n in {32, 53, 80}.
- [ ] palette effects: each copper effect T07/this task finds (panel split, colour cycling from `CCCL`) matches on 100 frames.
- [ ] levels' left/right edges with widescreen: columns outside the map render as the original's out-of-map colour, no panic.

## Done when

- [ ] Static-state render equal on all 200 sync points, in the test block (a 20-point subset; all in `sf-conform -- all`).
- [ ] docs/12 written.
- [ ] Render time per frame under 2 ms on this machine (release build), measured and noted.
