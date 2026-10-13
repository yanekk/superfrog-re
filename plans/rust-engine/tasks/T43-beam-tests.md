**Split from:** T36 — carries "green without re-syncs" (made a failure) and the Tests list's routine conformance for `map_spawn_scan_screen` and the time model.

# T43 — beam-tests

**Phase:** 2 · **Depends on:** T42 · **Weight:** light

Added by T36 (owner, 2026-10-13). T36's tests were written and discarded at the stop; rewrite them.

## Goal

1. `sf-conform/tests/items_routines.rs`: `map_spawn_scan_screen` (h0+$1777c) against the original on
   the routine rig: 300 states, camera anywhere in W1-1, some slots freed first, the markers with no
   spawner frozen by poking `$4e75 $4e71` over their `obj_flag_jumptab` stubs
   (`freeze::frozen_markers`). Passed in T36's session.
2. `sf-conform/tests/beam.rs`: the cost model free-running (no anchors) over `items-run-w1-1`, frames
   not mid-pass and not just after: median write error ≤ 1 line, ≥ 70 % within 3, ≥ 98 % within 10
   (T36 measured 72 % / 99 %). Re-sync the engine from the original when vars differ, as the runner.
3. Runner: a case key (e.g. `resync = false`) making a re-sync a failure; set it on the `items-*`
   cases; `frog_cases.rs` runs `items-fruit-w2-2` too (the test block's item case is
   `items-run-w1-1`, already there; add `compare` with `picture` to its case file).
4. Optional, if cheap: compare the picture and Paula on straddled frames and the frame after
   (T36 measured Paula equal on all of them, pictures on all but 2–4 per case).

## Done when

- [ ] Both tests green; `items-run-w1-1.toml` compares vars, picture and paula with no re-sync.
