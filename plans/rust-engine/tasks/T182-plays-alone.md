# T182 — plays-alone

**Phase:** 2 · **Depends on:** T180; blocks T181, T183, T21 and every completion task · **Weight:** medium

Added by the owner's amendment of 2026-10-15 (DESIGN §2.1 check 4).

## Goal

The check that proves the engine runs on its own. From a case's sync point the engine is started
once and then runs with nothing from the original (no re-sync, no beam anchor, no transplant): only
the inputs and the original's pacing. Both sides' milestones are extracted and compared, in order,
each within ±10 frames of play (the original's load frames not counted).

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

`SceneKind` names title, menu, level start, tally, slot machine, bonus level, game over and end
sequence; the worker extends the enum where the original has a scene this misses.

## Tests

- [ ] Unit: milestone lists equal; one shifted 10 frames passes, 11 fails; a missing or extra
      milestone fails with its frame and kind printed.
- [ ] `complete-w1-2` (green today with re-syncs) run alone: green, or its first differing milestone
      is a FINDINGS row with the cause and the task that owns it.
- [ ] The W1-1 replay above: its cause found and fixed, or owned by a named task.

## Done when

- [ ] Every `complete-*` / `*-complete` case on the branch has `alone = true`; each is green alone or
      its first divergence is in FINDINGS with an owning task.
- [ ] README updated.
