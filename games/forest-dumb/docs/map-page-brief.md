# Brief for `pir plan`: a map page of every Forest Dumb level

Written 2026-11-20 for the planner. Slug: `forest-dumb-map-page`. Read `games/forest-dumb/CLAUDE.md`
("Seeing a run on the map"), `fd-routeview`'s and `toolkit/crates/amiga-routeview`'s READMEs, Superfrog's
`sf-routeview` (`games/superfrog/engine/crates/sf-routeview/`), [LEVEL-INDEX.md](LEVEL-INDEX.md) and
[10-object-system.md](10-object-system.md) first.

## The goal

The owner keeps a map open on one screen and plays the app on the other, as with Superfrog's route page.
One command writes a page that opens from disk and shows any of the six levels: the whole tile map, every
collectible, creature, spring, water, vanishing block and bonus in its place, the exit marked, and how many
items the level needs before the exit opens. A switcher moves between W1-1 to W6-1. Routes, where there
are any, draw over it as today.

## Where it starts

- The page writer is shared already: `amiga-routeview` (toolkit) draws `fd-routeview`'s and
  `sf-routeview`'s pages. Nothing is to be copied from Superfrog; what is missing is game-side.
- `fd-routeview` draws W1-1 only, and only with an input script, replayed on `fd-core`'s hand engine
  (`Start::Level1`). The page shows the objects of the object map, those in play, the path and the events.
- Object names exist for W1-1 alone (`re/objects-W1-1.toml`, groups creature, spring, item, water,
  vanishing-block, bonus, exit). W2-1 to W6-1's object maps (`_MapaBobow2` … `6`) are read as bytes but
  their values are not named.
- Levels render already (`tools/render_levels.py`, `build/renders/W*-1.png`, [LEVEL-INDEX.md](LEVEL-INDEX.md)).

## What it delivers

| Part | Delivers | Proven by |
|---|---|---|
| A level with no route | `fd-routeview --level W2-1` (and `--all`) writes the map and its objects with no script and no engine run: tiles and objects read from the level's files | A test writes each level's page and checks its object counts against the files |
| Every world | All six levels in one page with the switcher; W6-1's 320 columns included | The page lists six levels |
| The objects named | Each world's object values sorted into the page's layers (items, creatures, hazards, exit), from the code where W1-1's table does not cover them; an unknown value is drawn as `other` with its number, never guessed | Each world's exit cell and item count match the original at the level's start, read on the oracle (`items_left` after `level_init`) |
| The owner's view | Layers on and off, the exit easy to find, the item count shown, the page legible beside the app on a second screen | A test checks the page's contents; the owner looks once (a person check, recorded in FINDINGS with the date) |

## Settled

- The page is a viewing tool, not the engine: it never changes what the app plays, and the faithful bar
  binds what it shows (positions and counts as the original's), not how it draws.
- It holds disk graphics: written under `games/forest-dumb/build/`, never committed.
- Reuse the toolkit's page writer unchanged where it serves; a change it needs is made there, game-free,
  with Superfrog's page staying green (root CLAUDE.md, "Before building something").

## Open, for the planner to settle with the owner

Proposals only.

1. **A live "you are here".** Neither game's page follows the app today. One option: the app writes the
   character's cell and the items left to a small file each second, and the open page shows a marker that
   moves as you play. Proposed: not in this plan; the static map first, the live marker a follow-up once
   the owner has used it. It is the largest open choice, so ask first.
2. **Opening it from the app.** Proposed: a menu item (View ▸ Show Map or Game ▸ Show Map) that writes the
   page for the current level and opens it in the browser, beside the View menu brief already drafted.
   Otherwise a command only, as Superfrog's.
3. **Routes on the translated program.** `fd-routeview` replays scripts on the hand engine, which covers
   W1-1 only. Proposed: leave route replay as it is here; replaying a route on the translated program for
   any world is the level plans' need, added when one needs it.
4. **Secrets.** If a world has hidden passages or bonus rooms, show them? Proposed: only what the object
   map holds; hidden passages are not read for this game yet.

## Out of scope

Editing a map, a route planner for worlds 2 to 6, the live marker (unless the owner chooses it, Open 1),
widescreen.
