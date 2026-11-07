# Backlog: the plans after forest-dumb-start

Owner's sequence, 2026-10-31. Nothing here is planned or built yet. Each plan is a `pir plan` run with a slug
starting `forest-dumb-`. `forest-dumb-start` is merged; the level 1 plan's brief is [level-1-brief.md](level-1-brief.md).

## Level 1

One plan, one `pir plan` run (owner, 2026-10-31; a split into several plans was considered and dropped).
The stages below are its order of work; the planner turns them into tasks and may run them side by side.

| Stage | Delivers | Proven by |
|---|---|---|
| Character in the level | Level 1 headless: running, jumping, collision, scrolling, drawing | Same inputs on the original and the engine; movement and picture match on stretches with no enemies or collectibles |
| Playable app | Window and keyboard; walk around level 1 | A test drives the real app through a scripted run |
| Recorder | Save a recording of play or a bug, replay it | A saved recording replays to the same result |
| Map page | HTML page: level 1's map with the character's path | A test generates the page from a scripted run and checks its contents |
| Route planner | Finds a route through level 1, shown on the map page; based on Superfrog's | The route replays on the engine and reaches the expected point |
| Collectibles | Level 1 with collectibles, playable end to end | A test |
| Enemies | Level 1 with enemies, playable end to end | A test |
| The rest of the level | Any special mechanics, such as doors or secret passages | A test plays the level from start to end |

## The translator

Owner, 2026-11-06: a static translator of the main program becomes the starting engine for everything
after level 1, with hand-written Rust where the owner chooses (option B). Plan `forest-dumb-translator`,
brief [translator-brief.md](translator-brief.md). Proposed order, settled at planning: after level 1
merges, before level 2.

The trial (stage 0, plan `forest-dumb-translator-trial`, [13-translator-trial.md](13-translator-trial.md)):
84 of 85 judgeable routines pass with no hand edit (98.8% against 90%) and level 1's path writes into
no executed code. Owner's go, 2026-11-07: the second plan, `forest-dumb-translator` (stages 1 to 6), is
next once level 1 merges.

## After level 1

One plan a level, in order: W2-1 to W6-1 if the count below holds. Then a plan for sound. All owner,
2026-10-31.

A plan for the title screen and menus, which can run at any point after level 1, independent of the
level plans.

The game has an ending: `end_screen`, after world 6, using `_END` (`docs/07-executable-map.md`, read from
the code and not run). Its work goes in the last plan below unless it proves large enough for its own.

Last, once everything above is done: a plan for the whole-game end-to-end test, as Superfrog has
(`games/superfrog/engine/crates/sf-e2e/`, `whole_game.rs`), played from power-on through the ending.

## Decided by the owner, 2026-10-31

- Level 1 first, as one plan with the stages above; then one plan a level; then sound; the whole-game
  end-to-end test last.
- The title screen and menus are their own plan, in any order relative to the levels.
- The app starts straight in level 1 until the title screen and menus plan lands.
- Silent until the sound plan, which follows the level plans.

The standing rule on disk-derived files applies to the map page: it holds disk graphics, so it stays in
`build/` and is never committed.

## Level count

Six levels, one a world, W1-1 to W6-1. `next_world` advances `world_index` and `end_screen` follows world 6
(`docs/07-executable-map.md`), read from the code and not run. Bonus stages or replays are not ruled out.
