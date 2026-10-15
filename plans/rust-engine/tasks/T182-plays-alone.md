# T182 — plays-alone

**Phase:** 2 · **Depends on:** T180; blocks T181, T183 and every completion task · **Weight:** medium

Added by the owner's amendment of 2026-10-15 (DESIGN §2.1 check 4).

## Goal

The check that proves the engine runs on its own. From a case's sync point the engine is started
once and then runs with nothing from the original (no re-sync, no beam anchor, no transplant): only
the inputs and the original's pacing. Both sides' milestones are extracted and compared, in order,
each within ±10 frames of play (the original's load frames not counted).

What "nothing from the original" means against today's runner (`sf-conform/src/lib.rs` `run_inner`):
the engine still gets each frame's input as the original consumed it (the script indexed by the
original's frame, its load and disk-wait frames skipped), the original's `Pacing`, and the RNG seeds
the original read from its clock (DESIGN §2.1 check 4). It gets none of the rest the runner feeds
today: `anchor_writes`/`anchor_blits`, `set_handler_entry`/`set_handler_end`/`set_blank_merged`,
`set_load_line` (found by matching the original), `set_load_state` and the pokes of
`runs_during_load`, `load_leaves` and `FrameOut::resync` after a load. Where the engine needs one of
those to go on, it supplies it itself as normal play does (a load leaves what the engine's own
import gives). A `Pacing::Late` whose cut the engine's pass cannot reach once the two have drifted
(fewer Paula writes, another phase or step) ends the engine's frame on its own clock; it never
panics.

Measured on 2026-10-15 (amendment session): `Game::new(Start::Level(W1-1))` replaying
`complete-w1-1.txt` alone loses a life at frame 613, where the original with the same inputs does
not. Whether that is the start (`Start::Level` versus the case's sync), the pacing, or a mechanic is
for this task to find; it is the first thing this check is pointed at.

## Design sections this implements

DESIGN §2.1 check 4, §4.

## Files

- `engine/crates/sf-conform/src/milestones.rs`: milestone extraction from the original's memory
  and from the engine's state, by the same `LAYOUT` variables.
- `engine/crates/sf-conform/src/{case.rs,main.rs}`: `alone = true` in a case (default for every
  `complete-*` and `*-complete` case and the whole-game case); `run --alone` for any case.
- `engine/crates/sf-conform/tests/alone.rs`; `engine/conform/README.md`.

## Interface

```rust
pub enum Milestone { Scene(SceneKind), CoinTaken { left: u16 }, ItemTaken { kind: u16, cell: (u16, u16) },
                     EnemyKilled { kind: u16 }, LifeLost { lives: u16 }, LevelExit(LevelId),
                     Tally { score: u32 }, End { score: u32, lives: u16, coins: u16 } }
pub fn milestones(frames: &[FrameState]) -> Vec<(u64 /* frame of play */, Milestone)>
```

`SceneKind` names title, menu, world loading picture (`LnLP`), progress map, level start, tally,
slot machine, bonus level, end-of-world picture (`LnET`), game over and end sequence; the worker
extends the enum where the original has a scene this misses. The original keeps no count of kills or
items taken: those milestones are derived from changes in `obj_slots`, `level_map`, `score` and the
coin counters, by one function applied to both sides' `LAYOUT` values, so a derivation fault cannot
make the two sides differ.

## Tests

- [ ] Unit: milestone lists equal; one shifted 10 frames passes, 11 fails; a missing or extra
      milestone fails with its frame and kind printed.
- [ ] `complete-w1-2` (green today with re-syncs) run alone: green, or its first differing milestone
      is a FINDINGS row with the cause and the task that owns it.
- [ ] The W1-1 replay above: its cause found and fixed, or owned by a named task.
- [ ] Unit: a `Pacing::Late` naming a cut the engine's pass does not reach ends the frame on the
      engine's clock without a panic.

## Done when

- [ ] Check 4 runs on every case that steps the engine (owner, 2026-10-15), not only completion
      cases: `alone` is on by default and `render`-only cases are exempt. The completion cases are green
      alone or their first divergence is in FINDINGS with an owning task; the rest are T181's to re-judge.
- [ ] README updated.
