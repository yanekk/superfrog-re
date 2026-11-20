# T00 — bring-in-level-1

**Phase:** 0 · **Depends on:** — · **Weight:** light

## Goal

The trial measures with level 1's judge, generators and routine inventory, which exist only on `pir/forest-dumb-level-1` (DESIGN §7). This task merges that branch, as it stands today, into this plan's branch and shows the judge still works here, so every later task builds on a known copy.

## Design sections this implements

DESIGN §5.3 (the merge row), §7 (why the merge and why this branch waits for level 1 to reach `main`).

## Files

No source change. The merge commit, plus a one-row baseline in `FINDINGS.md`.

## Tests

- [ ] `cd games/forest-dumb/engine && cargo build -q -p fd-conform --bin fd-judge` succeeds after the merge.
- [ ] `cargo test -q -p fd-conform --test judge` passes.
- [ ] The judge, given Muse's passing port of `h0+$5462` (`FD_PORT=<main checkout>/games/forest-dumb/build/translate/h0-5462/port.rs`), prints `"pass":true` with generator `char_sprite_position`. If that port is absent, regenerate nothing: use any `pass` row of `build/translate/batch.json` that has its `port.rs`, and say which.

## Done when

- [ ] The task branch contains `pir/forest-dumb-level-1` at a named commit, with no conflict left and no level 1 file edited.
- [ ] The three checks above pass, and the commit's SHA and the routine used are in a `FINDINGS.md` row.

## Outside actions

- Bring level 1's branch into the plan branch — `worker`
