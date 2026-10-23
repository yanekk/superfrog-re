# T12 — fix-procedure

**Phase:** 3 · **Depends on:** T05, T11 · **Weight:** light

## Goal

Write the procedure an `@repo/single fix the bug in <zip>` session follows, where a fresh session
finds it, and prove it works by running it on two recordings whose answer is known; and update the
README for the whole feature.

## Design sections this implements

DESIGN §2.10, §1 success criteria.

## Files

- `docs/BUG-FIX.md` (new): DESIGN §2.10 as steps with the exact commands, the report wording for a quirk, the `Escape:` trailer rule
- `CLAUDE.md`: one line under "What's known" pointing at it, worded so a session given a bug zip opens it
- `README.md`: Start Level, the recorder, the bug zip, `--replay`, and how to hand a bug to an agent
- `engine/conform/README.md`: link to the procedure

## Tests

- [ ] dry run 1 (no bug): a recording through W1-1's exit and the slot machine made with the app (`sf-e2e` script layer). Following the procedure ends at step 3 with the "same as the original" report
- [ ] dry run 2 (a planted bug): on a scratch branch never merged, an engine change that makes the tally give wrong points; a recording made on it. Following the procedure finds the planted frame and variable at step 4 and makes a case that fails there and passes on the clean engine. The branch is deleted, and the transcript is summarised in FINDINGS

## Done when

- [ ] `docs/BUG-FIX.md` exists; `CLAUDE.md` and `README.md` point at it and describe the feature
- [ ] both dry runs reach the expected step, recorded in FINDINGS
- [ ] the test command is green
