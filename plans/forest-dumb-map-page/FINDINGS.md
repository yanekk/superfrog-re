# Findings log

What the build taught. Newest first, forty words a row, counted. Flat prose, at most one bold phrase a row.
Whoever appends, compacts first when the file passes 60 rows or 15 KB. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-20 | 📌 | Collision is `collision_map` from shapes and vanishing blocks; tiles reach only the screen. Hidden passages: W1-1 133, W3-1 292, W4-1 63 cells (docs/17). Per-tile majority misses W1-1's tunnel, built mostly of tiles 301 to 310. |
| 2026-11-20 | 📌 | Judge "drawn as wall" on the raw render, not a collision-tinted overlay: tinting made W1-1's hidden tunnel look like a grey corridor and its solid twin look like a fake one (T07). |
| 2026-11-20 | 🐞 | `tools/smoke.sh` in a fresh worktree hit its 1200 s limit at plan review: Forest Dumb done at 820 s, 20 Superfrog entries unrun, another smoke beside it. Four afternoon runs hit it too. |
| 2026-11-20 | 📌 | Headless Chrome 157 answers over `--remote-debugging-pipe` (fds 3 and 4, NUL-separated). Move the pipe ends above fd 4 before `dup2`, else Chrome's end is closed and the first write breaks (plan review probe). |
| 2026-11-20 | 🐞 | DESIGN §2.7 had play as `irq_count != 0`, but it stands still, not at 0, at game over (docs/09) and SCENA (docs/14 T40). Plan review: play is `irq_count` advancing between samples (T06). |
| 2026-11-20 | 🔄 | The brief proposed the live marker and secrets as later work; the owner put both in this plan. |
| 2026-11-20 | 📌 | Headless Chrome 157 re-reads a `file://` script inserted with `?t=` every 300 ms after the file is replaced by rename (planner probe, CDP over a socket). |
| 2026-11-20 | 📌 | No `33fc 0282` (move.w #282) in the main program: W1-1's items-left start value is not a plain immediate word store. T00 finds its source. |
| 2026-11-20 | 📌 | Tiles with more than one collision shape: W1-1 44 of 328, W2-1 27, W3-1 80, W4-1 36, W5-1 8, W6-1 13. Candidates for secrets, mostly unexplained. |
| 2026-11-20 | 📌 | Unpacked object maps, used columns: only `$34` is outside W1-1's table (W3-1 31, W4-1 10, W5-1 25, W6-1 17). Every world's exit is a 2×3 block of `$ff`. |
