# forest-dumb-map-page — delivery report

## What was delivered

**The final test run is red.** The quick test suite did not pass at the end of the build, and one attempt to fix it did not make it pass. All 13 tasks were built and reviewed, and your main branch merged into this work without conflicts. See *Risks* for what is known about the failure.

What the map page does now:

- **One page, all six levels.** A single command writes one map page with every level, W1-1 to W6-1. You open it straight from disk in Chrome. Each level shows its tiles, every object in its place, the exit, and how many items the exit needs. That number is worked out from the map the same way the original game does it, and it matched the original in all six worlds.
- **Live while you play.** With the app running, the page shows a dot on your character about four times a second. It also shows the creatures near you where they really are, removes items as you take them, and keeps a live count of items left. It switches to the level you are playing by itself. If you pick a level by hand, it stays there until the game moves to a new world.
- **Follow character** keeps the dot in view. It is on when the page opens, scrolling by hand turns it off, and one click turns it back on.
- **No stale position.** In menus, between levels or at game over the page says "Not playing". If the app is closed it says "App not running". It never shows an old position as if it were current.
- **Game ▸ Show Map** in the app writes the page fresh and opens it in Chrome, without pausing the game.
- **Secrets layer**, off when the page opens. It marks hidden passages: walls the character can walk through, 133 spots in W1-1, 292 in W3-1 and 63 in W4-1. They were confirmed on the original game. No other kind of secret was confirmed, so no other kind is shown.
- **Your two answers to T12 are in.** A page with no routes keeps the route viewer's wording. While Secrets is off, the secret counts stay out of the object count and the level summary too.

Not delivered, as planned: routes for worlds 2 to 6, and browsers other than Chrome.

## Decisions made for you

None.

## What to check by hand

- You already checked the one hand step: Show Map opened Chrome on the level 1 map, with the dot on your character and moving as you played (noted on 2026-11-20). Nothing else is left for you to check by hand.
- If you'd like a second look once it's merged, play into world 2 with the map open. The page should jump to W2-1 by itself, with the dot on you and the items-left count going down as you collect.

## Risks and follow-ups

- **Red tests.** The final quick test suite did not pass, and a fix attempt did not change that. The build left no note saying why. Earlier in this run the same suite went over its 20-minute limit when it had to build the Superfrog game from scratch alongside another build. Then no test actually failed, and this plan's own new quick tests take about 22 seconds. That may be the cause again, but nobody has confirmed it. Re-run the suite on its own before merging, and don't merge if a real test fails.
- **Level 1 creature mismatch.** After a long scripted run of level 1, the translated game and the hand-built level 1 engine disagree slightly: one extra creature near the player, and another one 8 pixels apart. Items and taken objects agree. Nobody has looked into this yet.
- **Rare stale map update.** About once in 950 updates, Chrome reads the live file while it is being replaced. The page then keeps the previous update, so the dot is at most a quarter-second late. It never shows wrong data.
- **Small layout quirks**, left as they are:
  - On a small 800×600 window, less than half the window is map; the controls take the top.
  - Near a level's left edge, Follow centres the dot, which leaves an empty black strip beside the level.
- **Two secret areas weren't tested on the original:** a tower in W1-1 and one group in W3-1. They are marked on the map from the same rule as the rest, but nobody walked through them on the original.
- **An unrelated old code warning** makes a code-checking tool fail on parts of the project this plan did not touch. Worth a separate small fix.
- Possible follow-up: two smaller plans (level 2 and the title screen) share pieces with this one. Whichever is built next should take over this plan's map setup instead of starting its own.

## Branch

Synced with `main` at `319c6b3164f3` on 2026-11-20T20:19:37Z.
The tests were red at the end; a worker tried to fix them and they stayed red.
Tests: red. The branch is not ready to merge.
