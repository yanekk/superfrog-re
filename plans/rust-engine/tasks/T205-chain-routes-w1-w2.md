# T205 — chain-routes-w1-w2

**Phase:** 2 · **Depends on:** T202; blocks T203 · **Weight:** medium

Added by T202 with the owner's yes (2026-10-16): the per-level routes do not replay in the
whole-game chain, so `game-whole` cannot reach `end_seq_load` (T202's test) until they are
re-planned leg by leg. T205 takes W1–W2, T206 W3–W4, T207 the rest.

## Goal

1. **First, record why a standalone route breaks in the chain**, so T206 and T207 plan with it
   known: which of what the chain carries over (the frog's power-ups — T202 saw `frog_spud_on` and
   `frog_flight` 1 entering W1-2 — `rng_state`, the enemies' counters such as `w1_hover_phase`,
   lives, score, other object state) actually changes the route's outcome, measured on the
   original (`examples/whole_game --diff <leg>` prints what differs at the leg's start and the
   first frame the frog's path parts). One FINDINGS row naming the cause(s), and whether a route
   planned in the chain stays valid whatever the earlier legs do (it does not if it depends on
   state an earlier re-plan changes: say which).
2. Re-plan W1-2, W1-3, W1-4, W2-1, W2-2, W2-3 and W2-4 for the chain: each from the original's
   state at that leg's start in the chain (`whole_game --from <leg>` restores the snapshot it saved
   there), with the planning tools (`examples/plan_exit`, `examples/explore`, extended to start
   from a saved chain snapshot instead of an `AtLevel` sync point) and checked on the original
   chunk by chunk. A leg whose standalone route still replays keeps it. New routes are new
   scripts (`conform/scripts/chain-<leg>.txt`); the standalone `complete-*`/`w2-*-complete`
   scripts and their cases stay as they are.
3. Point `whole_game`'s `LEGS` at the new scripts and regenerate `game-whole.txt` up to W3-1's
   first frame (or further, if W3's standalone routes replay). Run `game-whole` (`run` and
   `run --alone`); record each divergence in FINDINGS (frame, level, check, first variable) and
   the run times, and set the case's `frames` to cover what the script now reaches.

## Files

`engine/crates/sf-conform/examples/{whole_game,plan_exit,explore}.rs` (start from a chain
snapshot); `engine/conform/scripts/chain-w1-*.txt`, `chain-w2-*.txt`, `game-whole.txt`;
`engine/conform/cases/game-whole.toml`.

## Interface

None new.

## Tests

- [ ] `whole_game` drives the original from power-on through W2-4's tally into W3-1 with no life
      lost to a broken route.
- [ ] `game-whole` run faithful and alone; its result recorded.

## Done when

- [ ] The cause row is in FINDINGS before the first re-plan is committed.
- [ ] W1–W2 replay in the chain; the regenerated script and case committed, divergences recorded.
