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

The original's order is docs/08's: per world `world_load_wN` (`world_intro_show`, the `LnLP`
loading picture), per level `level_load` (`progress_map_show`, the progress map) then
`level_init_wN` and `level_start`, and after a world's fourth level `end_world_pic_wN` (`LnET`).
The engine records `unported` for `world_intro_end` (`front/title.rs`) and `progress_map_vbl`
(`front/vbl.rs`) on this path today; both are this task's.

1. START GAME (and the attract demo's fire) starts W1-1 as the original does: world 1's loading
   picture, the progress map, `level_start`.
2. After the tally (and after the slot machine or bonus level when the tally leads there), the next
   level through the progress map; after a world's last level, the world's end picture (`LnET`), the
   next world's loading picture and its first level, the disks asked for and answered from the cache
   as instant loads (DESIGN §2.1 "loads").
3. Pause on P, as the original.
4. Starting from a level code on the options screen enters that level.

Every power-on case is red at frame 0 on `level1_vector` ($64: original `$00fc0c8e`, Kickstart
1.3's level 1 autovector, engine 0; FINDINGS 2026-10-14, dropped T169). The fix is known
(`State::default` sets it beside `EXEC_BASE`; `KICKSTART_LEVEL1` in `plans/rust-engine/T33-wip.patch`,
which T160 also applies). This task lands that line if T160 has not, since `chain-start-w1-1` starts
at power-on.

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
      While `complete-w1-1` is not green alone (T182 measured a life lost at 613), the chain is
      proven from W1-1 reached by START GAME with its exit forced (`level_state` = 1, as the slot
      sync point does), and the full-route variant waits for its owner.
- [ ] `chain-w1-4-to-w2-1`: W1-4's exit through the end picture and W2's loading picture into W2-1,
      alone. `complete-w1-4` reaches only frame 4054 today (T154 open), so the exit is forced as
      above from W1-4 reached by its level code, and T154's route is used once green.
- [ ] `chain-pause`: P in W1-1 pauses and resumes, faithful.
- [ ] `chain-level-code`: a W3-1 code on the options screen enters W3-1, faithful and alone.
- [ ] e2e: the real binary under `--script` from power-on shows W1-1 play by frame 1 500 (captured
      frames non-black, the frog's sprite present).

## Done when

- [ ] All of the above green; no `unported` entry is recorded on these paths.
