# engine-architecture — delivery report

## What was delivered

**Delivered: a full written map of the engine, in a new `docs/architecture/` folder, linked from the main README.** No engine code changed, as the plan intended.

- **Seventeen chapters, one for each part of the engine.** Each one says how much of that part pretends to be the Amiga, how much is Superfrog's own rules, how much is data copied in from the original disks, and how much is the modern app and test tooling around it. Each also says what would have to stay exactly as it is if the part were rewritten in a modern style, and which existing check proves that it did.
- **A reuse rating for every part**, judged against another Amiga game. Counted across the whole engine (about 111,000 lines of code today): about **21%** carries over to another Amiga game as it is or with small changes, about **16%** carries over as an approach but would be largely rewritten, and about **63%** is Superfrog's own. This confirms the earlier rough estimate of about 20%. What carries over is mostly the testing machinery that compares the engine with the original, the app, and the sound-chip imitation.
- **How much of the engine is Amiga imitation:** leaving the testing tools aside, more than a third of the game and app pretends to be the Amiga, and it is spread through every part rather than kept in one layer.
- **A recommended order for the later modern rewrite**, with a reason for each step. The first step is a decision only you can take: the tests compare the engine with the original by reading values at the original machine's memory addresses, and a modern rewrite has to keep that, translate it, or change the tests. The overview sets out the options and does not choose between them.
- **An automatic check** that fails if any engine file is not covered by a chapter, if a chapter is missing its ratings, or if a chapter points at a file that does not exist. The next plan that adds an engine file will be told to document it.

**Not delivered:** a check that what the chapters say is true. The plan ruled that out as too much ongoing effort. Instead, a fresh reviewer checked each chapter against the code by sampling.

## Decisions made for you

None.

## What to check by hand

Nothing has to be checked by hand: no task had a half that only you could check, and every test passed.

If you want to read it, start with the overview, `docs/architecture/README.md` (also linked from the main README). It holds the reuse figures, the recommended rewrite order and the list of open questions. The chapters are written for the session that will plan the rewrite, not for you.

## Risks and follow-ups

- **The rewrite has to start with a decision for you.** The tests compare the engine with the original by reading values at the original's memory addresses. The app does this in two places too, and so does the check that follows the game's progress. A modern rewrite must keep that, translate it, or change the tests. The options are laid out for the next plan.
- **Some questions are still open**, and each chapter says how to answer them. Examples: how much of the picture depends on exact timing, whether the app mixing every frame at full length (when the game sometimes uses shorter frames) can be heard, and whether simpler timing would still pass the tests.
- **Some behaviour has no test at all**, so a rewrite could break it unnoticed. Examples: the mouse moving, several slot-machine outcomes (the free spin, a lost gamble), aborting the credits, the attract demo in normal play, and some unusual enemy cases.
- **Some code is duplicated.** The chapters found the same work done two to four times: the original game file is held in memory four times, the keyboard reading is ported twice, the blitter (the chip that copies graphics) is copied three times, and some enemies are duplicated. Merging them is natural work for the rewrite.
- **Some notes and comments are out of date** and were left alone, because this plan changes nothing outside the new docs. The project's start-here file describes collision, the system the engine uses to decide what is solid, in a way that is not how the engine works. That file and the plan's own task list also call the sound-timing task T213 open when it is finished. One note about the original's palettes is wrong, and several code comments are stale. One small slip in the copied-in-data chapter is named in the overview. These are worth a small cleanup.
- **A few pieces sit in one chapter but belong partly to another**, because each file is assigned to exactly one chapter. Each chapter links the other where that happens, so nothing is undocumented; the line counts are a little rough as a result.
- **The game's story sequence is never started from the app**, only from the tests. Whether that is intended is not recorded; worth a word from you.

## Branch

Synced with `main` at `d9a62509ebb2` on 2026-10-23T06:48:37Z.
Tests: green.
