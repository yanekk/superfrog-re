# T207 — chain-routes-w5-w6-final

**Phase:** 2 · **Depends on:** T206; blocks T203 · **Weight:** medium

Added by T202 with the owner's yes (2026-10-16); see T205 for why. Needs T206's chain: the state
at W5-1's start exists only once W1–W4 replay. Carries T202's test.

## Goal

1. Read T205's FINDINGS row on why a standalone route breaks in the chain, and plan with it.
2. Re-plan W5-1..W5-4, the bonus level, W6-1..W6-4 (their first map set: the case runs with df1
   connected) and the final level for the chain, as T205 re-planned W1–W2. The bonus level's leg
   (`chain-bonus-to-w6-1`: fire held to 8111) loses 16 lives with fire held from a fresh start
   (`bonus-end-wb-1`): check the lives and score the chain arrives with, and plan a bonus route
   that ends the level with lives to spare if it does not.
3. Point `whole_game`'s `LEGS` at them and regenerate `game-whole.txt` to `end_seq_load`. Run
   `game-whole` (`run`, `run --alone`), record each divergence in FINDINGS and the run times, and
   set the case's `frames` to end at `end_seq_load`.

## Files

`engine/crates/sf-conform/examples/whole_game.rs`; `engine/conform/scripts/chain-w5-*.txt`,
`chain-wb-1.txt`, `chain-w6-*.txt`, `chain-ww-1.txt`, `game-whole.txt`;
`engine/conform/cases/game-whole.toml`.

## Interface

None new.

## Tests

- [ ] T202's: the assembled script parses and drives the original from power-on to
      `end_seq_load` (`whole_game --check conform/scripts/game-whole.txt`).
- [ ] `game-whole` run faithful and alone to `end_seq_load`; its result recorded.

## Done when

- [ ] The whole game replays on the original; the script and case committed with their first
      divergence (or green) recorded in FINDINGS and PROGRESS.
