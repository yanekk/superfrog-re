# T24 — enemies

**Phase:** 5 · **Depends on:** T20, T21, T22, T23 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T24` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again. A reviewer's pack has no FINDINGS: read it after your cold read.

## Goal
Port level 1's enemies, movement, drawing and hurting the character: the `objects` routines `re/translate.toml` gives T24. Most arrive as passing literal ports from T40. Agreement over runs is T46's (DESIGN §4.3).
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T40's and T49's ports and table, T33's service for a missing or failed port, T32's rig and T38's generators, T07's dispatch, T22's scan, T21's table, `$abc` and `$3a18` readings, `fd-assets`' `StworkiNew`.
## Files
`fd-core/src/objects/creature*.rs`, `fd-core/src/ported.rs` (appends), `fd-core/src/render/objects.rs`, `fd-conform/tests/enemy_routines.rs`, `fd-plan/src/game.rs`, `smoke.toml`.
## Interface
```
impl ObjectKind for Creature { .. }   // one per kind T21 lists, value 1 to 6
```
Random-number use follows T00; seed in state.
## Tests
- [ ] Routine conformance on 10 000 states for each kind.
- [ ] Planner's danger avoids them on the engine.
## Done when
- [ ] Every T24 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it. Non-zero hits on the original are shown by T46.
- [ ] The routines translated by hand, and why, are listed in `docs/11-translation.md`.
- [ ] Engine functions written by Muse through `tools/reshape.py --task T24 --all` and a `[T24]` table in `re/reshape.toml`, after the typed state, the rig test, two example units and stubs by hand (`docs/11-translation.md`, "Reshaping by Muse"). docs/11's "Ported" line names Muse's functions, its spend, and any written by hand with why.

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
