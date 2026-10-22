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
| 2026-10-22 | 📌 | The stored `.pir/settings.json` test line runs only the Python suites (34 tests, ~36 s on a fresh worktree); the Rust suites are not in it. This plan changes no Rust, so the drift check joins the Python suites. |
| 2026-10-22 | 📌 | The Amiga imitation is woven through `sf-core`, not layered: byte-exact `State` at original addresses, `spend` cycle charging, cycle-run copper, and the story's own bus simulator (`front/story/sim.rs`, `cpu.rs`, `mem.rs`). Hence mechanism chapters. |
