# T183 — level-chain

**Phase:** 2 · **Depends on:** T180, T182; blocks T21 · **Weight:** medium

Added by the owner's amendment of 2026-10-15, split ahead of T21 so the engine plays from the menu
through the worlds before the final boss and the whole-game case exist.

## Goal

The engine alone goes from the title to playing, and from one level to the next, as the original
does. Measured on 2026-10-15: from `Start::PowerOn`, fire on the title and on START GAME leaves the
engine on a black screen recording `unported: "a game's start (T21)"` (`front/attract.rs`); after a
level's tally it records `"the next level after the tally (T21)"` (`front/mod.rs`,
`Routine::Outcome`); pause (P) is `debug_assert`ed unported (`level/pass.rs`).

1. START GAME (and the attract demo's fire) starts W1-1 as the original's `level_start` does.
2. After the tally (and after the slot machine or bonus level when the tally leads there), the next
   level; after a world's last level, the world's end picture (`LnET`) and the next world's first
   level, the disks asked for and answered from the cache as instant loads (DESIGN §2.1 "loads").
3. Pause on P, as the original.
4. Starting from a level code on the options screen enters that level.

## Design sections this implements

DESIGN §2.1, §2.11 (any path the original would hang on).

## Files

- `engine/crates/sf-core/src/front/{mod.rs,attract.rs,menu.rs}`, `level/{exit.rs,pass.rs}`; a new
  `front/chain.rs` if the routines want one.
- `engine/conform/{scripts,cases}/chain-*`; `engine/crates/sf-conform/tests/chain.rs`.
- `engine/crates/sf-e2e/tests/smoke.rs`: the app from power-on reaches W1-1 play.

## Tests

- [ ] `chain-start-w1-1`: power-on → title → START GAME → W1-1 play, faithful and alone.
- [ ] `chain-w1-1-to-w1-2`: `complete-w1-1`'s route continued into W1-2's first 500 frames, alone.
- [ ] `chain-w1-4-to-w2-1`: W1-4's exit (from `complete-w1-4`'s route, T154's when green, else a
      transplant at its exit) through the end picture into W2-1, alone.
- [ ] `chain-pause`: P in W1-1 pauses and resumes, faithful.
- [ ] `chain-level-code`: a W3-1 code on the options screen enters W3-1, faithful and alone.
- [ ] e2e: the real binary under `--script` from power-on shows W1-1 play by frame 1 500 (captured
      frames non-black, the frog's sprite present).

## Done when

- [ ] All of the above green; no `unported` entry is recorded on these paths.
