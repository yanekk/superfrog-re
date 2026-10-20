# Findings log

**What the build taught.** Read the rows touching the task you pick up. Newest first, forty words a
row, counted. Whoever appends, compacts (over 60 rows or 15 KB). Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-20 | 📌 | `Start::Level` has no seed: `rng_state` comes from the boot image, and `level_entry.rs` `NOT_MODELLED` lists it with the front end's fades and music place. Why D2 is measured by T00. |
| 2026-10-20 | 📌 | `opt_game_mode` and `opt_lives` are read by `frog_new_game` before the level loads; poking them after `Start::Level` is too late (`frog/mod.rs`). |
| 2026-10-20 | 📌 | Full workspace suite: 6 247 s cold in a fresh worktree; `whole_game.rs` "the window fell behind" failed on a loaded machine. Block scoped to this plan's parts (D11). `.pir/settings.json` runs Python only. |
| 2026-10-20 | 🔄 | Owner: Lives is a 3/5/7 dropdown like the options screen, replacing 0–99; Energy a 1–4 dropdown. |
