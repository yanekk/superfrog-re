# T32 — story-intro

**Phase:** 2 · **Depends on:** T20 · **Weight:** heavy

Added by T20 (owner, 2026-10-12): the Team 17 logo and Eric Schwartz's story intro are a separate
program on disk 3 (`d3_c02` boots `d3_c04`, docs/08), played only when disk 3 is booted. The
oracle boots disk 1, which skips them, so T20 left them out.

## Goal

The disk-3 intro in the engine: the Team 17 logo and the story's ~16 skippable scenes (`d3_c04`,
graphics `d3_c05`–`d3_c08`, audio `d3_c03`), each frame-exact against the original booted from
disk 3, including every skip key and the hand-over to the game ("insert disk 1"), which then
starts as T20's `Start::PowerOn` does.

## Design sections this implements

DESIGN §1 (every part of the original), §2.1, §2.3.

## Files

- `engine/crates/sf-oracle/src/amiga.rs` — power on with disk 3 in df0 (today disk 1 always).
- `engine/crates/sf-conform/src/sync.rs` — a sync point booted from disk 3.
- `engine/crates/sf-core/src/front/story.rs` — the intro as a front-end scene (T20's `front` module).
- `engine/crates/sf-assets` — decode `d3_c03`–`d3_c08` (T08 kept the intro banks raw; their layout
  is ambiguous, FINDINGS 2026-10-11).
- `engine/conform/{scripts,cases}/story-*`; `docs/13-front-end.md` — the intro's scenes and timings.

## Interface

```rust
pub enum Start { /* … */ Disk3 { seed: Rng } }   // boot as disk 3 does: the intro, then the game
```

## Tests

- [ ] frame conformance from the intro's start, untouched, to its hand-over to disk 1, with loads
      re-synced as T20 does (owner, 2026-10-12: loads are not modelled frame by frame).
- [ ] the same with each skip input the original reads, at several points.

## Done when

- [ ] All cases green; one short intro case in the test block.
- [ ] docs/13 has the intro's scene list and timings.
