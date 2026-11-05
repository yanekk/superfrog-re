# T62 — engine-shape-trial

**Phase:** 2 · **Depends on:** T08, T09, T33 · **Weight:** medium

Added by the owner (2026-11-05, mid-run amendment): Muse may also write the engine-shaped port, not only the literal one (DESIGN §4.3). A trial first, on the numbers T08 and T09 leave. T10 waits for it (`blocks T10`).

## Goal
Find out whether Muse can turn literal ports into engine code on typed state, judged by the porting rig, for less Opus than a builder spends doing it. Measure on T10's routines, so a pass is used by T10 and nothing is thrown away. The result is a recommendation to the owner; nothing changes for later tasks without the owner's yes.
## Design sections this implements
§4.3.
## Reuses
T33's `translate.py` as the base (call, retries, spend guard), T32's rig with the porting task's `port` closure as the judge, T08's and T09's engine code and their rig tests as the prompt's example of the shape, T40's literal ports for T10's routines, the conversation logs in `plans/forest-dumb-level-1/.parallel/control/conversations/` for T08's and T09's Opus cost.
## Files
`games/forest-dumb/tools/reshape.py`, `games/forest-dumb/tools/test_reshape.py`, `games/forest-dumb/docs/11-translation.md` (the trial and its numbers), FINDINGS.
## Interface
```
python3 -I games/forest-dumb/tools/reshape.py 'h0+$9754' --task T10 [--tries 3] [--max-usd 0.10] [--out build/reshape]
→ build/reshape/h0-9754/{port.rs,rig_test.rs}, and one result line like T33's (pass|fail|no-key, tries, usd)
```
The prompt holds: the literal port, the typed state the subsystem's engine code uses (the structs and fields only, not whole files), one engine-shaped example and its rig test from T08 or T09. The judge builds the answer into a scratch copy of `fd-core` and runs the rig test on 10 000 states with the routine's generator.
## Tests
- [ ] Model calls stubbed: a canned answer that fails the rig gets its first failing case back; one that does not compile gets the errors.
## Done when
- [ ] Every T10 routine with a passing literal port was tried; per routine: pass or fail, tries, Muse cost.
- [ ] The Opus cost per ported routine of T08 and T09 (their implement and review sessions, from the conversation logs, divided by routines ported) is set against Muse's, in `docs/11-translation.md`.
- [ ] The owner is asked whether later porting tasks use `reshape.py`, with the numbers and a recommendation; until then, nothing else in the plan changes. Passing ports wait in `build/reshape/` for T10's builder, who may use or ignore them.
