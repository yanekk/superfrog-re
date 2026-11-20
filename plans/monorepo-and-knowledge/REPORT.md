# monorepo-and-knowledge — delivery report

## What was delivered

The repository is now one repo holding three projects, still named superfrog-re.

- **New layout.** Superfrog lives in its own folder under `games/`, the shared Amiga toolkit has its own folder, and the top level keeps only what is shared. Superfrog's behaviour was not meant to change, only where its files live.
- **Knowledge moved out of Claude's private memory.** Every fact now sits in the repo at one of three levels: how you work, the toolkit, or one game. The memory folder holds only a pointer. A new check fails if a game's name appears where it should not.
- **Tests follow the change.** A change to one game runs that game's tests, a toolkit change runs the toolkit's and every game's, a plans-only change runs nothing, and anything doubtful runs everything. The full test run on the finished build passed.
- **Ready for game two.** The toolkit now has a playbook of the steps Superfrog took and a starter kit for a new game. Forest Dumb Forever has its starting folder, with stub tests only. You decided it will be judged against the cracked two-disk release; its actual reverse-engineering is the next plan, not this one.
- **Added during the build.** The general parts of the six hardware chapters were carved into the toolkit as a new fifth task, because the chapters as written named Superfrog throughout.

**Not delivered:** your four Superfrog disk zips have not moved into the game's folder. See below.

## Decisions made for you

- **T02: I'm moving Superfrog into its new folders (task T02). The plan says six architecture chapters (display hardware, CPU time and beam, frame interrupts and input, memory model, sound chip, verification) go to the toolkit. But those chapters name the game on 322 lines, and they describe game files as their own, so the check that keeps game names out of the toolkit fails. What should happen to them? If you don't answer, I wait. / The task doc for T02 says to run every test suite on this branch (about an hour) and only then move your disk zips. My standing worker rules say a worker must never run the whole suite, because pir runs it once after all tasks merge. Which applies to T02? No other worker is running alongside this task, so the machine-overload reason doesn't apply here. If you don't answer, I wait.**
  Answer: I'm moving Superfrog into its new folders (task T02). The plan says six architecture chapters (display hardware, CPU time and beam, frame interrupts and input, memory model, sound chip, verification) go to the toolkit. But those chapters name the game on 322 lines, and they describe game files as their own, so the check that keeps game names out of the toolkit fails. What should happen to them? If you don't answer, I wait. → Keep game-side, split later (Recommended). Approved: the six chapters stay in games/superfrog/docs/architecture/, with architecture/README.md and the chapter validator adjusted to match, and record the departure from the T02 doc in PROGRESS and your commit message. Add one new task, depending on T02 and blocking nothing, to carve the generic parts into toolkit/docs/architecture/ under the name check. Do not mark lines 'learned on:' to pass the check.; The task doc for T02 says to run every test suite on this branch (about an hour) and only then move your disk zips. My standing worker rules say a worker must never run the whole suite, because pir runs it once after all tasks merge. Which applies to T02? No other worker is running alongside this task, so the machine-overload reason doesn't apply here. If you don't answer, I wait. → Run it in T02 (Recommended)
  Why: Q1: rule 1 (lowest tier where true) and rule 3 outrank a plan-time placement that the plan's own name check rejects; the marker option would hollow the rule, and an in-task rewrite of 1,650 lines risks an atomic move. Q2: T02's doc and DESIGN 5.2/5.3 make the green full suite the explicit precondition for moving the zips, and the build is serial.
- **T02: T02 (moving Superfrog into its new folder) is otherwise sound, but two of its finish items are not done: the whole test suite has not run on this branch, and your four disk zips are still at the top of the main checkout. The builder's notes say you stopped the suite run and asked for it to run after the plan is built. pir runs the whole suite at the end by itself, but nothing in the plan or the finishing rules moves the zips afterwards. They would stay put until someone runs the move script; everything still finds them there meanwhile. How should I close T02?**
  Answer: T02 (moving Superfrog into its new folder) is otherwise sound, but two of its finish items are not done: the whole test suite has not run on this branch, and your four disk zips are still at the top of the main checkout. The builder's notes say you stopped the suite run and asked for it to run after the plan is built. pir runs the whole suite at the end by itself, but nothing in the plan or the finishing rules moves the zips afterwards. They would stay put until someone runs the move script; everything still finds them there meanwhile. How should I close T02? → Accept, track it (Recommended)
  Why: The disk lookup's root fallback keeps everything working with the zips unmoved, pir's end-of-build suite still proves the layout, and the zip move stays gated on that green suite as DESIGN 5.2 intends. A follow-up task would move them before the proof; rerunning the suite would override a stop the builder reports the owner gave.
- **T05: A new task, T05 (architecture-chapters-carve), added to the plan by a worker**
  Answer: added to the plan and built
  Why: a worker proposed it during the build; the coordinator agent or you approved it in the worker's conversation

## What to check by hand

**Move the disk zips, after you merge.** No session ran this, because the whole test run had to pass first, and that only happened at the end. It no longer appears on the progress file's waiting list, so this report is the reminder. From your main checkout:

`games/superfrog/tools/move-disks.sh`

Expect it to list the four zips it moved. It is safe to run twice. To undo it, move the zips back to the top of the checkout. Until you run it, everything still finds them where they are, so nothing breaks.

There is nothing else you need to check by hand.

## Risks and follow-ups

- **Your existing bug recordings stay where they are.** New ones save in the game's folder and the old ones remain at the top of the main checkout. The app reads the game's folder if it exists, otherwise the top-level one, so once the new folder exists, older recordings may need moving by hand to appear. The move script handles disks only.
- **A partly moved disk set confuses the tools.** With only some zips in the new folder, different tools react differently. Run the move script once and all four move together.
- **The original memory files are gone.** The twelve notes now exist only in the repo, so that is where any way back starts.
- **Forest Dumb's measuring stick is the cracked release.** It may differ slightly from the original, so compare again if an uncracked copy turns up. The sources also disagree on its year (1996 or 1998); the game's first plan can settle that.
- **Slower tests for a while on some branches.** A branch cut before this merge will see the whole move as its change and run everything, about an hour.

**Small follow-ups, left alone during the build:**
- The top-level README and rules file do not yet mention Forest Dumb's folder, and the README does not mention the playbook or starter kit.
- A comment on one line of the ignore file stops generated renders from being ignored, so they could be committed by accident.
- One hardware chapter cites two tests that do not exist under those names; the docs check looks at file paths, not test names.
- The game-name check is slightly over-strict: it would also flag text like "steam 17". It fails loudly, so it cannot let anything through.

## Branch

Synced with `main` at `0861665f336b` on 2026-10-30T20:25:30Z.
Tests: green.
