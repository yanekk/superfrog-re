# forest-dumb-title — delivery report

## What was delivered

Everything a player can do before and between the levels is now checked by machine against the original game, and the rebuild matches it.

- **The title screen.** The disk 2 prompt, the title with its music through a full loop, and moving between START, KODY and KONIEC all match the original frame by frame.
- **Level codes.** The original has codes for worlds 1, 3 and 5 only, plus a cheat code that starts world 1 with the cheat on. Worlds 2, 4 and 6 have no code. Each code, a wrong code and the cheat are checked against the original.
- **Quitting.** KONIEC quits, and so does Esc on the title. The old notes said Esc did nothing there; the original actually quits.
- **Game over.** Dying to game over, typing a name into the high scores and returning to the title all match. The original keeps high scores in memory only and never saves them to disk, and so does the rebuild.
- **Start Level… in the app.** It now offers worlds 1 to 6 and a "Cheat on" box. The command line can do the same: pick a world from 1 to 6 to start in, and turn the cheat on with a separate switch.
- **Bug recordings.** A recording now notes which world it started in and whether the cheat was on, and it replays to the same place. Older recordings still replay.
- **Real key presses in the app.** The title, codes, quitting and game over were driven in the app by real key presses, at three window sizes, and matched a run without the window at every step.
- **Bugs found and fixed.** The rebuild handled the keyboard's timing slightly differently from the original. That made codes, Esc and the title's timing drift a frame or more. It also counted a digit the original drops when you type fast and two keys overlap. All of these now match the original.

**Not delivered:**
- Comparing a recording made from a world start against the original. It was out of scope; such recordings only replay.
- The levels' own play beyond each world's first moments. That belongs to each level's plan.

## Decisions made for you

- **T03: I wrote the two title checks and both pass on the translated game: the idle title through the music's first full loop, and moving the menu cursor down and back up. Each check has a finish line it must reach: the music back at its first position, and the cursor back on START. Both are already true on the very first frame, so the finish line is ticked at once and proves nothing. Every frame along the way is still compared and matches. How should I handle the finish line?**
  Answer: I wrote the two title checks and both pass on the translated game: the idle title through the music's first full loop, and moving the menu cursor down and back up. Each check has a finish line it must reach: the music back at its first position, and the cursor back on START. Both are already true on the very first frame, so the finish line is ticked at once and proves nothing. Every frame along the way is still compared and matches. How should I handle the finish line? → Add 'not before'
  Why: The task doc already asks for mt_songpos back to 0 'after its last position' and the cursor at START 'at the end'; a goal met on frame 0 does not implement that. Keep the addition general and optional (existing cases unchanged), unit-test it in the case parser, confirm run-all reads as before, and add a FINDINGS row so T04 to T06 can use it.
- **T10: The short title test (rows plus one code, 13 s) needs the translated program, which the quick test set's fresh checkout doesn't have. Where should it go?**
  Answer: The short title test (rows plus one code, 13 s) needs the translated program, which the quick test set's fresh checkout doesn't have. Where should it go? → Nightly set
  Why: A smoke test that fails in every fresh checkout breaks the end-of-plan gate, and building the translated program in smoke costs 1-2 min of a budget at 842/900 s plus a plan change. Follow the translator plan's rule: label both files full, no [[area]]. Record the deviation from T10's 'short test in smoke' in FINDINGS and in your commit, naming that the title paths are checked nightly only.
- **T11: The Mac's screen is locked, so the real-window part of the drill (the Start Level… window clicked through the OS at three sizes) cannot run. Asking the owner to unlock it; everything else is built, committed and green.** (night)
  Answer: The owner is away until morning and only they can unlock the screen, so keep the task open and wait: do not mark the OS-layer Start Level… check done, and do not split it into a follow-up task, since it is three minutes of the owner's time and belongs to this drill. Make sure PROGRESS.md shows T11 with its automated half green and the real-window half unchecked (screen locked), then end your turn. When the owner says "unlocked", run the bounded test as you described and finish. Everything else you reported stands: the key-order fix with its failing-without test, the Up+Down finding, and the pre-existing title-quit-while-recording timeout logged in FINDINGS and left alone.
  Why: Needs the person's hands on the real machine; a worker waiting until morning is acceptable and costs nothing to undo, whereas a follow-up task would close the drill with its real-window half unseen.

## What to check by hand

Nothing is left that only you could check: the one real-window check, the Start Level window clicked at three sizes, ran and passed once you unlocked the screen.

If you want to see it yourself, open the game, choose Game ▸ Start Level…, pick world 3 with "Cheat on" ticked, and start it. You should land at the start of world 3. On the title, choose KODY and type 7531, then START. That should also take you to world 3.

## Risks and follow-ups

- **World 1 by code differs when level 1's own engine takes over.** If you reach world 1 by typing a code and the rebuilt level 1 is plugged in, the very first frame of play is off: the camera is in the wrong spot. The translated game on its own is correct. The open level 1 mismatch belongs to the level 2 plan, so this was logged there and left open.
- **Holding HELP with the cheat on can stop the rebuilt game.** Holding HELP for 5 or more frames to skip a level stops it as the level ends. The original handles that moment inside the Amiga's system code, which the rebuild does not copy. Copying it is a larger choice that the plan left to you. A quick tap works.
- **Up and Down held together on the title** behave differently: the original keeps only Down. You chose to leave this during the level 1 plan; it is still open.
- **The title tests in the app run nightly, not on every change.** I decided this on your behalf: they need the translated game built first, and the quick check doesn't do that. A break in the title screen inside the app would be caught the next night, not at the end of a plan.
- **The quick check is close to its time limit.** It took about 767 of its 900 seconds.
- **The tests broke at the end of the run, and a worker fixed them.** One check that keeps the core code self-contained failed, and one test flickered. The fixes landed after the tasks were reviewed.
- **Two side notes for later plans.** Superfrog and the rebuilt level 1 may have the same fast-typing difference that was fixed here; nobody has checked yet. One code checker reports an old fault that nothing currently catches.

## Branch

Synced with `main` at `8e93ef32bc10` on 2026-11-21T07:29:10Z.
The tests were red at the end; a worker fixed them.
Tests: green.
