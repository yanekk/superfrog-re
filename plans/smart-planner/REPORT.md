# smart-planner — delivery report

## What was delivered

**One route planner instead of four.** An agent now asks a single command for a route: it names the level, and the planner plays the level to its exit and hands back an input recording that drops straight into the project's test cases. The three old level-routing tools, and the fourth one the plan also retired, have been deleted. The bonus-level tool stays, as planned. This target (one tool to choose between) is met.

**How it works now.** The planner explores the whole level on our engine, keeping the best way it has found to every spot. In original mode it checks its route against the real 1993 game as it goes, and repairs the route from wherever the two part. It can be given a guide, either an existing route or a short list of spots to pass, and the counted runs were all guided by each level's existing route, as you decided. It also plans routes for the app's own timing ("app mode"), and those routes replay in the real app.

**The counted results (guided):**
- Single levels on the original (set A): **21 of 24** finish, against a target of 24.
- Levels inside a whole-game run (set B): **18 of 24**, against 24.
- App-timing levels (set C): **22 of 24**, against 24. All 22 also replay to the exit in the real app.
- Routes that lose no heart: **10 of 24**, against a target of 20.
- Route length: four routes run more than 25% longer than the existing ones. App mode also misses its speed target: a median of 7.3 minutes per level against 2.
- One failed level overran its 30 minutes by 18 seconds.
- Without a guide, in the tuning run, 9 of 24 single levels finished. These were recorded, not counted, as you decided.

You accepted every one of these misses. The seven levels that don't finish (W2-3, W3-2, W3-3, W3-4, W4-4, W6-1, W6-4) are listed in the plan's bench folder for after the merge. All of them come from two stalls. In one, the frog has its coins but stalls short of the exit. In the other, a repair can't rejoin its old plan.

**Not delivered.**
- **A fair head-to-head with the old tools:** that comparison is only the early six-level sample, taken on a busy machine (the old tools finished 0 of 6). The full comparison was dropped to finish sooner, and it can't be redone now that the old tools are gone.
- **The movement map:** it is built, but only as an optional sense of direction. The trial found it helped almost nowhere.

**Engine fixes found along the way**, each with a test against the original:
- Coins were taken a frame early while the original was running slow, which showed up as a sound mismatch.
- The frog was freed from a passage one frame early.
- Two speed-ups that change nothing the game does: the engine skips drawing the picture when planning, and it looks up its memory faster.

## Decisions made for you

- **T19: A new task, T19 (step-unseen-no-picture), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T04: 11 of 24 maps take over 5 minutes to build on a quiet machine (worst W5-4, 566 s). Most of the time is the engine searching a table on every memory access. How should this be handled?** (night)
  Answer: 11 of 24 maps take over 5 minutes to build on a quiet machine (worst W5-4, 566 s). Most of the time is the engine searching a table on every memory access. How should this be handled? → Add engine speed task: add it as a new task (speed only, no behaviour change, like T19) that blocks T20; its Done-when must show the full conformance cases unchanged and record per-level build times for all 24 maps. If builds are still over 5 min after it, record them in FINDINGS and accept them; do not shrink the move set. Finish T04 without waiting for it.
  Why: Night mode. Mirrors T19, a speed-only engine change already accepted; keeps map coverage, which the 24/24 target needs; reversible by git and gated by conformance.
- **T20: A new task, T20 (map-gaps), added to the plan by a worker** (night)
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T21: A new task, T21 (engine-memory-lookup), added to the plan by a worker** (night)
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T09: How should a guide path treat switches? Recommended: guide points also carry which switches had been pressed (known when the guide comes from a route; a hand-written waypoint matches any), which made W6-2 exit on 3 of 3 seeds in about 2 minutes.**
  Answer: How should a guide path treat switches? Recommended: guide points also carry which switches had been pressed (known when the guide comes from a route; a hand-written waypoint matches any), which made W6-2 exit on 3 of 3 seeds in about 2 minutes. → Add switches
  Why: Mirrors the existing optional coins_left field of DESIGN 2.11 (unknown matches any), so hand waypoints are unaffected; T10/T12 not built yet; W6-2 is the level 2.11 exists for. Add the clause to DESIGN 2.11 and record the W6-2 result in FINDINGS.
- **T10: W1-4 guided can't be finished because the search stalls near its exit, even without the original involved. How should T10 handle this?**
  Answer: W1-4 guided can't be finished because the search stalls near its exit, even without the original involved. How should T10 handle this? → Follow-up task
  Why: The stall is in T08's search, not the oracle check, so fixing it in T10 breaks scope; an owned task blocking T14 keeps W1-4 (an M2 level) from reaching the counted runs open. Add T23 with a blocks T14 clause, record the stall in FINDINGS, and mark W1-4 unmet in T10's notes.
- **T23: A new task, T23 (w1-4-ending), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T23: DESIGN §2.11 says a guide point counts only for a frog with the same number of coins left as the route had there. The W1-4 fix also counts it for a frog with fewer coins left (it is ahead on coins). With it, W1-4 passes the check against the original in 629 s; without it, it never exited. Approve this change to the rule?**
  Answer: DESIGN §2.11 says a guide point counts only for a frog with the same number of coins left as the route had there. The W1-4 fix also counts it for a frog with fewer coins left (it is ahead on coins). With it, W1-4 passes the check against the original in 629 s; without it, it never exited. Approve this change to the rule? → Approve
  Why: A frog with fewer coins left than the guide is ahead of it, so matching it serves 2.11's intent; affects guided runs only; it is what makes W1-4 pass on the original. Update DESIGN 2.11's band clause to say so, and record the deviation plus seed 3's miss in FINDINGS for T14.
- **T14: Route length is the main miss so far (set A routes run 30–40% over the committed ones; the target is at most 25%). I tried one general change: when the guide is a played route, the planner already knows how fast that route got to each point, so places the search reaches later than the route did get picked less often (the weight halves at about 170 frames behind). Engine-only trial on W1-2, W2-1 and W1-3, three seeds each: routes 18% shorter in total; W1-3 went from 8 577 / 9 577 / 5 791 frames to 4 758 / 4 905 / 7 558 (its committed route is 7 729) and planned faster. One of nine runs got worse (W2-1 seed 3, 2 377 → 2 859). DESIGN §2.11 spells the guide weight out exactly, so this adds to a written rule. Hand-written waypoint files carry no timing, so they are unaffected. If you say nothing I will keep it on and record it as a deviation.**
  Answer: Route length is the main miss so far (set A routes run 30–40% over the committed ones; the target is at most 25%). I tried one general change: when the guide is a played route, the planner already knows how fast that route got to each point, so places the search reaches later than the route did get picked less often (the weight halves at about 170 frames behind). Engine-only trial on W1-2, W2-1 and W1-3, three seeds each: routes 18% shorter in total; W1-3 went from 8 577 / 9 577 / 5 791 frames to 4 758 / 4 905 / 7 558 (its committed route is 7 729) and planned faster. One of nine runs got worse (W2-1 seed 3, 2 377 → 2 859). DESIGN §2.11 spells the guide weight out exactly, so this adds to a written rule. Hand-written waypoint files carry no timing, so they are unaffected. If you say nothing I will keep it on and record it as a deviation. → On by default
  Why: Extends D21's accepted use of committed routes as guides, guided runs only, waypoints unaffected; 18% shorter is the main lever on M4. Add the pace term to DESIGN 2.11 and record it as a deviation in PROGRESS and FINDINGS, noting it ties guided route length more closely to the committed route.
- **T14: W5-3: the planner reached the exit on the original, but the faithfulness check rejected the route. While the original is busy it can end a frame halfway through its work; the check cannot compare such frames, so it fails any route where that happens more than 100 frames in a row (a few levels' completion cases allow more, because their busy stretches run longer: W3-1 105, W6-1 133, W4-3 261). The planner never looks at this, so it can hand in a route the check will refuse. How should it handle it?** (night)
  Answer: W5-3: the planner reached the exit on the original, but the faithfulness check rejected the route. While the original is busy it can end a frame halfway through its work; the check cannot compare such frames, so it fails any route where that happens more than 100 frames in a row (a few levels' completion cases allow more, because their busy stretches run longer: W3-1 105, W6-1 133, W4-3 261). The planner never looks at this, so it can hand in a route the check will refuse. How should it handle it? → Bench passes the limit, with one change: the planner looks up the limit itself from the level's own completion case (the same bar the faithful check applies, as in DESIGN 4.1), falling back to 100 when the level has none, so neither the benchmark nor an agent passes a per-level value. Keep the option only as an override that sf-bench never sets. Record it in FINDINGS and DESIGN 2.6 as a deviation.
  Why: Night mode. Matching the check's own bar is the only way the planner stops handing in routes the check refuses; reading it from the case keeps the stance of no per-level options in the benchmark. Reversible.
- **T14: T12's app-mode test (W1-1, seed 1) now loses a heart because of T14's guide pace, which is on by default (your choice of 2026-10-22). On three seeds, hearts lost come out even with pace on or off, but with pace on app-mode routes were 15–56% longer. How should I settle it?**
  Answer: T12's app-mode test (W1-1, seed 1) now loses a heart because of T14's guide pace, which is on by default (your choice of 2026-10-22). On three seeds, hearts lost come out even with pace on or off, but with pace on app-mode routes were 15–56% longer. How should I settle it? → Pace off in app mode
  Why: Pace-on-by-default was the coordinator agent's call on engine-only set-A evidence; app-mode evidence now says it lengthens routes against C4's 25%, so it is narrowed to where it helped. Picking a seed to suit the test would hide a real effect. Record the narrowing in DESIGN 2.11 and FINDINGS; T25 may revisit app-mode pace on more seeds.
- **T24: A new task, T24 (repair-cost), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T25: A new task, T25 (tuning-set-a), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T26: A new task, T26 (tuning-set-b), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T27: A new task, T27 (tuning-unaided), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T28: A new task, T28 (compass-trial), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T24: Keep T24's change: a repair plans about 1,000 frames ahead along the last good plan, instead of all the way to the exit as the design says?**
  Answer: Keep T24's change: a repair plans about 1,000 frames ahead along the last good plan, instead of all the way to the exit as the design says? → Keep it
  Why: A general fix (no per-level option) for T24's pattern 2, turning four no-route levels, three of them M2 levels or set A, into passes; length is T25's to trim. Update DESIGN 2.6 to match and record before/after in FINDINGS. Pattern 1's hearts-for-completion trade remains the owner's, as T24's doc says.
- **T25: T25's planner fix trades one level for another. When the original game runs slow for a long stretch, the planner has to back up and try again. T25 changed how far back it goes: W5-2 now finishes (4.3 min), but W1-4, which finished in 9.2 min under the old rule, now fails its 30-minute budget. T25's results still list W1-4 as done because that run used code from before the fix. I tried two other rules in review and each one finishes one of the two levels and loses the other. Which way should T25 close?**
  Answer: T25's planner fix trades one level for another. When the original game runs slow for a long stretch, the planner has to back up and try again. T25 changed how far back it goes: W5-2 now finishes (4.3 min), but W1-4, which finished in 9.2 min under the old rule, now fails its 30-minute budget. T25's results still list W1-4 as done because that run used code from before the fix. I tried two other rules in review and each one finishes one of the two levels and loses the other. Which way should T25 close? → Old rule back, add task. Re-run W5-2 on the reverted code and record it as a miss with this cause; the new task (blocks T15) finds a back-up rule that finishes both W1-4 and W5-2, keeping the two failing rules you tried as its starting notes. If the owner approves T25's separate 'repair follow-up' task, the two should be one task, so name it to cover both.
  Why: W1-4 is one of M2's four stall levels, the plan's headline target; reverting also makes T25's recorded results match the code that produced them. Either way a task owns the trade-off, so the choice is cheap to undo.
- **T29: A new task, T29 (engine-sound-ramp), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T30: A new task, T30 (lag-repair), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T26: How should W2-2's set-B leg be handled, given it can't pass the faithful check from its current start for any route?**
  Answer: How should W2-2's set-B leg be handled, given it can't pass the faithful check from its current start for any route? → Add a fix task. Its goal must say the fix is to where and how the leg's start is synced, never a loosening of the faithful check's tolerances (DESIGN 1 Stance, 8); if the only fix found would loosen the check, that task stops and asks the owner. It blocks T17; T26 records W2-2 as waiting on it.
  Why: M5 is 24/24 and no route can pass from the current start, so it is a start defect, not a planner miss; approving the task is reversible, and the guard keeps the faithful bar untouched.
- **T26: Five set-B legs (W2-3, W2-4, W3-2, W3-3, W4-4) fail because a repair can't rejoin the old plan, and the whole-rest re-plan that follows never finishes. How should they be handled?** (night)
  Answer: Five set-B legs (W2-3, W2-4, W3-2, W3-3, W4-4) fail because a repair can't rejoin the old plan, and the whole-rest re-plan that follows never finishes. How should they be handled? → Record as misses. Write one FINDINGS row naming the five legs, the cause, and that it is the same pattern as set A's W2-3, W3-4, W6-1 budget misses (FINDINGS 2026-10-23), so the owner can choose in the morning whether one fix task should cover both sets. Carry on with the rest of T26.
  Why: Night mode. The owner chose on 2026-10-23 to record set A's misses of this same pattern and carry on; the proposed task would re-open that for set A too. Recording now is the easiest to undo: a fix task can still be added before T17 starts.
- **T26: W1-4, W6-1 and W6-4 in set B never find a first exit (W1-4 at the old dead end, W6-4 13 cells short, W6-1 one coin short). How should they be handled?** (night)
  Answer: W1-4, W6-1 and W6-4 in set B never find a first exit (W1-4 at the old dead end, W6-4 13 cells short, W6-1 one coin short). How should they be handled? → Record as misses
  Why: Night mode. Consistent with the owner's 2026-10-23 choice to record budget misses and carry on; W6-4 and W1-4's twin are already in T28; recording is the easiest to undo, since a fix task can still be added before T17.
- **T31: A new task, T31 (leg-start-sync), added to the plan by a worker** (night)
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T31: W2-2 and W5-1 now fail only because, when our engine replays the planned route on its own, it drifts one round of game logic ahead of the original partway through the level. How should T31 close?**
  Answer: W2-2 and W5-1 now fail only because, when our engine replays the planned route on its own, it drifts one round of game logic ahead of the original partway through the level. How should T31 close? → Misses + follow-up task. The new task's goal: find why the engine drifts a round ahead; prefer fixing the engine with a conformance case that fails without the fix (as T29 did); never loosen the faithful check; if the cause is not an engine defect, make the planner reject such routes, or bring the choice to the owner.
  Why: Same shape as T29, which the owner approved: an engine mismatch against the original that blocks set-B legs, fixed narrowly with a conformance case. T31 stays in scope; the task is cheap to drop before it starts.
- **T32: A new task, T32 (alone-pass-drift), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T33: A new task, T33 (dead-end-repair), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation
- **T15: W3-4 and W6-4 missed on set C: each got every coin it needs and then ran out its 30 minutes 45–48 cells short of the exit, alone or not. What should happen to them?** (night)
  Answer: W3-4 and W6-4 missed on set C: each got every coin it needs and then ran out its 30 minutes 45–48 cells short of the exit, alone or not. What should happen to them? → Record as misses. In the FINDINGS row, note that W6-4's near-exit stall now shows on sets A, B and C and W3-4's on A and C, so the owner can weigh one fix task across sets in the morning.
  Why: Night mode. Matches the owner's 2026-10-23 choice to record budget misses and carry on; the easiest to undo, since a fix task can still be added before T17; holding T16 overnight on a quiet machine would waste it.
- **T15: The set-C baseline of the old tool (plan_exit) has 9 levels left after the two now running, about 2 more hours. With D27 aiming to finish sooner, finish it or stop?**
  Answer: The set-C baseline of the old tool (plan_exit) has 9 levels left after the two now running, about 2 more hours. With D27 aiming to finish sooner, finish it or stop? → Stop at 15 levels. Let the two running levels finish, label the column 15 of 24 run, and name the nine not run in RESULTS and FINDINGS (D27).
  Why: D27 (owner, 2026-10-25): no target is gated on the baseline, it is context, and finishing sooner was chosen over a complete comparison at a far larger cost; C1/C3/C4 are unaffected.
- **T18: The levels that don't finish (M1 21/24, M5 18/24, C1 22/24) all come from the same two stalls. What should happen to them? / Only 10 of 24 set A routes lose no heart, against a goal of 20 (M6). What should happen? / Four routes are more than 25% longer than the reference (M4), and one failed level overran its 30 minutes by 18 seconds (M3). Accept both?** (night)
  Answer: The levels that don't finish (M1 21/24, M5 18/24, C1 22/24) all come from the same two stalls. What should happen to them? → Record the misses with their causes, but do not delete the old tools tonight. D5 deletes them only once M1 holds or the owner accepts the miss, and the owner's standing rule is that a level the planner fails twice goes to them to play the stretch (recorded, spliced into its guide). Set this aside as a follow-up task: add it as T34 retire-old-tools with this open question in its goal (owner accepts the misses, or plays the failing stretches first; then delete the old tools and recheck M9), finish the rest of T18 without the deletion, and carry on.; Only 10 of 24 set A routes lose no heart, against a goal of 20 (M6). What should happen? → Accept, fix in next plan; Four routes are more than 25% longer than the reference (M4), and one failed level overran its 30 minutes by 18 seconds (M3). Accept both? → Accept both
  Why: Night mode. Recording misses is reversible and matches the owner's run-long practice; deleting the old tools is the one step D5 and the owner's twice-failed rule leave to them, so it waits as T34 rather than closing it overnight. M6's fix fits the difficulty-modes plan.
- **T34: A new task, T34 (retire-old-tools), added to the plan by a worker** (night)
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation

## Context splits

- T14 split at 500k (peak 527k) → added T24, T25, T26, T27, T28

## What to check by hand

Nothing in this plan needs your eyes to count as done: every target is judged by machine. Three things are worth doing before you merge:

- **Check that the tests are green.** The final test run on the build's branch failed, and a worker then fixed it. Run the project's test suite once on the branch and expect it to pass. It takes about 50 minutes, so it's best run on a quiet machine. One drawing test (W1-4) is known to fail at random when the machine is busy; if that's the only failure, run it again.
- **Try the planner on a level the old tools struggled with,** for example W1-4 guided by its existing route, in original mode. Expect a finished route within 30 minutes, which passes the check against the original.
- **When you get to the failing-levels list after the merge,** your own rule applies: you play the stuck stretch, and it is recorded and stitched into the level's guide. W6-4 is the one to start with, because its stall shows up in all three sets.

## Risks and follow-ups

- **The counted timings ran on a busy machine.** Other sessions kept the Mac loaded, so you let the counted run go ahead anyway, with the load written beside each level. Planning times and the 30-minute limit therefore mean less than the plan intended. Some levels may finish on a quiet machine.
- **The headline numbers lean on existing routes.** Every counted level was guided by the route already in the repo. Without a guide, only 9 of 24 finished. A level that has no route yet, such as a map-editor level, is the unproven case, and the next plan needs exactly that.
- **Hearts are the weakest result** (10 of 24 routes lose none). When a repair takes a hit, it never tries to win it back. The difficulty-modes plan needs routes that never get hit, so this is the first thing to fix there.
- **Two stalls account for all seven failing levels.** Neither has a fix in this plan.
- **A few design notes are out of date:** the guide's weighting is now different between original mode and app mode, the default direction finder is now the wall-aware fill, and route shortening now keeps fewer lost hearts ahead of shorter length. The code and the findings log are right; parts of the design text still read the old way.
- **Smaller loose ends, all in the findings log:**
  - The benchmark writes this machine's folder path into its failure messages.
  - The planner's starting state for two whole-game legs (W3-1, W4-2) begins partway through a frame. Both still finished.
  - The repo's stored test setting runs only the Python tests, not the engine's.
  - The ordered-guide setting that finished W6-4 in your video trial is on a branch that hasn't been merged.

## Branch

Synced with `main` at `13b9175e39d8` on 2026-10-26T09:28:51Z.
The tests were red at the end; a worker fixed them.
Tests: red. The branch is not ready to merge.
