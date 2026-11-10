# T85 — observes-only-flake

**Phase:** 6 · **Depends on:** —; blocks T29 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T85` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T82 with the coordinator agent's yes (2026-11-10), under DESIGN §4.2. `fd-conform/tests/edge_latch.rs`, `sampling_for_the_edge_cut_leaves_the_original_unchanged_every_frame`, is the test that holds every measurement on the original to observe only (owner, 2026-11-09; T78 made it pass on every level case). It fails now and then.

## What T82 measured (2026-11-10, debug profile, load 4 to 7)
- The whole file (`cargo test -p fd-conform --test edge_latch`), on T82's branch: the test failed in 2 of 4 runs, `death-spikes: sampling changed the original` and `stretch-pit: sampling changed the original`, a different short case each time. Both failing runs came straight after other test files of the same `cargo test` command; both passing runs were the file alone.
- The test alone (`... sampling_for_the_edge_cut`): passed, 1 of 1.
- The whole file at `16178f85`, before T82: the test passed, 1 of 1.
- Not measured: the step and what differed (chip or slow RAM, the picture, the CPU registers); whether it ever fails at `16178f85`; whether the other tests of the file must be running for it to fail.
- The test runs every level case at once, each on two copies of the original in one process, `route-W1-1`'s 22 536 steps among them (about 340 s).

## Goal
Say which it is, measured, and fix it:
1. **The test is unreliable**: something outside the two runs compared reaches one of them (state shared between oracle instances or between the file's tests in one process, a cache written while read, a time limit). Then the test is made reliable and still fails when sampling is made to change a run.
2. **The measurement disturbs the original sometimes**: a run sampled as the runner samples it differs from the run alone. Then every acceptance rule that relies on the measurement is named in `FINDINGS.md` with what it read: T71 (the status redraw cut, `interrupted_pc`), T74 and T75 (`edges_cut`), T80 (`creature_cuts`, which samples nothing), T82 (the status redraw's call count from T31's counter, and `AddCut` and the pending bob from `edges_cut`), and T48's `shape_words`. **The owner is asked before the run finishes**: what each rule accepted may not have been what the original showed.

T74's review found sampling shifting the run with creatures live; T78 (✅) fixed a step ending early where a blit held the CPU across the frame's end. Whether this is a rest of that is part of the question.

## Design sections this implements
§4.1, §4.2.

## Reuses
`edge_latch.rs`'s `sampled_against_alone` (it returns the first differing step), `Sampler`, `amiga-oracle`'s `tests/line_samples.rs`, T78's findings and its task doc, `toolkit/crates/amiga-oracle/README.md`.

## Files
`fd-conform/tests/edge_latch.rs`; `toolkit/crates/amiga-oracle/**` if the oracle is at fault; `plans/forest-dumb-level-1/FINDINGS.md`.

## Interface
None.

## Tests
- [ ] The failure is reproduced on purpose: the command, how often, and the first differing step with what differed.
- [ ] The cause is shown by a run that fails with it and passes without it.
- [ ] The test passes ten whole-file runs in a row under the load that failed it, and still fails when a sampled step is made to differ.
- [ ] If the oracle changed: `amiga-oracle`'s own tests and both games' oracle smoke pass.

## Done when
- [ ] The tests above pass and `FINDINGS.md` says which of the two it was, with the measurement.
- [ ] If it was the second: the rules are named there and the owner has answered.
- [ ] `tools/smoke.sh --check-only` is clean.
