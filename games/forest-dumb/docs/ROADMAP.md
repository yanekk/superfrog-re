# Backlog: the plans after forest-dumb-start

Owner's sequence, 2026-10-31. None of these is planned or built. Each is a `pir plan` run with a slug
starting `forest-dumb-`. Start plan 1.1 once `forest-dumb-start` is merged: a plan branches from
`main`, which until then lacks that plan's disk reader, formats and program map. Size each plan again
from T06's map; 1.1 may need a further split.

## Level 1

Plan 1 was split into 1.1 to 1.5 so each ends with something that can be run or seen.

| # | Plan | Delivers | Proven by | Needs |
|---|---|---|---|---|
| 1.1 | Character in the level | Level 1 headless: running, jumping, collision, scrolling, drawing | Same inputs on the original and the engine; movement and picture match on stretches with no enemies or collectibles | `forest-dumb-start` |
| 1.2 | Playable app | Window and keyboard; walk around level 1 | A test drives the real app through a scripted run | 1.1 |
| 1.3 | Recorder | Save a recording of play or a bug, replay it | A saved recording replays to the same result | 1.2 |
| 1.4 | Map page | HTML page: level 1's map with the character's path | A test generates the page from a scripted run and checks its contents | 1.1 |
| 1.5 | Route planner | Finds a route through level 1 as far as it can get, shown on the map page; based on Superfrog's | The route replays on the engine and reaches the expected point | 1.4 |
| 2 | Collectibles | Level 1 with collectibles, playable end to end | A test | 1.1 to 1.5 |
| 3 | Enemies | Level 1 with enemies, playable end to end | A test | 2 |
| 4 | The rest of the level | Any special mechanics, such as doors or secret passages | A test plays the level from start to end | 3 |

1.2 and 1.4 can run side by side. After plan 4, continue with the next levels.

## Decided by the owner, 2026-10-31

- Small plans first, in the order above, instead of one port plan.
- The app starts straight in level 1. The title screen and menus are a later plan.
- Silent for now. Sound is a later plan.

The standing rule on disk-derived files applies to the map page: it holds disk graphics, so it stays in
`build/` and is never committed.

## Open, for the plan that meets it

- 1.1: what "correct" means while the engine has no enemies or collectibles and the original does.
  Proposed: compare movement, collision and scrolling only, on stretches where nothing else interferes.
- 1.4 and 1.5: copy Superfrog's route planner and map page, or move their shared parts into the toolkit.
  Both are outside the toolkit today. Proposed: move the shared parts, since every later level uses them.
- 1.5: if the exit needs a mechanic from plan 4, the route stops short of it. Proposed test: reaches the
  furthest point possible, with the full run in plan 4.

## Level count

Six levels, one a world, W1-1 to W6-1, per the unreviewed T05 of `forest-dumb-start` (2026-10-31). It read
the files only. Whether the game replays a level or has bonus stages is unconfirmed; T06 or plan 1.1
confirms it.
