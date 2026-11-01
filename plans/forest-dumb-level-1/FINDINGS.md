# Findings log

**What the build taught.** Newest first, forty words a row, counted. Whoever appends, compacts, over 60 rows or 15 KB. Never drop a ✅ row or a term somebody would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-01 | 🔄 | Owner replaced the planner's always-on recorder with Superfrog's Start Level modal, one level in its selector, fields added as discovered. |
| 2026-11-01 | 📌 | `sf-e2e/tests/plan_routes.rs` is labelled smoke but its only test is `#[ignore]`d, so smoke only compiles it. Left alone. |
| 2026-11-01 | 📌 | Nothing pins a Superfrog planned route across a refactor: its determinism tests compare runs within one build. T02 adds the pin. |
| 2026-11-01 | 📌 | Smoke took 503 to 887 s of 900 s in recent runs (`~/.cache/superfrog-re/smoke-times.tsv`). T01 makes room. |
