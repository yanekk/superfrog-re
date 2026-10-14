# T128 — w3-4-sprite-race

**Phase:** 2 · **Depends on:** T127; blocks T113 · **Weight:** light

Added by T102 (owner, 2026-10-14). T113 waited on T102 for W3-4 frame 194, "the beam race T102
owns". T102 fixed `items-shield-w3-1` (its 366 was already green after T101), but W3-4 194 is still
red, and it is not a bob: the frog's hardware sprites show only in part.

## Goal

1. **Start by re-running W3-4 frame 194 once T127 has landed** (`enemies-w3-4.toml` with
   `frames` raised past 193, run locally). If it is green, this task closes at once: say so in
   `PROGRESS.md` and `FINDINGS.md`.
2. Otherwise find and fix it. On T102's base the frame is on time, every variable equal; the
   picture differs on rows 117–158 (292 pixels, the frog jumping off the first crumbling platform
   at 289,49), the original showing only the frog's lower part. The original's `place_frog_sprites`
   runs on lines 28–35 that frame (`examples/profile enemies-w3-4.toml 193 195 top`), across line
   `$20`, where the copper fetches the frog's sprite pointers: T127's wait (h0+$19c9c) is the first
   suspect.
3. Raise `enemies-w3-4`'s `frames` past 194, so T113 starts from a case green beyond it.

## Files

`engine/crates/sf-core/src/frog/anim.rs`; `engine/crates/sf-core/src/level/{mod.rs,beam.rs}`;
`engine/conform/cases/enemies-w3-4.toml`; `engine/crates/sf-conform/tests/enemies_w3.rs`.

## Interface

Nothing new expected: the fix lands in the existing frog-sprite placement and beam
(`place_frog_sprites`, `LevelCtx::sprite_wait`).

## Tests

- `enemies-w3-4` green through frame 194 and on to where its script ends (or the next fault,
  recorded), its re-syncs pinned in `enemies_w3.rs` as now.
- Every other level case stays green.

## Done when

- [ ] W3-4 frame 194 matches the original, with `enemies-w3-4` taken past it in the test block.
- [ ] The other level cases stay green.
