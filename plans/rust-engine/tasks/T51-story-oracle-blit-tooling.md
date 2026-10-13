# T51 — story-oracle-blit-tooling

**Phase:** 2 · **Depends on:** T46; blocks T52 · **Weight:** light

**Split from:** T46. Tooling only; no engine change.

## Goal

The oracle reports what times the original's blits, so draw timing can be checked to the cycle:

- the shim keeps every custom-register write the CPU makes, with its beam (the existing write hook,
  accessor 0): `sf_amiga_cpu_writes`, `Amiga::cpu_writes` (shim only, no core rebuild);
- core patch 0005 also calls `vamiga_blit_hook(event, vpos, hpos)` in `Blitter::beginBlit` (0),
  `clearBusyFlag` (1) and `endBlit` (2): `Amiga::blit_events`, with a `next` flag as for interrupts;
- `Amiga::frames_run()`: frames a step ran (`agnus.pos.frame` at its end minus its start);
- `story_dbg`: `BLITS=1` (BLTSIZE/BLTCON0 writes and blitter events), `REGS=1` (CPU registers),
  `CMP=addr:len` (bytes differing between the original and the engine).

## Found in T46

A step runs until the CPU ends an instruction after the frame's end: a blit holding the CPU makes
the step, and the events it lists, run into the next frame (events flagged `next`). Changing a
patch rebuilds vAmiga once (`tools/setup/oracle.sh`, about a minute).

## Interface

`sf_amiga_cpu_writes`, `Amiga::cpu_writes`, `Amiga::blit_events`, `Amiga::frames_run` (above).

## Files

`engine/crates/sf-oracle/` (`patches/0005-irq-entry-hook.patch`, `shim/shim.{h,cpp}`,
`src/{amiga,ffi,lib}.rs`); `engine/crates/sf-conform/examples/story_dbg.rs`.

## Tests

- [ ] `sf-oracle` tests: a frame of the intro has BLTSIZE writes and blitter events in order
  begin, free, end.

## Done when

- [ ] Tests in `sf-oracle` for the new calls (a frame of the intro has BLTSIZE writes and blitter
  events in order begin, free, end); the test block green.
