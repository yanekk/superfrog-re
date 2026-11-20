# T02 — level-start

> **Amendment (T00, 2026-10-20): the start is (b), measured on the oracle.** `start_level` boots
> `Start::PowerOn { seed }`, waits for the title (`title_waits`), fire, the menu, down + fire for
> OPTIONS, then on the options screen (cursor on LIVES, presses from 100 frames after OPTIONS is
> taken, each held 10 frames, 40 apart): right/left from `opt_lives` 0 to the setup's lives, down,
> left from `opt_game_mode` 1 for easy, down twice to LEVEL CODE, the code's digits 60 frames on as
> `type_code` types them, fire to leave, then the menu's START GAME; it steps unseen until
> `scene()` is `Level` or `Bonus`, which is frame 0. Then it writes `score`, `frog_next_life`,
> `frog_hearts`, `frog_spud_on` and `frog_flight` (1 measured for wings). The spike drove this from
> outside with `Game::step`; generalising `type_code` to take the two options is fine. Measured:
> equal to the original in every gameplay variable at frame 0 on W1-1, W1-2, W4-3, WB-1 in easy and
> normal with lives 3/5/7, and faithful for 2 000 frames, through W1-1's end and W1-2's load too.
> No hard (owner). The cell formula held (T00 Q3; FINDINGS).

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
