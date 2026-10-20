# T202 — whole-game-script

**Split from:** T21 (its whole-game case: "one scripted game from power-on through every world,
slot machine, bonus level and the final boss to the end sequence (assembled from the per-level
scripts)")

**Phase:** 2 · **Depends on:** T21, T198, T199; blocks T23, T29 · **Weight:** medium

## Goal

Build the whole-game case and run it once, recording where it parts:

1. A tool (`sf-conform/examples/whole_game.rs`) that assembles one power-on script: START GAME,
   then each level's completion script (`complete-w1-1`..`-w1-4`, `w2-1-complete`..,
   `enemies-w3-N-complete`, `complete-w4-N`, `enemies-w5-N-complete`, `enemies-w6-N-complete`,
   `final-win`) spliced at the frame that level's first frame plays, found by running the
   original; nothing held through the screens between (fire held skips them). At least one
   tally takes the slot machine (up in its wait, as `SyncPoint::Slot` does); the bonus level
   played to its end (`chain-bonus-to-w6-1`'s fire held to 8111, with lives enough: check the
   score and lives the chain arrives with).
2. Two drives: the engine plays W6-2..4's first map set (`front::chain::LevelLoad`), which the
   original loads only with a second drive (docs/10). Add a power-on sync point with df1
   connected (`DF1_LEAD`: the boot reaches the title 219 frames sooner).
3. The case `engine/conform/cases/game-whole.toml`, `lag-limit` pinned at the longest run met
   (complete-w4-3's 261) with its reason. Run `sf-conform run` and `run --alone`; record each
   divergence in FINDINGS (frame, level, check, first variable) and its run time.

Known on the path (FINDINGS): `complete-w4-2` red under checks 1–3 (101 mid-pass frames, limit
100), `w2-2-complete` 1229, `w2-3-complete` 2166, `enemies-w6-1-complete` 2136, none owned; game
over by `player_died` a frame early (the route must not lose its last life). Per-level routes
were planned with a fresh level's lives and score; a route may not replay in the chain.

## Files

`sf-conform/examples/whole_game.rs`; `sf-conform/src/sync.rs` (power-on with df1);
`engine/conform/{scripts,cases}/game-whole*`.

## Interface

None new.

## Tests

- [ ] The assembled script parses and drives the original from power-on to `end_seq_load`.

## Done when

- [ ] The case committed with its first divergence (or green) recorded in FINDINGS and PROGRESS.
