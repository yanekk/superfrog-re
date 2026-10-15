# T21 — game-flow

**Phase:** 2 · **Depends on:** T18, T19, T20, T181, T183, and every open completion task (PLAN.md) · **Weight:** medium

> **Amended** by the owner, 2026-10-15: judged by DESIGN §2.1's faithful checks; T183 has taken
> the game's start, level-to-level and world-to-world chaining and pause, so this task builds on it.

## Goal

Chain every scene the way the original does: power-on, intro, title, game start or password, world
and level order, slot machine and bonus levels between levels, the final level, game over and continue,
disk-swap prompts, and the end sequence. Exposes progress (furthest world) for the app to remember,
and is proven by scripted whole games from power-on that the engine plays alone with the original's
milestones (DESIGN §2.1 check 4).

## Design sections this implements

DESIGN §2.1, §2.9, §3.3.

## Files

- `engine/crates/sf-core/src/flow.rs`
- `engine/crates/sf-core/src/game.rs` — `Start::PowerOn` runs the flow.
- `engine/conform/{scripts,cases}/game-*`
- `engine/crates/sf-core/tests/flow.rs`

## Interface

```rust
impl Game { pub fn progress(&self) -> Progress; pub fn start_at(data: &GameData, progress: &Progress) -> Game /* as the original's password would */ }
pub enum DiskPrompt { Insert(DiskSlot) }   // surfaced in FrameOut so the app can answer it automatically from the cache
```

## Tests

- [ ] the whole-game case: one scripted game from power-on through every world, slot machine, bonus level and the final boss to the end sequence (assembled from the per-level scripts), green faithful and alone.
- [ ] game over and continue; starting from each password.
- [ ] disk prompts: the engine raises the same prompt on the same frame as the original, and the app-side answer is a no-op wait of the same length.
- [ ] `start_at(progress)` equals entering that password on the title.

## Done when

- [ ] The whole-game case is green in `sf-conform -- all`, and its run time is in PROGRESS notes.
- [ ] No case file anywhere excludes a variable or object.
- [ ] `progress()` and `start_at()` exist for T23.
