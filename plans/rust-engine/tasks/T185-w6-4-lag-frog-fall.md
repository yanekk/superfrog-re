# T185 — w6-4-lag-frog-fall

**Phase:** 2 · **Depends on:** T182; blocks T21 · **Weight:** medium

Added by T181 with the owner's yes, 2026-10-15 (DESIGN §2.1 checks 1 and 3).

## Goal

`enemies-w6-4` green under the faithful checks with `resync = false`, as it is alone: the engine's
frog falls through the original's long lagging stretch (frames 1052–1119) as the original's does.

Measured by T181 (2026-10-15, `sf-conform scan conform/cases/enemies-w6-4.toml`), only two frames
differ, and the runs agree again after them:

- 1120 (on time, the frame after 68 frames ending mid-pass): `frog_y` original `$192`, engine `$18f`.
  The frog is falling 3–4 px a frame, so the engine is about one frog step behind.
- 1123: channel 3's volume 30, which the original sets at 1120 (the landing), has no match in the
  engine within ±2 frames.

With `resync = true` the case re-syncs at 1120 and still fails at 1123 on that sound. Alone
(check 4) it is green. Suspected, not shown: frame 1115's late pass ended with its last Paula write
on line 0 (`Pacing::Late { writes: 73, line: 0, hpos: 13, .. }`), and the engine merged or skipped
that frame's vertical blank (`set_blank_merged`, `sync::handlers`), so its frog work (50 Hz, in the
vertical blank) ran once fewer than the original's. Find the frame the frog's step is lost, then
the cause.

## Design sections this implements

DESIGN §2.1 checks 1 and 3.

## Files

- `engine/crates/sf-core/src/level/{frame.rs,mod.rs}` (the vertical blank in a late pass) and
  `engine/crates/sf-conform/src/{lib.rs,sync.rs}` (`handlers`, `pacing`) as the cause needs.
- `engine/conform/cases/enemies-w6-4.toml` (its comment), `engine/crates/sf-conform/tests/enemies_w6_cases.rs`.

## Interface

None new.

## Tests

- [ ] `enemies-w6-4` green (`sf-conform run`), with a test in `enemies_w6_cases.rs` (`green("enemies-w6-4")`).
- [ ] A test that fails without the fix: the frame the frog's step was lost, `frog_y` equal there.
- [ ] `enemies-w6-4-alt` and the other `enemies-w6-*` tests stay green.

## Done when

- [ ] The above; or, if the cause turns out to be one T184 fixes, the case green after T184 and
      this task closed with a FINDINGS row saying so.
