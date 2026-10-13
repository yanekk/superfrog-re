# T36 — items-on-the-beam

**Phase:** 2 · **Depends on:** T15 · **Weight:** heavy

Added by T15 (owner, 2026-10-13: "split the rest of the known work into tasks"). T15 ported the
object core and every non-enemy kind, proven routine by routine; its frame cases still run the frog
alone (`objects` freeze), and `items-run-w1-1` compares variables and Paula only. What is left
before items are frame-exact in the picture, and in `Start::Level`.

Earlier work, not merged and unproven, is on the local branch `t15-flow-wip` (commit after
`da2f926`): `level/bobs.rs`, `spawn::scan_screen`, the bob sheet loaded at entry, the sync-key fix.

## Goal

Items and their pickups frame-exact in the picture, and every level entered with its objects.

1. **The pass on the beam.** In a level's first pass the bobs are drawn into the buffer being shown
   (measured on W1-1: blits on lines 114–182, coins below line 245 appear that frame), and a taken
   pickup's tile is redrawn into both buffers while the beam may be below it. Give the pass a CPU
   time model (`LevelCtx::spend`, calibrated on the oracle's INTENA/DMACON beam positions) and draw
   the picture with rows below each write's line showing it.
2. **Lag.** With objects live the original's pass overruns (86 of 700 frames in `items-run-w1-1`,
   re-synced by the runner). Model `Pacing::Late` (DESIGN §2.1's fixed point; T14 FINDINGS) so
   item cases need no re-sync.
3. **`Start::Level` with objects**: the world's `LnBO` into hunk 3, `h0+$1995a` in full (frame
   records, blit parameters, sub-records, the mask), `map_spawn_scan_screen`; remove the objects
   group from `level_entry.rs`'s `NOT_MODELLED`.
4. **Frame cases** (`enemies` freeze): collect every pickup type at least once (W1–W6 tables,
   h0+$f382), each power-up active until it ends (spud thrown, wings, speed, shield), the hidden
   coin spring, switches, pads, launchers; score, lives and clock equal every frame.

## Interface

`LevelCtx::spend(cycles)`, the pass's CPU time on the beam; `Scene::step` honouring
`Pacing::Late`; `Start::Level` entering with the level's objects spawned.

## Tests

- [ ] routine conformance for the time model's calibration points and `map_spawn_scan_screen`.
- [ ] frame conformance, picture included: the item cases of goal 4, with no re-sync.
- [ ] `level_entry.rs` with the objects group gone from `NOT_MODELLED`.

## Files

- `engine/crates/sf-core/src/level/{objects.rs,frame.rs,timeline.rs,bobs.rs,load.rs}`, `scene.rs`
- `engine/conform/{scripts,cases}/items-*`; `engine/crates/sf-conform/tests/{frog_cases,level_entry}.rs`

## Done when

- [ ] The item cases compare the picture, green without re-syncs; one in the test block.
- [ ] `level_entry.rs` excuses no object variable for W1–W6.
