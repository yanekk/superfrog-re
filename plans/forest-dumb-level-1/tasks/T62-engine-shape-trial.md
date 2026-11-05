# T62 — engine-shape-trial

**Phase:** 2 · **Depends on:** T08, T09, T33 · **Weight:** medium

Added by the owner (2026-11-05, mid-run amendment): Muse may also write the engine-shaped port, not only the literal one (DESIGN §4.3). A trial first, on the routines T08 and T09 ported (owner, read-back decision: T10 has no engine structures yet, so it cannot be the target). It holds up no task.

## Goal
Find out whether Muse can turn literal ports into engine code on typed state, judged by the porting rig, for less Opus than a builder spends doing it. Measure on T08's and T09's routines, whose engine code and rig tests exist, so Muse's answer and its cost set directly against what Opus wrote and spent. Muse's passing ports are measurements only; nothing in `fd-core` is replaced. The result is a recommendation to the owner for T22, T24, T25 and later porting; nothing changes for any task without the owner's yes.
## Design sections this implements
§4.3.
## Reuses
T33's `translate.py` as the base (call, retries, spend guard), T32's rig with T08's and T09's `port` closures and rig tests as the judge, T08's and T09's engine code as the target shape, T40's literal ports, the conversation logs in the main checkout's `plans/forest-dumb-level-1/.parallel/control/conversations/` (not the task's worktree; every `T08-*`, `T09-*` and `T51-*` session, restarts included) for their Opus cost.
## Files
`games/forest-dumb/tools/reshape.py`, `games/forest-dumb/tools/test_reshape.py`, `games/forest-dumb/docs/11-translation.md` (the trial and its numbers), `smoke.toml` (`test_reshape.py`), FINDINGS.
## Interface
```
python3 -I games/forest-dumb/tools/reshape.py 'h0+$4274' --task T08 [--tries 3] [--max-usd 0.10] [--out build/reshape]
→ build/reshape/h0-4274/port.rs, and one result line like T33's (pass|fail|no-key, tries, usd)
```
Leave one out: for each routine the prompt holds its literal port, the typed state of its subsystem (the structs and fields only, not whole files) with the routine's own engine function removed, and one other routine's engine-shaped port and rig test from the same task as the example. The judge builds the answer into a scratch copy of `fd-core` in place of the existing function and runs that routine's existing rig test on 10 000 states with its generator. The committed `fd-core` is never changed.
## Tests
- [ ] Model calls stubbed: a canned answer that fails the rig gets its first failing case back; one that does not compile gets the errors.
- [ ] The prompt for a routine never contains that routine's own engine function or rig test body (checked on the built prompt).
## Done when
- [ ] Every T08 and T09 routine with a passing literal port and an engine port was tried; per routine: pass or fail, tries, Muse cost.
- [ ] The Opus cost per ported routine of T08 and T09 (their implement and review sessions, from the conversation logs, divided by routines ported) is set against Muse's, in `docs/11-translation.md`.
- [ ] The owner is asked whether T22, T24, T25 and later porting tasks use `reshape.py`, with the numbers and a recommendation; until then, nothing else in the plan changes.
