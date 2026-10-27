# amiga-chips — delivery report

## What was delivered

Both tasks are built and reviewed. **The final test run was red, so this branch is not ready to merge yet** (see Risks).

- **One copy of each Amiga chip piece.** The engine had several copies of the same pieces. There were three different ways of copying graphics around in memory (the blitter), nine places that each worked out the size of such a copy, two identical checks for whether a hardware register belongs to the sound chip, and three separate stand-ins for the Amiga's memory. Each is now a single copy. One copy-size reader now covers what was nine scattered copies; the plan's target counts it as 3, the shared copy plus two the plan deliberately leaves alone.
- **A new toolkit package, amiga-chips.** Those single copies, plus the code that times graphics changes to the TV beam (the copper), now live in their own package. Nothing in it mentions Superfrog, so a second Amiga game could use it as it is. It sits beside the existing sound-chip package as the toolkit's second piece.
- **Nothing about the game is meant to change.** This was a tidy-up only. The project's architecture notes and its progress log for the toolkit work record the step.
- **One behaviour was found and kept on purpose.** The level's graphics copies handled a rare memory-boundary case differently from the general copier. The merged copier keeps the level's way for the level, so nothing shifts.

Not delivered: none of the plan's own scope was dropped. The full whole-game comparison and the whole-game app test were outside this plan from the start.

## Decisions made for you

- **T01: Two blit-size decodes came in with the widescreen work after this plan was written. Should T01 switch them to the one shared helper so the count reaches the plan's 3?**
  Answer: Two blit-size decodes came in with the widescreen work after this plan was written. Should T01 switch them to the one shared helper so the count reaches the plan's 3? → Fold both in (Recommended)
  Why: The plan's goal is one decode and its finish line (3) was locked by the owner; the widescreen copies postdate the plan. render/bobs.rs is identical. render/mod.rs differs only for a BLTSIZE field of 0, where the helper's 1024/64 rule makes the placement rectangle agree with what render/bobs.rs already draws for that frame; no game frame should have a zero size. Conditions: record the two extra files as a deviation in the commit and a FINDINGS row naming the size-0 edge; every DESIGN §4 run, complete-w1-1 included, must stay green with no tolerance touched. If any goes red, revert render/mod.rs's change and finish at 4 as a recorded deviation instead.

## What to check by hand

Nothing in this plan needs you to look at the screen or play. It is an internal tidy-up, and the machine checks it against the original game.

Before you merge, do one thing: **make sure the test suite is green.** The final run pir recorded was red. Ask for a fresh full test run on the branch and merge only if it passes.

If you'd like reassurance that the game still plays the same, the quickest sign is the level 1-1 playthrough comparison you added to this plan. Ask a session to run it and confirm it matches the original.

## Risks and follow-ups

- **The tests ended red.** The failures were two slow tests that start the real app and gave up after their 2½-minute limit while the whole suite ran at once. The same failure was logged once before, during the widescreen work on 2026-10-22. Run on their own, those tests took 42 seconds on this branch, against 68 seconds on main, so this plan did not make them slower. The repair session raised their limit to 10 minutes, the limit other long app tests already use, and reported a full green run afterwards. pir's own final check still came out red, so treat the branch as unproven until a clean run confirms it.
- **The comparison runs against the original are not written down.** The plan asked for slow side-by-side runs against the original game: the drawing cases, the attract screens, the story scenes and the level 1-1 playthrough. Their pass or fail was to be recorded in the progress file, and it is empty. The tasks passed review, but I can't point you to where those runs were recorded. That gap is worth closing before you merge.
- **A tiny edge-case change, agreed during the build.** One of the two copy-size readers added by the widescreen work now treats a graphic of recorded size zero as full size, the way the Amiga itself does. No real graphic has size zero, and the comparisons were required to stay green.
- **A measurement moved, not the game.** Moving the copper code also moved a piece the toolkit's progress script counts. One of that script's counts now reads 5 instead of 6, though nothing was merged there. The next plan should count across the whole engine.
- **Follow-ups:** the two progress-log questions for you (marked "not asked") are still open, and the toolkit's next steps are separate plans.

## Branch

Synced with `main` at `9078e1d49c2e` on 2026-10-27T15:51:38Z.
The tests were red at the end; a worker tried to fix them and they stayed red.
Tests: red. The branch is not ready to merge.
