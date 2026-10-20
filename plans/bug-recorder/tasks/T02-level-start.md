# T02 — level-start

**Phase:** 1 · **Depends on:** T01 · **Weight:** medium

## Goal

Let the engine start a level with a `LevelSetup` and a seed, by the way T00 chose, so the app and
the replay tool start a recording identically; and let it say where the frog is, as the owner
addresses tiles, or which screen is showing.

## Design sections this implements

DESIGN §2.1, §2.6 (`seed`, `stopped-at`), §7 D2 (as T00 settled it).

## Files

- `engine/crates/sf-core/src/game.rs`: `start_level`, `location`
- `engine/crates/sf-core/tests/start_level.rs` (new)
- `engine/crates/sf-core/src/` (a module chosen in the task): `SceneKind` and its classifier, moved from `sf-conform/src/milestones.rs`
- `engine/crates/sf-conform/src/milestones.rs`: uses the moved classifier

## Interface

```rust
impl Game {
    /// The level `setup.level` at its first frame with the setup's values, started the way T00 chose.
    pub fn start_level(display: Arc<DisplayData>, setup: &LevelSetup, seed: Rng) -> Game;
    /// Where the frog is in a level (DESIGN §2.6), else the screen's name.
    pub fn location(&self) -> Location;
}
```

Screen names: `title`, `intro` (the world's loading picture and text), `world map`, `tally`,
`slot machine`, `game over`, `story`, `end sequence`. The screen is told apart by the classifier
that exists in `sf-conform/src/milestones.rs` (`SceneKind`, `FrameState::scene`, by COP1LC), moved
into sf-core so the app and sf-conform share one copy; milestones.rs then calls it with no
behaviour change (owner, plan review 2026-10-20). Names it lacks (`intro`, `world map`) are added
there, not in a second classifier. The cell is `((frog_x + 0x17)/16, (frog_y - 0x10)/16)`, as T00 checked.

## Tests

- [ ] for each of the 29 levels with defaults: `start_level` leaves the scene a level, the level id right, the frame the level's first
- [ ] each field lands: lives 3/5/7 → `frog_lives` 2/4/6; points; `frog_next_life`; energy 1–4; spud; wings; difficulty in `opt_game_mode` and in the tuning `frog_new_game` derives from it
- [ ] the same setup and seed twice give byte-equal states after 1 000 steps of the same inputs; a different seed differs in `rng_state`
- [ ] `location` at W1-1's start is the cell T00 checked; after a scripted level end it is `tally`, then `slot machine` when the slot is entered, `world map`/`intro` as the chain goes
- [ ] an invalid setup panics in debug with the reason (callers validate first)
- [ ] sf-conform's milestone tests pass unchanged after the classifier move

## Done when

- [ ] `start_level` and `location` exist with the tests above
- [ ] the purity test is green (no clock, no fs)
- [ ] the test command is green
