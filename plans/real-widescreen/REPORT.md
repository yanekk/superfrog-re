# real-widescreen — delivery report

## What was delivered

**Widescreen is now real game, not just a wider picture.** With View ▸ Widescreen on, the two side strips show everything that is alive there: enemies, coins, moving platforms, bonus pieces, and the frog when it walks off-centre near a level's end. Things wake up as they reach the edge of the wide view and are cleared after they leave it, by the original game's own distances measured from the wider edge.

- **No black past the map.** Near a level's left or right end the camera now stops 65 pixels earlier, and the frog walks on into the strip.
- **Room for more.** Widescreen keeps up to roughly four times as many things alive at once (the busiest level peaked at 92 of 100 places). The extra work never slows the game down.
- **Switching mid-level works.** Turning widescreen on wakes whatever is in the strips at once; turning it off lets the extras fade out as the camera moves on.
- **Bug recordings carry the mode.** A recording notes when widescreen was on and each switch; a replay plays and draws it that way, and the Widescreen menu item is greyed out during a replay. A recording with widescreen in it cannot be compared with the original game, and the bug tools say so.
- **Every level finishes in widescreen.** All 24 normal levels, the bonus level and the final level each have a widescreen route kept as a test, and all 26 replay to the end. 25 were found by the route planner; W6-4's is the run you played yourself, labelled as yours.
- **With widescreen off nothing changed:** the game is still the original, judged against it as before.

Not delivered: a planner-found widescreen route for W6-4 (the planner stops 48 squares short there even with your run as its guide).

## Decisions made for you

- **T08: A wandering enemy entering from the right can, for one frame, show a 2-pixel-wide gap at the join between the middle picture and the right strip (the original game hasn't drawn it yet; the strip has). Which wins?** (night)
  Answer: A wandering enemy entering from the right can, for one frame, show a 2-pixel-wide gap at the join between the middle picture and the right strip (the original game hasn't drawn it yet; the strip has). Which wins? → Keep middle exact (Recommended)
  Why: DESIGN §2.9 keeps the centre exactly what the display buffers hold; changing it is a rule exception, and late pop-in is worse than a rare 1-frame 2 px gap. Log it in FINDINGS so T11's drill judges it against the 'no seam' success criterion. Easiest to undo.
- **T10: While the app replays a bug recording (--replay), what should View ▸ Widescreen do? Today it is allowed during a replay because it only changed the picture. Now widescreen changes the game itself, so switching it mid-replay would make the replay play a different game from the one recorded. The design says the replay takes the mode and the window shape from the recording, at its start and at each switch, and leaves your saved setting alone.** (night)
  Answer: While the app replays a bug recording (--replay), what should View ▸ Widescreen do? Today it is allowed during a replay because it only changed the picture. Now widescreen changes the game itself, so switching it mid-replay would make the replay play a different game from the one recorded. The design says the replay takes the mode and the window shape from the recording, at its start and at each switch, and leaves your saved setting alone. → Disabled in a replay (Recommended)
  Why: DESIGN §2.8: the replay plays the mode as recorded and leaves the saved setting alone; disabling the item with its tick showing the replay's mode is the only option that keeps the replay faithful, and the simplest to undo.
- **T11: When widescreen is switched on or off while the game is paused, what should the paused screen show? / In the final level, what should the strips beside the castle show?** (night)
  Answer: When widescreen is switched on or off while the game is paused, what should the paused screen show? → Keep as is (Recommended); In the final level, what should the strips beside the castle show? → Keep the map (Recommended)
  Why: Both: no change, easiest to undo. Q1: §2.7 takes effect at the next step, and a paused game takes none; the bars clear on unpause. Q2: §2.9 draws the strips from the map and §8 rules out special-casing; record the final level's look in FINDINGS for the owner.
- **T12: How should W6-4's widescreen route be finished? (If you don't answer, I'll do option 1 and stop just before handing you the command to play.)** (night)
  Answer: How should W6-4's widescreen route be finished? (If you don't answer, I'll do option 1 and stop just before handing you the command to play.) → You play it, now (Recommended). Make the hand-over tools widescreen-aware behind an opt-in flag, so their default 4:3 behaviour is unchanged. First show the stall in sf-routeview as CLAUDE.md asks. Then hand the owner the one command and wait for them: they are away until morning, and waiting is fine. Finish the other 25 meanwhile.
  Why: CLAUDE.md's owner rule: a planner stall is played by the owner, not accepted or hinted. T12's Done-when needs all 26. The tools change is opt-in and reversible, so it is allowed beyond T12's file list. Waiting for the owner until morning costs nothing.
- **T12: The planner still can't finish W6-4 in widescreen, even with your full run to the exit as its guide (stuck 48 squares short on both seeds). Your recording itself replays to the exit from the normal widescreen start. What should W6-4's widescreen route be? (If you don't answer, I'll wait.)**
  Answer: The planner still can't finish W6-4 in widescreen, even with your full run to the exit as its guide (stuck 48 squares short on both seeds). Your recording itself replays to the exit from the normal widescreen start. What should W6-4's widescreen route be? (If you don't answer, I'll wait.) → Keep your recording (Recommended). Label the route file and its test as owner-played, not the planner's, and add a FINDINGS row: the planner fails W6-4 wide 48 squares short even with the owner's guide, on both seeds.
  Why: T12's purpose is proving every level can be finished in widescreen. The owner's recording replaying to the exit proves W6-4 can; it departs from DESIGN §1's 'by the smart planner' for one level, labelled so it is not mistaken for the planner's. Reversible: swap in a planner route later.

## What to check by hand

Nothing is required: every hands-on check in the plan was done by a worker driving the real app (the drill played eight levels, the bonus level and the final level in widescreen, with switches, recording and replay, fullscreen included, and saw no black, pop-in, tears or crashes).

If you want to judge how it feels, which no test can:

```
cd engine && cargo run --release -p sf-app
```

Turn on View ▸ Widescreen and play a level. Things to look at:

- enemies and coins appearing at the far edges of the strips, not suddenly inside them;
- a level's start or end, where the frog walks into the strip instead of black appearing;
- the final level, where the strips beside the castle show bits of the level map (plain purple on the left, a thin band of leftover scenery on the right). Say if you would rather have them flat purple;
- very rarely, an enemy walking in from the right can leave a sliver 2 pixels wide, for one frame, where the strip meets the middle. The drill did not notice it; say if you do.

## Risks and follow-ups

- **The full set of original-game comparisons was not run by the workers.** Several tasks were told to run the whole test suite once, which pins the 4:3 game against the original, but the parallel-build rules stopped them, so only the regular test set ran. It is green, but the slowest original-game checks (about 50 minutes) have not been run on this branch. Worth running once before or soon after merging.
- **One recorder test crashed while talking to the original game** during the build: it looked like the new extra-object room was being written into the original's memory, where it has nowhere to go. The tests are now green after a worker's fix, but that this exact cause was fixed is not stated anywhere.
- **The bonus level is shorter in widescreen**, about 5.6 seconds sooner to the end, because it starts further on and its end marker is reached sooner. You accepted the 3-second part at plan review; the extra start offset was found during the build.
- **Widescreen is a different game:** more enemies are awake earlier, so old routes mostly fail in it (only 2 of 24 old routes still finish in widescreen). Expected, but anything that replays old routes must play them in the mode they were made in.
- **Save states from before this change no longer load**, because the game's saved state grew.
- **The planner is weak on W6-4** in widescreen as it was in 4:3; W6-1 also needed a second try. A follow-up could improve the planner and replace your W6-4 run with one of its own.
- **One test is weaker than its name**: the check that a restart in widescreen wakes the strips only looks at the left strip, and could pass even if the restart's own wake-up were missing. Not fixed.

## Branch

Synced with `main` at `4bef5979321c` on 2026-10-27T09:07:04Z.
The tests were red at the end; a worker fixed them.
Tests: green.
