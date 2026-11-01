# T15 — recording-compare

**Phase:** 3 · **Depends on:** T03, T14 · **Weight:** medium

## Goal
Play a recording on the original from the matching start and compare, the step that lets a session tell "ours" from "the original does it too", and write the game's bug procedure.
## Design sections this implements
§2.8; `toolkit/PLAYBOOK.md` step 8.
## Reuses
`sf-conform/src/bug.rs` (`retime_played`, `compare_inner`) as pattern; game-free parts move to `amiga-conform`, because this is the second user. `games/superfrog/docs/BUG-FIX.md`.
## Files
`fd-conform/src/bug.rs`, `toolkit/crates/amiga-conform/src/*`, `games/forest-dumb/docs/BUG-FIX.md`, `games/forest-dumb/CLAUDE.md`, `fd-conform/tests/recording_compare.rs`, `smoke.toml`.
## Interface
```
fd-conform bug compare <zip>   // same | differs at frame N, values, both pictures in build/
```
## Tests
- [ ] A scripted recording stays in step on the original.
- [ ] A lag-free stretch stays in step; lagged input is measured and its result recorded.
- [ ] An engine mutation is reported as different at the expected frame.
## Done when
- [ ] `docs/BUG-FIX.md` states the procedure, and CLAUDE.md links it.
- [ ] The result under lag is in FINDINGS, including any weakness of "same".
