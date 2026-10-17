# T209 — whole-game-alone

**Split from:** T203 (its done-when "Green in `sf-conform all`; run time in PROGRESS notes" and its test `game_whole` green faithful and alone)

**Phase:** 2 · **Depends on:** T203, T208; blocks T23, T29 · **Weight:** medium

Added by T203 with the owner's yes (2026-10-17). Carries T203's done-when: `game-whole` green.

## Goal

1. **Check 4.** `run --alone conform/cases/game-whole.toml` is red at play frame 43522: the
   original's coin (92 left for the exit) is taken by the engine alone 59 frames later (43581)
   (FINDINGS, T203). It was red at 15053 before T203's W1-2 fix. Find the first frame at which the
   engine alone ends a pass with a gameplay variable unlike the original's, and fix the cause.
   Do not re-run the whole game for each try: cut a sync point shortly before play frame 43522 from
   the alone run (the original's snapshot from `whole_game --scratch`, or a new sync point that
   resumes the engine as check 4 runs it), and probe a short case from there (owner, 2026-10-17).
   Each further divergence the alone run meets, the same way.
2. **Green.** `game-whole` green faithful and alone in `sf-conform all`, nothing `unported` on the
   way; its run time in PROGRESS notes; `sf-conform/tests/chain.rs`'s `game_whole` test asserts it
   (`#[ignore]`, named in `conform/README.md` with its time).

Judge by the faithful checks (DESIGN §2.1), not `--strict`; fix what fails a check that matters to
play, accept what is within tolerance (owner, 2026-10-17).

## Files

As the causes need; `engine/crates/sf-conform/tests/chain.rs`; `engine/conform/README.md`; a short
case or sync point for the probe if it earns a place.

## Interface

None new.

## Tests

- [ ] `game_whole` green faithful and alone; no `unported` on the way.

## Done when

- [ ] Green in `sf-conform all`; run time in PROGRESS notes.
