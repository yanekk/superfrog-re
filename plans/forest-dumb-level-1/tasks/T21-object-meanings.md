# T21 — object-meanings

**Phase:** 5 · **Depends on:** T05, T06, T32 · **Weight:** medium

## Goal
Measure what each object value in `W1-1` does, listing the enemies, collectibles and mechanics present, so T22, T24 and T25 start from facts. The object maps are grouped today, not understood.
## Design sections this implements
§2.2; `docs/05-file-formats.md`.
## Reuses
`toolkit/docs/porting-routines.md` §1 and §2; `fd-conform` routines and oracle, `render_levels.py`'s object boxes, Ghidra MCP, `tools/re_listing.py`.
## Files
`games/forest-dumb/docs/10-object-system.md`, `re/symbols.toml`, `docs/05-file-formats.md`, `fd-conform/src/freeze.rs` (the entry points), `fd-conform/tests/object_claims.rs`, `games/forest-dumb/re/objects-W1-1.toml` (the spawnable values and their porting task), `fd-conform/tests/ported.rs` (T32's; appends its test), `re/translate.toml` (T34's; appends the object routines it finds), `smoke.toml`.
## Interface
A table per value used in `W1-1`: count, location example, group, entry routine, what it does, seen how. A list of mechanics, each with a spot such as `W1-1:10,20`. Freeze entry points for §2.2.
## Tests
- [ ] Every non-zero value in `_MapaBobow1`'s used columns has a row (test on real data).
- [ ] Claimed entry points are reached on the oracle when a scripted run reaches a spot, shown by T31's counter.
- [ ] Every value `W1-1`'s object map can spawn is in `re/objects-W1-1.toml` with the task that ports it (T22, T24 or T25); `fd-conform/tests/ported.rs` fails on a spawnable value missing from the list, and reports which listed values T07's dispatch has no `ObjectKind` for yet.
## Done when
- [ ] Every value is measured or named as unreached with why.
- [ ] Each spawnable value's routines are in `re/translate.toml` with T22, T24 or T25 as `port_task`. If T24's or T25's share passes DESIGN §4.3's size, the owner is asked to add tasks, split by kind, before they start.
