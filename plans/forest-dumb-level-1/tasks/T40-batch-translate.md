# T40 — batch-translate

**Phase:** 1 · **Depends on:** T35, T36, T37, T38, T39 · **Weight:** light

## Goal
Run the translation service over `game_frame`'s call tree in one unattended run, before the stages build on it, and leave a table that says, per routine, whether a passing port is waiting for its porting task (owner, 2026-11-05).
## Design sections this implements
§4.3, §5.2, §5.3.
## Reuses
T33's service, T34's `re/translate.toml`, T35 to T39's generators, the spike's `summary.py` as pattern.
## Files
`games/forest-dumb/tools/translate_batch.py`, `games/forest-dumb/tools/test_translate_batch.py`, `games/forest-dumb/docs/11-translation.md` (the table), `smoke.toml`.
## Interface
```
python3 -I games/forest-dumb/tools/translate_batch.py --max-usd 5 [--parallel 8] [--resume]
```
The table, committed: routine, result (`pass`, `fail`, `too-big`, `no-gen`), port path under `build/translate/`, generator, paths, tries, cost. The ports stay in `build/`, which is shared by the checkout's worktrees only if the builder copies them; a porting task re-runs `translate.py` on one routine when its checkout has no port.
## Tests
- [ ] `--resume` redoes only unfinished routines (stubbed calls).
- [ ] The cap stops the run, and the table names what was not attempted.
## Done when
- [ ] The run finished under its cap, started through the `ask` row of §5.3; its cost and duration are in FINDINGS.
- [ ] Each `fail` and `too-big` routine is listed under the porting task that translates it by hand.
