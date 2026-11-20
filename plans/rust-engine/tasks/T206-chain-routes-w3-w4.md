# T206 — chain-routes-w3-w4

**Phase:** 2 · **Depends on:** T205; blocks T203 · **Weight:** medium

Added by T202 with the owner's yes (2026-10-16); see T205 for why. Needs T205's chain: the state
at W3-1's start exists only once W1–W2 replay.

## Goal

1. Read T205's FINDINGS row on why a standalone route breaks in the chain, and plan with it.
2. Re-plan W3-1..W3-4 and W4-1..W4-4 for the chain, as T205 re-planned W1–W2: each from the
   original's state at that leg's start (`whole_game --from <leg>`), with T205's snapshot-start
   planning tools, checked on the original chunk by chunk; a leg whose standalone route still
   replays keeps it; new routes in `conform/scripts/chain-<leg>.txt`.
3. Point `whole_game`'s `LEGS` at them, regenerate `game-whole.txt` up to W5-1's first frame (or
   further), run `game-whole` (`run`, `run --alone`), record each divergence in FINDINGS and the
   run times, and set the case's `frames`. The case's `lag-limit` stays complete-w4-3's 261 unless
   the chain meets a longer run (then the longest, with its reason).

## Files

`engine/crates/sf-conform/examples/whole_game.rs`; `engine/conform/scripts/chain-w3-*.txt`,
`chain-w4-*.txt`, `game-whole.txt`; `engine/conform/cases/game-whole.toml`.

## Interface

None new.

## Tests

- [ ] `whole_game` drives the original from power-on through W4-4's tally into W5-1 with no life
      lost to a broken route.
- [ ] `game-whole` run faithful and alone; its result recorded.

## Done when

- [ ] W3–W4 replay in the chain; the regenerated script and case committed, divergences recorded.
