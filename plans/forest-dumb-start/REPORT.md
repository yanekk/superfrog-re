# forest-dumb-start — delivery report

## What was delivered

All ten tasks are built and reviewed, and the full tests pass. Forest Dumb Forever is now open as game two, and our code is compared with the original game automatically for the first time.

**What works now**

- **The original runs under the emulator with no one watching.** Your two disks boot from power-on to the title screen, and two runs with the same input match byte for byte on every frame. The emulator check, which could have stopped the plan, passed on the first Kickstart tried (1.3). The game asks for disk 2 to be swapped into the first drive, and the title first appears at frame 4921.
- **Every file on both disks is extracted and listed.** There are 61 files (12 and 49), and a committed list records their names, sizes and checksums. The established Amiga disk tool agrees on every name, size and byte. The disks and everything taken from them stay out of git.
- **The disks are documented.** That covers how they boot, how each file is packed, and the level and graphics formats. Files are unpacked using the original programs' own routines.
- **Every level map and sprite sheet has been drawn as a picture from the real data.** A worker opened and judged them, as the plan asks.
- **There is one shared map of the main program**, for all later porting work to start from.
- **The toolkit no longer assumes a fixed frame length.** A game now supplies the value. Forest Dumb measures 313 lines, the same as before, so Superfrog behaves as it did.
- **The first comparison passes.** Our stub models one variable from the title screen, a counter in the game's music player. It matches the original on all 15,079 passes from frame 4922 to 20000, in about 35 seconds. Starting one frame too early is caught and fails, and a test covers that.

**What was not delivered**

- Nothing of the game is ported beyond that one variable. There is no playable game, window or sound; those are the next plans, as agreed.
- The file formats exist only in Python tools so far. The Rust readers come with the plan that uses them.
- The meaning of the values in the object maps is not understood yet, only how they group.
- Four front-end picture files (`_NAPIS`, `_CZOLOWKA`, `_OPCJE`, `_END`) were left undecoded, because they were outside the task's list.

## Decisions made for you

- **T03: The plan's design says every file name inside Forest Dumb's main program must be on the two disks. On the real files, all 49 data files are named and found, but three names are on neither disk: `FileName.exe` (default text in a buffer that is overwritten before each load), `HD_ID` (nothing uses it that I can find), and `DF0:ST` (a high-score save file; I found no code that calls the save, and if it ran, the game would keep asking for a disk). The task doc also expected the program to contain its own name, `FOREST_DUMB_FOREVER`; it does not, only the startup scripts do. How should the automatic check treat these? If nobody answers, nothing gets committed until someone does.**
  Answer: The plan's design says every file name inside Forest Dumb's main program must be on the two disks. On the real files, all 49 data files are named and found, but three names are on neither disk: `FileName.exe` (default text in a buffer that is overwritten before each load), `HD_ID` (nothing uses it that I can find), and `DF0:ST` (a high-score save file; I found no code that calls the save, and if it ran, the game would keep asking for a disk). The task doc also expected the program to contain its own name, `FOREST_DUMB_FOREVER`; it does not, only the startup scripts do. How should the automatic check treat these? If nobody answers, nothing gets committed until someone does. → Exact exception list (Recommended)
  Why: The rule exists to prove the reader and inventory agree with what the game names (DESIGN 2.4): all 49 data files do. An exact list keeps any new stray name failing, and tracing open calls is T06's job (DESIGN 2.7, T03 Interface). Deviation from 'every name' and from the task's own-name test approved, recorded in the docs.

## What to check by hand

Nothing needs your hands. No task has an unchecked hand-tested half, and the one step the plan left to you, copying the two disk zips into place, is done.

The zips must stay in `games/forest-dumb/` in the main checkout. Tests that need the disks fail, naming where they looked, if the zips are missing. On another machine, the same copy has to be made once.

## Risks and follow-ups

**What the passing tests do not show**

- **The first comparison only proves the two sides can be compared.** It covers one counter on an idle title screen, with no input changing after the start. A later case that uses input after the start needs a new test, because a mistake in how that input is timed would not show up today.
- **The level pictures have not been compared with the game's own screen.** A worker judged them by eye. One mistake already got past the first pass: every level was drawn sideways until the review caught and fixed it. Treat them as very likely right, not proven.

**Known limits for the next plans**

- **Comparing pictures during play will not work yet.** During a level, the emulator's captured picture holds only the 320x56 status bar. The title screen is not affected, but this has to be fixed before any picture comparison in a level.
- **The cracked release is the measuring stick, as you decided.** Any differences from an uncracked copy are unknown.
- **The sound code accepts only frames of 312 or 313 lines.** That is fine for this game, but a future game with another length needs it widened.
- **The game never saves high scores on these disks.** The save routine and one other leftover name have no reference in the program. So a faithful rebuild has no save to reproduce here.

**Follow-ups outside this plan's scope, noticed and left alone**

Reviewers found these in Superfrog's and the toolkit's code. None was changed, and none was shown to fail in practice.

- Superfrog's `atn.py` unpacker has no error handling, so it may return damaged input as plausible data.
- Two sums in Superfrog's comparison tool could crash on an absurdly long run.
- Two one-byte entries in Superfrog's program map could throw its listing out of step, as the same mistake did in Forest Dumb's map before review fixed it.
- One toolkit dependency test (`dry-fit`) still guards against leaning on the first game only, not on Forest Dumb.

The Superfrog listing entries are the ones I would check first.

## Branch

Synced with `main` at `eb1cfb8425ec` on 2026-10-31T14:58:38Z.
Tests: green.
