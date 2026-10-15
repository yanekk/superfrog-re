# T30 — acceptance

**Phase:** 3 · **Depends on:** T29 · **Weight:** medium

## Goal

Close the plan: a final check that the success criteria in DESIGN §1 hold on the merged tree, all of
them established by the agents headless (owner, 2026-10-15: no play session as a gate).

## Design sections this implements

DESIGN §1 success criteria, §5.1.

## Automated checks (the worker runs these)

```
both test-block lines from a fresh worktree after setup
cd engine && cargo run -q --release -p sf-conform -- all
tools/check_no_disk_data.py
cargo tree -p sf-app | grep -E 'sf-oracle|sf-conform|sf-e2e'   # must print nothing
```

## Files

- `README.md` — status table and a "Play it" section (build, first run, where data lives).
- `CLAUDE.md` — "What's known" and "Next up" updated for the engine and the editor plan.

## Done when

- [ ] Every automated check above passes and its output summary is in PROGRESS notes.
- [ ] README and CLAUDE.md updated.
- [ ] The end-to-end checks below green, their worker-driven FINDINGS row written.

## End to end (the worker drives this, replacing the owner's session; owner, 2026-10-15)

- [ ] The real binary under `--script` (script layer, `--max-frames` and `--timeout` set, the
      original's pacing file as in T29) plays the
      whole-game script from power-on to the end sequence; the captured run's milestones equal the
      original's (DESIGN §2.1 check 4) and its audio capture is non-silent where the original's is.
- [ ] A soak: 50 000 frames of scripted play through the app with no crash, no `unported` entry, no
      frame drift between picture and audio clocks.
- [ ] A dated, worker-driven FINDINGS row with what was run and seen.

The owner may still play it (`cd engine && cargo run --release -p sf-app`); that is optional and not
a gate.
