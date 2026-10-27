# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** once this file is over 60 rows or 15 KB. Never drop a ✅ row or its
date, or a term somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-27 | 📌 | Planning: the stored setup and test lines, run in a fresh worktree on `874c78f`, are green: setup clean, Python ok, `cargo test -q --workspace` 145 suites ok in 1 h 45 min, near the 120-minute `testTimeoutMinutes`. |
| 2026-10-27 | 📌 | Planning: `story/sim.rs`'s HRM word table differs from `level::beam::slots_per_word` for channels 0x6 and 0xE (B and C without D: 3 against 4). Not one table; both stay (DESIGN §2.4). |
| 2026-10-27 | 📌 | Planning: `slot/draw.rs` `spend` and its line sum wrap lines at the front end's 313 (`crate::front::FRAME_LINES`), while the slot's `CUT_LINE` uses its own 312. Each keeps its length (DESIGN §2.2). |
| 2026-10-27 | 📌 | Planning: frame/line lengths `measure.sh` does not count: sf-paula `CLOCKS_PER_LINE`, `LINES_PER_FRAME`; story `LONG_FRAME`, intro `LINES`; `blit.rs` `l % 313`. T01 derives them from `amiga-clock` (DESIGN §2.2). |
| 2026-10-27 | 📌 | Planning: `level/timeline.rs` reads `front::Cx`, `Yield` and `game::Job`, and `word_times` takes `&copper::Fetch`; neither moves as it is. T02 splits them (DESIGN §2.4, §2.6). |
