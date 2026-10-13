# T90 — story-draw-blits-oracle

**Split from:** T52. Carries T52's "Done when" for one draw's restores.

**Phase:** 2 · **Depends on:** T89; blocks T53 · **Weight:** light

## Goal

The first draw's blits (oracle frame 3 027, case 543: restores with BLTCON0 `$09f0`, then the
objects with `$cfca`/`$2fca`/`$4fca`/`$8fca`) begin, free and end on the original's cycles in
`story::sim`, given its writes: the bitplane half of the fixed DMA (`Fetch::takes`) checked.

## Found in T52 (measured)

- DMACON `$8400` (BLTPRI) at 51,108 before the restores, so the CPU cannot hold them up. COP1LC
  `$df3e` from oracle frame 3 022. Restores below line ~62 see refresh only (first: write 52,14,
  begin 52,17, free 55,20, end 55,22); later ones meet five-plane lores fetches.
- Check every blit of the frame; if a later one runs without BLTPRI the CPU matters (BLS) and
  belongs to T53: say so and check up to it.

## Interface

None new: `story::sim` and `copper::run_bus` as T52 and T89 leave them; one more frame in
`story_blits.rs`.

## Files

`engine/crates/sf-conform/tests/story_blits.rs` (T89's test, one more frame); `render/copper.rs`
or `front/story/sim.rs` (`Fixed`) if the fetch timing needs it.

## Tests

- [ ] The oracle test extended to frame 3 027: every blit's begin/free/end.

## Done when

- [ ] Frame 3 027's restores match to the cycle; the test block green.
