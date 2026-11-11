# T21 — default-check

**Phase:** 3 · **Depends on:** T20 · **Weight:** light

## Goal

Prove by machine that a fresh session reaches for the translator unprompted (brief, stage 7): give fresh `claude -p` sessions a porting-style task and only the repo's docs, and check from their transcripts that the first thing they run to turn code into Rust is `tools/translate.py`.

## Design sections this implements

DESIGN §2.12, §5.2, §5.3.

## Files

- `tools/default_check.py`, `tools/test_default_check.py` (new).

## Interface

```
default_check.py [--runs 3] [--max-turns 15] [--minutes 10] [--routine <symbol>]
# each run: a scratch worktree of HEAD under .claude/worktrees/, claude -p with the default model,
#   --output-format stream-json, tools allowed: Read, Grep, Glob, Bash(python3 -I games/forest-dumb/tools/*)
#   prompt: "Port <routine> of Forest Dumb Forever to Rust for a level plan. Start the way this repo's docs say."
# pass per run: the first Bash call that produces Rust from the original runs tools/translate.py,
#   and no Write of a .rs file precedes it
# {"runs":3,"pass":3,"per_run":[{"first_tool":"...","translate_at_turn":n}]}
```

The scratch worktree is removed after each run, pass or fail.

## Outside actions

- Run the stage 7 check — `ask`.

## Tests

- [ ] On recorded fake transcripts: translate first passes; writing Rust by hand first fails; reading the listing then translate passes.

## Done when

- [ ] 3 runs of 3 pass; the summary is in FINDINGS and `docs/14-translator.md`.
- [ ] A failure is fixed in the docs T20 wrote (never in the prompt), then rerun.
