# T30 — acceptance

**Phase:** 3 · **Depends on:** T29 · **Weight:** medium

## Goal

Close the plan: a final check that the success criteria in DESIGN §1 hold on the merged tree, and the
owner's own play session with the 8BitDo pad, which is the one judgement no tool can make.

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
- [ ] The owner's verdict below is recorded as a dated ✅ (or 🐞 with what to fix) row in FINDINGS.

## Needs a person

```
cd engine && cargo run --release -p sf-app
```
(built and imported by the worker beforehand; quit with ⌘Q)

Expect: the game from the Team 17 logo onward, played with the 8BitDo Ultimate 2 and the speakers on,
for as long as you like (at least one world, ideally with a slot machine and a boss).
Tell me: whether it is Superfrog as you remember it, and anything that felt or sounded off, with the
level and roughly what you were doing.
