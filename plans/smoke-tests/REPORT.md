# smoke-tests — delivery report

## What was delivered

**The tests are now split into a quick smoke set and a full set, and pir uses the quick one.** All 8 tasks are done and reviewed, and the end-of-build tests passed using the smoke set.

- **Smoke set:** it covers the root checks, the toolkit and both games, whatever a branch changed. It took about 8.5 minutes with the code already compiled, and about 14.8 minutes from a completely empty start (887 seconds). Both runs had other work going on on the machine. That compares with about 70 minutes for everything before.
- **Time limit:** past 15 minutes smoke prints a warning and still passes. At 20 minutes it stops, lists what did not run, and fails. Every run adds a line to a local history of times, which agents are told to read before adding a slow test.
- **Forgotten tests:** a check runs first and takes seconds. It fails the run if a test file has no label, or if a crucial area on your list has no test that actually runs.
- **Full set:** a new command, `tools/full.sh`, runs every test, including the slow ones normally skipped. It leaves out the 32 screen tests and one helper tool (see the decisions below). pir never runs it.
- **New plans:** new plans and small single changes now take smoke as their end check. Plans already written keep their own line.
- **Instructions to agents:** the project instructions now tell agents to label every new test, add a test for anything crucial, and ask you if the crucial tests alone would not fit in 15 minutes.

**Not delivered:**

- **Nightly schedule:** this was left out of the plan on purpose, so nothing runs the full set yet.
- **Full set never run for real:** as you ordered, no session ran it. It was only tried with a stand-in for the build tool, so nobody knows yet whether it passes or how long it takes.
- **Screen tests:** running them in a virtual machine is left to a later plan.

## Decisions made for you

- **T01: Design §2.2 contradicts itself about how a game's smoke.toml writes a Rust test's path. Its sentence says paths start from the folder holding smoke.toml, which gives "engine/crates/x/tests/quick.rs". Its example, with workspace = "engine", writes "crates/x/tests/quick.rs", which starts from the engine folder. The toolkit is the same either way. Which form should the check accept? Python paths such as "tools/test_x.py" start from the smoke.toml folder under both options.** (night)
  Answer: Design §2.2 contradicts itself about how a game's smoke.toml writes a Rust test's path. Its sentence says paths start from the folder holding smoke.toml, which gives "engine/crates/x/tests/quick.rs". Its example, with workspace = "engine", writes "crates/x/tests/quick.rs", which starts from the engine folder. The toolkit is the same either way. Which form should the check accept? Python paths such as "tools/test_x.py" start from the smoke.toml folder under both options. → Follow the example (Recommended)
  Why: The worked example and T01's wording agree, and no labels exist yet, so this is cheap to change later. Recorded as a deviation from §2.2's sentence.
- **T04: The nightly full test command (tools/full.sh) would fail every night whatever the code does. One ignored Superfrog test, `zips`, is really a helper tool: it writes bug-report zip files into the folder named by the setting SF_T14_ZIPS, and it stops with an error when that setting is missing. I confirmed this by running that one test the way the full command would. The design leaves only the 32 screen tests out of the full run, so this needs your call. What should the full command do with `zips`? If you say nothing, I keep waiting.** (night)
  Answer: The nightly full test command (tools/full.sh) would fail every night whatever the code does. One ignored Superfrog test, `zips`, is really a helper tool: it writes bug-report zip files into the folder named by the setting SF_T14_ZIPS, and it stops with an error when that setting is missing. I confirmed this by running that one test the way the full command would. The design leaves only the 32 screen tests out of the full run, so this needs your call. What should the full command do with `zips`? If you say nothing, I keep waiting. → Leave it out (Recommended)
  Why: Its own ignore reason calls it a tool, and it asserts nothing about the code, so no coverage is lost. A permanently red nightly would hide real failures. One line to undo. Goes against §1 and §2.1 (only the screen tests are left out), so record it as a deviation.

## What to check by hand

1. **Set up the nightly run of `tools/full.sh`, and read its first result.** This is the only real test of the full set. Expect it may be long or red: it includes slow tests that normally never run, and one alone can take 40 minutes. You chose that knowingly.

2. **Clear a leftover game process.** A copy of the Superfrog program started by the timing task was still running 4 hours later, and nobody stopped it. If it is still there, stop it. To check first, run `ps -p 40416`; if the line it shows names `superfrog`, run `kill 40416`. Nothing depends on it, and stopping it loses nothing.

## Risks and follow-ups

- **A break only the full set can see now shows up later.** It passes a build's end check and appears in the next nightly run, possibly after the merge. You accepted this, but until the nightly is set up nothing will catch such a break at all.
- **Little room from an empty start.** The cold run finished only 13 seconds under the 15-minute warning, with other work on the machine. An occasional warning is likely, and that alone does not fail a build. As tests are added, some fast ones will need moving to the full set.
- **The warning is easy to miss.** It does not reach the build report, only the end check's output and the history of times.
- **Thin spots in the crucial areas.** The label check confirms a test exists for each area, not how much of the area it exercises.
  - No comparison with the original plays a valid level code into a level. "Entering a level by its code" rests on a code-reading test and a level-entry test.
  - Two files labelled smoke, the recorder drill and the route plans, contain only skipped tests, so they run nothing in smoke.
  - Worth a follow-up task if you want those areas stronger.
- **Programs can outlive a stopped run.** The plan says nothing survives when smoke is stopped, but this does not fully hold. An app started by the end-to-end tests can keep running for up to about 60 minutes' worth of nothing only in the sense of its own limit, by default 60 seconds. The leftover game process above went further and outlived its limit, and its cause is still open. Neither affects whether a run passes, but stray copies can slow later runs on this machine.
- **Skipping tests by name.** The full set leaves tests out by their name, so a future ordinary test with the same name as a left-out one would silently drop out of it too. No such clash exists today.
- **Borderline labels.** Times were measured on a busy machine, so some files near the 10-second line for "fast" could be labelled either way. This changes how long smoke takes, not whether anything is tested eventually, since the full set keeps everything except the left-outs above.

## Branch

Synced with `main` at `63726e85d9d0` on 2026-10-31T22:33:30Z.
Tests: green.
