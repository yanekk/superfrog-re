# T66 — reshape-decision

**Phase:** 2 · **Depends on:** T62 · **Blocks:** T22 · **Weight:** small

Added by T62 on the coordinator agent's instruction (2026-11-05): T62's last "Done when" item asks the owner, who was asleep, so the question was set aside here. `blocks T22` so the owner answers before the first porting task that would use the tool.

## Goal
Get the owner's answer to the open question: **do T22, T24, T25 and later porting tasks use `games/forest-dumb/tools/reshape.py`** (Muse writes the engine-shaped port from the literal one, judged by the task's own rig test), and record it. Until the owner answers, no task changes.

What T62 measured (`games/forest-dumb/docs/11-translation.md`, "The engine-shape trial"):
- 15 of 15 units, 21 routines of T08 and T09, passed the rig (10 000 states a routine) on Muse's first answer: $0.0324 and 800 s in all, $0.0015 a routine.
- Opus's whole-task cost per routine ported: T08 $0.89 (14), T09 $0.97 (10), T51 $1.56 (6). That covers notes, symbols, rig tests and mutants, docs and review as well as the reshaping; the reshaping share is not measured.
- Muse's answers are on typed state but less factored: 717 lines against Opus's 441; four near-identical arms where Opus wrote one, repeated slide code, bare offsets where the file has named constants, no doc comments.

T62's recommendation: **Muse drafts, the builder tidies.** The builder writes the rig test first, calls `reshape.py` per routine, then tidies the passing answer (names, merged repeats, doc comments) and re-proves it on the rig; by hand only after three failed tries. Needed for that: a `TASKS` entry in `reshape.py` per porting task (source file, `impl`, rig test, its loop line, harness items, and the unit list: each engine function, its routines and its exclusive helpers).

The other answers offered: keep builders writing engine code as T08 and T09 did; or commit Muse's passing code untidied.
## Design sections this implements
§4.3.
## Reuses
T62's `reshape.py`, its table in docs/11, `--opus-cost`.
## Files
`plans/forest-dumb-level-1/FINDINGS.md` (the answer, dated), `games/forest-dumb/docs/11-translation.md` (how porting tasks use the tool, if they do). Editing T22's, T24's or T25's task docs follows the owner's answer and is the owner's change to the plan.
## Interface
None.
## Tests
None: a decision.
## Done when
- [ ] The owner has answered, in their words, and the answer is in `FINDINGS.md` with the date.
- [ ] If the answer is to use `reshape.py`, docs/11 says how a porting task uses it, and what a `TASKS` entry needs.
