# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**What goes here:** what the next session would otherwise rediscover · what you noticed and left
alone under the scope rule (an `owns` path that fits another chapter better, DESIGN §2.7) ·
answers from the user, dated.

**Whoever appends, compacts** (over 60 rows or 15 KB). Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-23 | 📌 | T03: `sf-core` never reads `sf_assets::Tables`. Templates, the marker jump table and the slot tables are decoded there, but the engine reads them by address from relocated hunk 0; only `catalogue` and tests use `Tables`. |
| 2026-10-23 | 📌 | T03: hunk 0 is held four times (`ExeImage`, `Rom::h0`, `FrontData::h0`, `SoundData`) by three relocators (`mem.rs` `loaded`, `front/data.rs` `relocate`, `story/data.rs` `data_hunk`); `sound.rs` repeats `H0`, `H3` and its own `file_table` reader. |
| 2026-10-23 | 📌 | T03: the engine reads original code as data: `Chain::run` steps through `top_loop`, `password::table` reads handlers' `move.l` immediates, `table_entries` scans for `lea`. A native rewrite replaces these before dropping hunk 0. |
| 2026-10-23 | 📌 | `tools/architecture.py totals` counts 111k code lines today (verification 47.8k, app 10.4k), against DESIGN §1's ~75k on `pir/rust-engine`. The T01 ownership table covered every tracked file; nothing was reassigned. |
| 2026-10-22 | 📌 | The stored `.pir/settings.json` test line runs only the Python suites (34 tests, ~36 s on a fresh worktree); the Rust suites are not in it. This plan changes no Rust, so the drift check joins the Python suites. |
| 2026-10-22 | 📌 | The Amiga imitation is woven through `sf-core`, not layered: byte-exact `State` at original addresses, `spend` cycle charging, cycle-run copper, and the story's own bus simulator (`front/story/sim.rs`, `cpu.rs`, `mem.rs`). Hence mechanism chapters. |
