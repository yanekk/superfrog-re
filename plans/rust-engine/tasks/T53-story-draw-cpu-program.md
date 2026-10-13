# T53 — story-draw-cpu-program

**Phase:** 2 · **Depends on:** T52; blocks T54 · **Weight:** medium

**Split from:** T46.

## Goal

`bobs::draw` builds, beside its memory work, the CPU's instructions of `bobs_draw` (h0+$87de..$8b66)
branch for branch as a `sim::Prog`, and the intro times its draws with `sim` in place of
`BlitClock`'s `CPU_PER_BLIT` / `CPU_PER_OBJECT` / `CPU_RESTORE` estimates.

## Found in T46

- The draft program (discarded) put the original's first ~6 BLTSIZE writes of a draw within 1–3
  cycles of the oracle's, from the draw's `DMACON = $8400` write; later ones drift with the
  display's contention, which depends on the draw's absolute start (T54).
- The engine's draw ends were within ±8 lines of the original's (outliers 50), enough for most
  frames but not near a frame's end.
- The instruction letters per addressing mode follow Moira: e.g. `move.l d(An),d(An)` `prrpwwp`,
  `clr.l d(An)` `prrpww`, `addq.l #8,d(An)` `prrpww`, `move.w (An,Dn),Dn` `nprp`, `lsr.l #4`
  `p`+6 idle, `mulu` `p`+17+ones idle, `btst #14,DMACONR` `ppprp`.

## Interface

`sim::Prog`, built by `bobs::draw`.

## Files

`front/story/` (`bobs.rs`, `sim.rs`, `intro.rs`, `clock.rs`).

## Tests

- [ ] The done-when's BLTSIZE check, kept as a test or a tool.

## Done when

- [ ] Given the original's draw start, every BLTSIZE write of a draw in scenes 1 and 4 lands on
  the original's cycle (story_dbg `DRAWSIM`-style check, kept as a test or a tool).
