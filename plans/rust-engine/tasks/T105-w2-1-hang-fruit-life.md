# T105 — w2-1-hang-fruit-life

**Phase:** 2 · **Depends on:** T85; blocks T21 · **Weight:** medium

Added by T85's review (owner, 2026-10-14). T85's `items-upper-w2-1` was planned to the fruit 48,22
and the extra life 63,35, but against the original it took neither: from about frame 1012 the
original never finished another pass to the run's end (1 480). Its frog stayed at x 1231, y 633
(cell 78,42); the PC at frame ends was in `$c0f39e..$c0f3b2` (h0+$e3fe..$e412); the level clock kept
ticking. The harness counts each such frame as a lag (`sync::mid_pass`) and compares nothing, so the
case was green. T85 cut the case to 1 010 frames. The old 1 480-frame script is in commit `4789f2e`
(`engine/conform/scripts/items-upper-w2-1.txt`).

## Goal

1. **Is the hang the original game's own?** Find what the loop at h0+$e3fe..$e412 is and why it
   does not end with the frog at 78,42. Rule out the oracle and the test set-up: the enemy freeze
   (`Freeze::Enemies`, `sf-conform/src/freeze.rs`), the transplanted sync point, the route.
   - **The game's own:** a DESIGN §2.11 case. Reproduce it up to the point of failure, replace it
     with the safest nearby behaviour, and list it in §2.11 with its address for the owner.
   - **An artefact:** fix the oracle, the freeze or the route, whichever it is.
2. **Cover the fruit 48,22 (20 000 points, handler h0+$f008) and the extra life 63,35** in a W2-1
   case that the original plays through, with `resync = false`: lengthen `items-upper-w2-1`, or add
   a case.
3. **Check in the case run, not on the engine alone.** `examples/plan` steps the engine without
   the original's pacing: on this route, the case run bounced off the pad 91,35 two frames later
   than the plan, with the frog record's `+$22` counter two steps out of phase. So the inputs that
   follow miss. Read score, lives and the item cells through `sf_conform::run_with`. Show that the
   items are taken and that the frames that take them are not straddled.

## Files

`engine/conform/{cases,scripts}/items-upper-w2-1.*` (or a new case);
`engine/crates/sf-conform/tests/frog_cases.rs`; `plans/rust-engine/DESIGN.md` §2.11 if the hang is
the game's own; the engine or `sf-conform` where the fix lands.

## Done when

- [ ] The hang's cause is found and recorded (FINDINGS.md): the game's own, with its address and
      §2.11 entry and the engine's replacement, or an artefact, fixed.
- [ ] A W2-1 case is green with no re-sync: the fruit's 20 000 points and the extra life are taken,
      and score and lives are equal on the frames that take them (not straddled).
- [ ] `items-switch-w2-1` and `items-upper-w2-1` still green.
