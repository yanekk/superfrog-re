# T35 — end-sequence

**Phase:** 2 · **Depends on:** T20 · **Weight:** heavy

Added by T20 (owner, 2026-10-12): the end sequence (`end_seq_load` h0+$a65c, `end_sequence`
h0+$a6e4) loads `END1` and `END2` from disk 3 into hunk 1, unpacks each into hunk 3 and plays it
as an IFF ANIM (`h0+$ab02` open, `h0+$ab82` next frame, `h0+$a890`/`$a95e` the two players) with
fades and timed holds (`h0+$5ea4`). Each animation frame is decoded by the CPU, so how many frames
each step takes must be measured (the decoding can run past a frame's end: T20's jobs and
`Cx::busy`). It is reached only after the final level.

## Goal

The end sequence in the engine from its start to the return to `top_loop`, every frame matching
the original, entered from a transplanted state at `end_seq_load`.

## Design sections this implements

DESIGN §2.1; docs/08 "Scene flow"; docs/13; T08's ANIM decoder (`sf_assets::decode_anim`).

## Files

- `engine/crates/sf-core/src/front/ending.rs`
- `engine/crates/sf-conform/src/sync.rs` — a sync point at the end sequence's start (the final
  level has no level code: a transplant of the original's memory and CPU there, built by driving
  the oracle or by writing the state the end sequence reads).
- `engine/conform/{scripts,cases}/ending-*`; `docs/13-front-end.md` — the end sequence's timings.

## Tests

- [ ] frame conformance from the end sequence's start to the title, untouched.

## Done when

- [ ] The case green in `sf-conform -- all`.
