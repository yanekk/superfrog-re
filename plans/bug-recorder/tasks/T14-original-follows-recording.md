# T14 — original-follows-recording

**Phase:** 3 · **Depends on:** T05, T12 · **Blocks:** T13 · **Weight:** heavy

Added by T12 with the owner's yes (2026-10-20): both of T12's dry runs failed at `bug compare`.

## Goal

Make `sf-conform bug compare` judge the owner's game, not wherever the original drifts to. On a
really played W1-1 (the app's recording, not a forced exit) the original lags, leaves the recording
at frame 1815 (`score original 209e, recording 2102`), and by STOP shows a score-0 W1-1 start
(lives lost, seemingly the attract demo). Today the tool then:

- says `same` (the legs re-sync the engine from the original, so they judge the engine along the
  original's drifted path), and saves that `original-stop.png`; a quirk
  report built on it would be false (dry run 1);
- panics at `bug.rs` `the original's load (Left(c010d6)) was not over within 20000 frames of the
  engine's (step 4695)` when the engine loads and the original is elsewhere (dry run 2).

So a bug after the tally, the slot machine or the next load (T13's black screen among them) cannot
be found on a played W1-1. This task keeps the original on the recording, and makes the tool say
plainly when it could not.

## Design sections this implements

DESIGN §1 success criteria ("stays in step through a level's end, the tally, the slot machine and
the next level's load"), §2.8, §2.9, §2.10 step 2–4.

## Approach (the worker's to settle; the owner approved the direction, not the mechanism)

When the original leaves the recording (T05's `Left`), put it back on it: write the engine's
modelled state (`LAYOUT`, at a pass end in a level, where both sides are at a clean point) into the
oracle's memory, as T03 writes the start's leftovers into the original, and go on judging from
there. Then each leg says whether the original, from the owner's state, does what the engine did.
Measure first whether a written-back original runs on (custom chips, the CPU's place and stack are
not in `LAYOUT`); if it cannot, say so and bring the finding to the owner before building another
mechanism. Whatever the mechanism, the tool never prints `same` over frames where the original was
not on the recording, and never panics: it reports which stretch it could not judge and why.

## Files

- `engine/crates/sf-conform/src/bug.rs` and whatever of `sync.rs` / `lib.rs` the write-back needs
- `engine/crates/sf-conform/tests/recorder_compare.rs` (or a new `recorder_*.rs`)
- `engine/conform/README.md` § Comparing a bug zip: the new behaviour and report lines
- `docs/BUG-FIX.md` steps 2–3, if the report's wording changes

## Tests

- [ ] T12's dry run 1, re-run: a W1-1 recording played to the exit, the slot machine and W1-2,
  made with the app (`SF_E2E_KEEP=1 cargo test --release -p sf-e2e --test recorder_drill --
  --ignored w1_1_played`, the zip in the kept home's `test-bugs/`). `bug compare` says `same` and
  `original-stop.png` shows W1-2 where `engine-stop.png` does, or reports the stretch it could not
  judge; never `same` with the original elsewhere
- [ ] T12's dry run 2, re-run: on a scratch branch never merged, `level/tally.rs`'s points choice
  planted to give `pages * PAGE_POINTS * 2`; W1-1 by `sf-e2e/tests/data/w1-1-exit-engine.txt`
  (seed 0 0, dialog defaults), `tally-points` from the exit, 300 frames of W1-2, saved as a zip.
  `bug compare` on the planted engine differs at the tally naming `score`; `bug case` makes a case
  that fails on the planted engine and passes on the clean one. Branch deleted; FINDINGS row
- [ ] an automated test in the block or ignored with its time: a recording on which the original
  leaves, compared, is put back (or reported), with no panic

## Done when

- [ ] both dry runs reach the step `docs/BUG-FIX.md` expects (dry run 1 step 3, dry run 2 step 5),
  recorded in FINDINGS
- [ ] `bug compare` never panics on a recording the original leaves, and never says `same` for
  frames it did not judge on the recording
- [ ] the test command is green
