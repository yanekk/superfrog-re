# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts**: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a
term somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-31 | 📌 | T03 review: `amiga-clock/src` re-measured at 8.6 s, not the 10.5 s in `timings.tsv`, on a less busy machine, so load can flip a borderline row. T05, T06 re-measure rows near 10 s before labelling. |
| 2026-10-31 | 📌 | T03: pass had another session's `cargo test --workspace` running, so times read high. Finished over 10 s: `amiga-clock/src`, `bosses_routines`, `widescreen_objects`, `sf-plan/tests/map`. No file failed; `over` hides any red. Re-measure before trusting borderline rows. |
| 2026-10-31 | 📌 | T01: parser counts 114 `#[ignore]`d tests, not the 104 in DESIGN §7: `code_cases!` makes 23 in `chain.rs`. Reconciled against grep: 1399 written plus 93 macro-made is 1492 tests. |
| 2026-10-31 | 🔄 | Owner: DESIGN §2.2 contradicts itself. `[tests]` keys are relative to `workspace` (`crates/x/...`), as its example shows; `[python]` keys relative to the project folder. T05, T06 write game labels this way. |
| 2026-10-31 | 🔄 | Plan review: 32 of 104 `#[ignore]` tests panic without `SF_E2E_OS=1` and post real screen input with it. Owner excluded them from `tools/full.sh`; a later plan runs them in `tart`. |
| 2026-10-31 | 📌 | Plan review: `cargo tree` shows `-p` picks different features of shared crates (`bitflags`, `syn`, `thiserror`, `objc2-*`) than `--workspace`, so a workspace `--no-run` does not cover per-crate test calls. |
| 2026-10-31 | 📌 | Plan review: pir spawns each test line detached and kills with `SIGKILL` to that group (`commands.mjs`). An entry in its own group would survive unless something outlives the runner. |
| 2026-10-31 | 📌 | Plan review: all Python test files together took 33 s in a fresh copy, not about 23 s. |
| 2026-10-31 | 📌 | pir's report records only pass or fail, written by the coordinator without the gate log. A passing run's `smoke: WARNING` is seen only in the end gate's output; hence `smoke-times.tsv`. |
| 2026-10-31 | 📌 | pir's end gate parses the DESIGN block and sets `PIR_TEST_SCOPE=all`. It reads the block on the merged branch at the end, so a block naming a script a task creates works. |
| 2026-10-31 | 📌 | 104 `#[ignore]` tests run in no normal run. Some docs ask for `--release`; `tools/full.sh` runs them in debug, so times may exceed their comments. Total unknown. |
| 2026-10-31 | 📌 | W1-1's route in the real app is `an_app_route_reaches_w1_1s_exit_in_the_real_app_with_no_life_lost`, `#[ignore]`, about 3 min. Smoke uses `complete_w1_1_to_exit_fade` instead. |
| 2026-10-31 | 🔄 | Owner dropped measured code coverage from the requirements after first choosing it as a separate nightly command. |
