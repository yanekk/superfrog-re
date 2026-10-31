# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts**: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a
term somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-31 | 📌 | pir's report records only pass or fail, written by the coordinator without the gate log. A passing run's `smoke: WARNING` is seen only in the end gate's output; hence `smoke-times.tsv`. |
| 2026-10-31 | 📌 | pir's end gate parses the DESIGN block and sets `PIR_TEST_SCOPE=all`. It reads the block on the merged branch at the end, so a block naming a script a task creates works. |
| 2026-10-31 | 📌 | 104 `#[ignore]` tests run in no normal run. Some docs ask for `--release`; `tools/full.sh` runs them in debug, so times may exceed their comments. Total unknown. |
| 2026-10-31 | 📌 | W1-1's route in the real app is `an_app_route_reaches_w1_1s_exit_in_the_real_app_with_no_life_lost`, `#[ignore]`, about 3 min. Smoke uses `complete_w1_1_to_exit_fade` instead. |
| 2026-10-31 | 🔄 | Owner dropped measured code coverage from the requirements after first choosing it as a separate nightly command. |
