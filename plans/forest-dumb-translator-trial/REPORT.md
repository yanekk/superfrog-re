# forest-dumb-translator-trial — delivery report

## What was delivered

**The translator trial ran, it cleared the bar, and you said Go.**

- **A first translator now exists.** It reads Forest Dumb Forever's game program and writes Rust that does the same thing, one instruction at a time. No AI model is involved, so the cost of the work grows with the number of kinds of instruction, not the number of routines. It reuses Ghidra's own description of what each instruction does.
- **Every routine in the game's per-frame code (87 of them) was translated and checked against the original game in the emulator.**
  - 85 could be checked. 84 of those came out identical to the original on all 2 000 test situations each, with nothing fixed by hand and no hints. That is 98.8% against your 90% bar.
  - The two sound routines can't be checked by anyone, because level 1 is silent. They are listed by name and left out of the count, as you decided.
  - The one failure stops itself safely rather than going wrong silently. It is caused by the test's own made-up situations (see risks), not by the game's real behaviour.
- **A second check found the game never rewrites its own program code while it plays.** It covered level 1's 33 recorded play-throughs, about 81 000 frames, and found no change to any instruction the game ran. That is the other half of your bar.
- **On the first full run 75 of 85 passed (88%).** Nine of the ten failures were a mistake in Ghidra's description of two arithmetic instructions. The translator now corrects it in general, for every routine at once. The run of all routines takes about two minutes.
- **A report with the full per-routine table is committed, and your Go is recorded with its date.** The table also shows Muse's earlier hand-made translations beside it, for comparison only.

**Not delivered:**
- Stages 1 to 6 of the translator. They are the second plan, written once level 1 merges.
- Exact processor timing. This trial did not check it.
- One rare instruction (`nbcd`) is still not handled. It stops with a clear message if ever reached; it did not appear in this trial.

## Decisions made for you

- **T06: The trial command is built and its tests pass. But the real run over the game's 87 routines can't start: the translator (built in task T03) crashes on the game's real data. The export step (task T01) labels some relocated addresses as belonging to an instruction's result. The translator reads that label as 'the instruction's last input', overwrites the wrong value and crashes. Fixing it is four lines in the translator plus a test. With that fix, one routine already runs end to end and passes. Strictly, fixing the translator is the next task's job (T07, 'trial fixes'). But this task's 'done' requires one finished real run, which can't happen without the fix. Should I make the fix here? If you don't answer, I won't make the fix: I'll mark this task as blocked on it and hand it to T07.**
  Answer: The trial command is built and its tests pass. But the real run over the game's 87 routines can't start: the translator (built in task T03) crashes on the game's real data. The export step (task T01) labels some relocated addresses as belonging to an instruction's result. The translator reads that label as 'the instruction's last input', overwrites the wrong value and crashes. Fixing it is four lines in the translator plus a test. With that fix, one routine already runs end to end and passes. Strictly, fixing the translator is the next task's job (T07, 'trial fixes'). But this task's 'done' requires one finished real run, which can't happen without the fix. Should I make the fix here? If you don't answer, I won't make the fix: I'll mark this task as blocked on it and hand it to T07. → Fix it here (Recommended)
  Why: A general translator fix, not a hand edit (DESIGN 2.2), with a test on a made-up program as T07 requires. T06's done-when needs one real run, and T07 depends on T06 so nothing runs in parallel on that file. Record it as a deviation and as a FINDINGS row. If T03 is already reviewed, the commit carries an Escape: forest-dumb-translator-trial T03 trailer.
- **tests-fix: The end-of-build check for the translator trial fails in 4 places. None comes from the translator work. All four come from the level 1 work this plan pulled in on purpose at its start, and level 1's own build is still running. Here they are: (1) a docs check: two file paths are stale and two new toolkit pieces have no chapter. I fixed this here but haven't committed it. (2) A keyboard-controls test failed once and passed on a rerun, so it's flaky under load and needs nothing now. (3) A level 1 comparison test says one known gap ('hits_taken', for creature hits) no longer hides any difference, so the exception should come out. (4) A level 1 tool test fails because the springs task lists only one hand-written example, and the tool needs a second. Should I fix (1), (3) and (4) here, in level 1's files? If you don't answer, I'll do nothing and stay parked.**
  Answer: The end-of-build check for the translator trial fails in 4 places. None comes from the translator work. All four come from the level 1 work this plan pulled in on purpose at its start, and level 1's own build is still running. Here they are: (1) a docs check: two file paths are stale and two new toolkit pieces have no chapter. I fixed this here but haven't committed it. (2) A keyboard-controls test failed once and passed on a rerun, so it's flaky under load and needs nothing now. (3) A level 1 comparison test says one known gap ('hits_taken', for creature hits) no longer hides any difference, so the exception should come out. (4) A level 1 tool test fails because the springs task lists only one hand-written example, and the tool needs a second. Should I fix (1), (3) and (4) here, in level 1's files? If you don't answer, I'll do nothing and stay parked. → Fix nothing here: discard the uncommitted docs fix too. DESIGN section 8 puts any change to level 1's run or its plan files out of scope, and DESIGN section 7 already holds this branch until level 1 merges, when its own fixes come in through the re-sync. Do record each of the four failures in this plan's FINDINGS.md, one row each, naming the failing test and that level 1's build owns it, so the report states why the tests are red.
  Why: DESIGN 8 makes level 1's files and plan out of scope; DESIGN 7 means this branch cannot merge before level 1 anyway, so red now costs nothing and editing level 1's files mid-build risks conflicts with its own fixes.

## What to check by hand

Nothing here needs your eyes or hands. Your Go was the only part a person had to give, and it is recorded.

One thing to watch rather than check: **this branch can only merge into main after level 1's build has merged.** It carries level 1's unfinished work. Once level 1 merges, this branch should be brought up to date and its tests run again before you merge it.

## Risks and follow-ups

- **The tests are red, and every failure comes from level 1's work, not the translator's.** The four failures are:
  - a documentation check pointing at two moved files and two new toolkit pieces with no chapter;
  - a known-gap exception in a level 1 comparison test that no longer hides anything;
  - a level 1 tool that needs a second worked example for the springs task;
  - a keyboard-controls test that failed once under load and passed on a retry.

  I decided to leave all four to level 1's own build, which is still running and owns them. Expect them to clear when level 1 merges and this branch catches up. If they don't, they need fixing before this merges.
- **The one failing routine.** Its made-up test situations use a number far larger than the game ever does (33 against at most 4). That makes it write into its own code and stop. The game's real play never does this, as the second check showed. Fixing it belongs to stage 2, in the second plan.
- **Ghidra gets a few things wrong.** Its description of two arithmetic instructions, and of a flag set by compare instructions, does not match the real processor. The translator corrects both in general, and both are written down for stage 1 to check against an independent processor emulator.
- **A routine that calls itself forever crashes the checking tool** instead of being reported as stuck. No routine in the trial did this, but a later level might. A limit on how deep calls can go is the fix, and it is noted for the second plan.
- **A small mismatch in how the checker sets up the stack for Muse's earlier hand-made translations.** It doesn't affect the translator's results. It could make one of Muse's translations look wrong by four bytes, and it is left as a note.
- **One sentence in the design describes the code-writes check as weaker than it now is.** It is left for you to correct in the design, not a risk to the results.
- **Building the full translated program takes about a minute.** That is fine today, but worth knowing as the second plan translates far more code.

## Branch

Synced with `main` at `24cc49e87a00` on 2026-11-07T17:44:47Z.
The tests were red at the end; a worker tried to fix them and they stayed red.
Tests: red. The branch is not ready to merge.
