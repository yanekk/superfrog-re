# T49 — translate-objects

**Phase:** 5 · **Depends on:** T21, T38, T40 · **Weight:** medium

## Goal
Translate the object routines T21 finds through the dispatch tables, which T34's call-tree scan cannot see and so missed T40's batch, before T22, T24 and T25 port them (owner, 2026-11-05, plan read-back decision 2).
## Design sections this implements
§4.3.
## Reuses
T61's `generate.py` for each generator first (owner, 2026-11-05 mid-run amendment), the builder reading and committing what it returns and writing by hand only what it leaves `partial` or `fail`; T21's `re/translate.toml` entries and `docs/10-object-system.md`, T38's generators as pattern, T32's `Gen` and rig, T33's service, T40's `translate_batch.py` and table.
## Files
`fd-conform/src/gen/objects.rs` (appends), `fd-conform/tests/gen_objects.rs` (appends), `games/forest-dumb/re/translate.toml` (`gen_task` filled for T21's entries), `games/forest-dumb/docs/11-translation.md` (the table), `smoke.toml`.
## Interface
One `Generator` per entry T21 added, named as there. The batch runs with `translate_batch.py --resume --max-usd 5`, so routines T40 already judged are not paid for again.
## Tests
- [ ] Each new generator runs 2 000 states on the original with no hang, and its `RigReport::paths` is recorded.
- [ ] Each conditional branch in the routine's listing is taken both ways by some state, shown by T60's `Rig::coverage` rather than witness models, or is named with why no state can take it.
## Done when
- [ ] Per generator: written by `generate.py` (`covered` or `partial`) or by hand, with why, in `docs/11-translation.md`.
- [ ] Every entry T21 added has a generator and a result in the table, or is named as unreachable from a generated state with why.
- [ ] Each `fail`, `too-big` and `no-oracle` routine is listed under the porting task that translates it by hand.
- [ ] The run's cost, `generate.py`'s and the batch's, is in FINDINGS (§5.3 `worker` actions).
- [ ] If T21's entries pass DESIGN §4.3's size for one session, the owner is asked to add a task for the rest, split by kind.

## Shared files
`fd-conform/src/gen/objects.rs` is T38's; this task appends only.
