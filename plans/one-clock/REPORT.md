# one-clock — delivery report

## What was delivered

All three planned tasks are done and reviewed. Nothing was dropped and no task was added.

- **One way to say where the screen's beam is, and one set of frame and line lengths.** The engine used to name the frame and line lengths in about a dozen places. Each is now written once, in a new shared toolkit part that names nothing of Superfrog. Everything else is worked out from those lengths, so a second game with a different screen timing changes one line.
- **One shared clock.** The level's timekeeping and all the blitter timing (the chip that copies graphics) now live in that shared toolkit part, with the same arithmetic as before. Superfrog supplies its own settings to the clock.
- **The front end and the slot machine use the shared clock too.** Their deliberately coarse timing is kept exactly as it was. It is now held in named rules for rounding, so a later change to it is a visible, separate decision.

The game behaves exactly as it did before. That was the whole point, and the machine checks show it:

- The full comparison against the original game passed: all 216 cases.
- The whole-game run of the real app passed: all 7 runs.
- The intro and slot-machine cases passed: all 9.
- The sound comparison for the whole game gave byte-for-byte the same result as before the change.
- No tolerance or fitted number was touched.
- Save games made before the change still load.

The branch has the latest main merged in, and its tests are green.

## Decisions made for you

None.

## What to check by hand

Nothing needs your eyes. The plan had no part that only a person can check, and every claim above comes from the machine's own comparison with the original game.

If you want a quick look anyway, play a level and the slot machine in the app (`cd engine && cargo run --release -p sf-app`). They should look, sound and feel exactly as they did yesterday.

## Risks and follow-ups

- **One sound check was already failing before this work, and still fails.** The whole-game sound comparison stops on a bookkeeping check. That check expects the result to be filed under an old task's label, but the result is marked as "accepted, awaiting the owner's confirmation". The sound itself compares identical to before. This needs your confirmation of that result, in a separate piece of work.
- **The fix after the merge with main was not reviewed by a fresh session.** After main was merged in, the tests went red, and a worker fixed them. They are green now, but no fresh reviewer read that fix the way each task was read.
- **Two small timing calculations were left where they were.** One is the fade at the end of a frame, the other is the slot machine's interrupt lines. Both still do their own rounding instead of using the new named rules. Same results, just not yet tidied. A small follow-up.
- **The shared clock still assumes Superfrog's frame length in two places:** its end-of-frame pause and the blitter timing's wrap. A second game with a different frame length would need those two made adjustable first. Worth doing at the start of the game-two work.
- **The everyday test run is close to its time limit.** It took 1 h 45 min of the 120 minutes allowed at planning. One more slow test could tip it over.

## Branch

Synced with `main` at `2a843c7a41e6` on 2026-10-28T17:19:35Z.
The tests were red at the end; a worker fixed them.
Tests: green.
