# Findings log

What the build taught. Newest first, forty words a row, counted. Flat prose, at most one bold phrase a row.
Whoever appends, compacts first when the file passes 60 rows or 15 KB. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-20 | 📌 | `amiga-browser` has no console capture (T04): a page test catches errors by writing a script into `index.html` before the page's own, collecting `error`, `unhandledrejection` and `console.error` into `window.ERRS` (`page_chrome.rs` `CATCH`). |
| 2026-11-20 | 📌 | **Items needed is computed from the map** (T00): `load_list_world_N` stores BCD(count of `$20`-`$27` over all 38400 object-map bytes - 50). T02 counts the file; the oracle agrees in all six worlds (docs/10). |
| 2026-11-20 | 🐞 | `tools/smoke.sh` in a fresh worktree hit its 1200 s limit at plan review: Forest Dumb done at 820 s, 20 Superfrog entries unrun, another smoke beside it. Four afternoon runs hit it too. |
| 2026-11-20 | 📌 | Headless Chrome 157 answers over `--remote-debugging-pipe` (fds 3 and 4, NUL-separated). Move the pipe ends above fd 4 before `dup2`, else Chrome's end is closed and the first write breaks (plan review probe). |
| 2026-11-20 | 🐞 | DESIGN §2.7 had play as `irq_count != 0`, but it stands still, not at 0, at game over (docs/09) and SCENA (docs/14 T40). Plan review: play is `irq_count` advancing between samples (T06). |
| 2026-11-20 | 🔄 | The brief proposed the live marker and secrets as later work; the owner put both in this plan. |
| 2026-11-20 | 📌 | Headless Chrome 157 re-reads a `file://` script inserted with `?t=` every 300 ms after the file is replaced by rename (planner probe, CDP over a socket). |
| 2026-11-20 | 📌 | Tiles with more than one collision shape: W1-1 44 of 328, W2-1 27, W3-1 80, W4-1 36, W5-1 8, W6-1 13. Candidates for secrets, mostly unexplained. |
| 2026-11-20 | 📌 | Unpacked object maps, used columns: only `$34` is outside W1-1's table (W3-1 31, W4-1 10, W5-1 25, W6-1 17). Every world's exit is a 2×3 block of `$ff`. |
