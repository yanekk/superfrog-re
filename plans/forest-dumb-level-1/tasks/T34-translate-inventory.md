# T34 — translate-inventory

**Phase:** 1 · **Depends on:** T00 · **Weight:** medium

## Goal
List every routine in `game_frame`'s call tree, so the batch translation (T40) has a fixed list and every porting task has a known size before it starts. It takes over T00's sizing question: which routines each porting task owns, and whether one task is too much for one session.
## Design sections this implements
§4.3.
## Reuses
The spike's call-tree scan (about 8 300 instructions with callees), `tools/re_listing.py`, T00's note (`docs/09-play-measured.md`), `re/symbols.toml`, Ghidra MCP.
## Files
`games/forest-dumb/re/translate.toml`, `games/forest-dumb/docs/11-translation.md` (inventory section), `games/forest-dumb/tools/call_tree.py`, `games/forest-dumb/tools/test_call_tree.py`, `smoke.toml`.
## Interface
`re/translate.toml`, one entry a routine: `name`, `hunk`, `offset`, `instructions` (with callees), `subsystem` (`character`, `camera`, `drawing`, `objects`, `rest`, `sound`), `gen_task` (T35 to T39), `port_task` (T08, T09, T10, T22, T23, T24, T25, T26 or an added one), `split_at` (the callees it is split at when over 180 instructions). Addresses and sizes only; no listing text.
## Tests
- [ ] Every `bsr`/`jsr` target reachable from `game_frame` has an entry or a reason (`sound`, a Kickstart call, an indirect jump named as such).
- [ ] No entry is over 180 instructions without `split_at`; each split piece is its own entry.
- [ ] `sound` entries are out of scope (the plan is silent) and have no `gen_task`.
## Done when
- [ ] Each porting task's share is summed in `docs/11-translation.md`. Over 1 500 instructions or 15 routines for one task (§4.3) means asking the owner to add sibling tasks, each with `blocks` edges onto the tasks after it; no existing task is edited (root CLAUDE.md: a worker adds tasks, never splits one).
- [ ] The same check, at the same limit, for T35 to T39's generator batches.
- [ ] A result that changes a later task is raised, not patched in.
