# T21 — object-meanings

**Phase:** 5 · **Depends on:** T05, T06 · **Weight:** medium

## Goal
Measure what each object value in `W1-1` does, listing the enemies, collectibles and mechanics present, so T22, T24 and T25 start from facts. The object maps are grouped today, not understood.
## Design sections this implements
§2.2; `docs/05-file-formats.md`.
## Reuses
`fd-conform` routines and oracle, `render_levels.py`'s object boxes, Ghidra MCP, `tools/re_listing.py`.
## Files
`games/forest-dumb/docs/10-object-system.md`, `re/symbols.toml`, `docs/05-file-formats.md`, `fd-conform/src/freeze.rs` (the entry points), `fd-conform/tests/object_claims.rs`, `smoke.toml`.
## Interface
A table per value used in `W1-1`: count, location example, group, entry routine, what it does, seen how. A list of mechanics, each with a spot such as `W1-1:10,20`. Freeze entry points for §2.2.
## Tests
- [ ] Every non-zero value in `_MapaBobow1`'s used columns has a row (test on real data).
- [ ] Claimed entry points are reached on the oracle when a scripted run reaches a spot, shown by T31's counter.
- [ ] Every value `W1-1`'s object map can spawn is in a committed list with the task that ports it (T22, T24 or T25); `fd-conform/tests/ported.rs` fails on a listed value that has neither a `PORTED` entry nor an open task.
## Done when
- [ ] Every value is measured or named as unreached with why.
- [ ] If the list exceeds T24 or T25's one-session scope, the owner is asked to add tasks before they start.
