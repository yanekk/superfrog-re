# T08 — side-objects

**Phase:** 2 · **Depends on:** T02, T03 · **Weight:** heavy

## Goal

Draw every live bob in the widescreen side strips, pixel-identical to the same bob drawn in the centre
and with no seam where one crosses the centre's edge, from a draw list `bob_draw` records for the buffer
it draws.

## Design sections this implements

DESIGN §2.9, D6.

## Files

- `engine/crates/sf-core/src/layout.rs` (two wide draw lists, one per display buffer)
- `engine/crates/sf-core/src/level/objects.rs` (`bob_draw` fills the list for its back buffer)
- `engine/crates/sf-core/src/render/{mod,objects,playfield}.rs` (compose: map, then the shown buffer's
  list, then sprites, in the strip columns)
- `engine/crates/sf-core/src/render/bobs.rs` (new: cookie-cut a bob from its frame record and mask into a
  picture)

## Interface

```rust
// One entry per bob drawn by a pass, in draw order (layout: fixed-size array + count, presentation).
pub struct WideBob { pub x: i16, pub y: i16, pub frame: u32, pub background: bool }
// x, y: level position the pass drew it at (latched camera + screen_positions' x).

// render/bobs.rs
pub(crate) fn draw_bob(p: &mut Field, at: (i32, i32), frame: u32, mem: &Mem, data: &DisplayData,
                       masked: bool);
```

The list includes bobs `draw_bob` culled (screen x ≥ $170 or left-clipped) since those are the strip's.
`compose` draws only in strip columns; the centre stays the buffer. Lists are presentation (never
compared) and serialized with `State`.

## Tests

- [ ] a bob straddling the centre's right edge: the strip half continues the centre half pixel for pixel
  (render test on a constructed state and on W1-1 at a chosen frame).
- [ ] the same bob scrolled from the centre into the strip draws the same pixels at both positions.
- [ ] a background bob (bob +4 negative) is drawn unmasked; front/back priority order is kept.
- [ ] widescreen off: the list may be filled but the 4:3 picture is unchanged (existing render tests).
- [ ] a bob in an extra slot (T03) shows in the strip.
- [ ] a loaded save state draws its strips at once.

## End to end (the worker drives this)

- suite: `sf-e2e` new `widescreen_objects.rs` · size: 2× widescreen
- [ ] W1-1 with widescreen on from the start, 1 500 frames walking right: some frame shows a non-map
  pixel (a bob) in the right strip, and every frame's strip equals a headless render of the same state.

## Done when

- [ ] Tests green; e2e test green.
- [ ] `cd engine && cargo test -q --workspace` run once, recorded.
