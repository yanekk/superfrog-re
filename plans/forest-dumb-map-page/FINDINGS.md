# Findings log

What the build taught. Newest first, forty words a row, counted. Flat prose, at most one bold phrase a row.
Whoever appends, compacts first when the file passes 60 rows or 15 KB. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-20 | 📌 | T10 review: in `--level 1` after 240 route steps the translated app and a hand `Game` from W1-1's start disagree: one more creature in view, another 8 px apart. Items left and taken cells agree. Not investigated. |
| 2026-11-20 | 📌 | T09, worker-driven at 1280×800: the heading's object count (W1-1 "1156 objects") includes the 133 secrets with the layer off, so it hints at them; `page.html` counts `D.objects`. Left for T12. |
| 2026-11-20 | 📌 | T09: in live play a cell the app changed is drawn by its live value only, so W3-1 41,78, a passage holding an object, loses its secret mark once that object is taken (`page.html`). Left for T12. |
| 2026-11-20 | 📌 | T08 review, worker-driven: the map-only page's heading reads "Forest Dumb Forever route · W3-1" and the summary counts "0 clean 0 with faults"; toolkit `page.html` wording, left for T12. |
| 2026-11-20 | 📌 | T08 drill, worker-driven at three sizes: at the fitted view each of the six `$ff` cells draws its own EXIT label, so they overlap into one smudge. One label per block is `page.html`'s drawing; left for T12. |
| 2026-11-20 | 🐞 | `cargo clippy` on any crate linking fd-conform fails today: `while_immutable_condition` at `fd-conform/src/bug.rs:292`, untouched by T08. |
| 2026-11-20 | 🐞 | T05 drill, worker-driven at three sizes: after the page switched level by itself the last hover's tip still named a cell of the old level; hidden on a live switch, asserted in `live_chrome.rs`. |
| 2026-11-20 | 📌 | T07 review: the character stood in 34 listed passage cells across 9 of 11 groups on the original; W1-1's tower `121,79` and W3-1's `164,53` group were not probed. docs/17's group extents were measured and corrected. |
| 2026-11-20 | 📌 | Collision is `collision_map` from shapes and vanishing blocks; tiles reach only the screen. Hidden passages: W1-1 133, W3-1 292, W4-1 63 cells (docs/17). Per-tile majority misses W1-1's tunnel, built mostly of tiles 301 to 310. |
| 2026-11-20 | 📌 | Judge "drawn as wall" on the raw render, not a collision-tinted overlay: tinting made W1-1's hidden tunnel look like a grey corridor and its solid twin look like a fake one (T07). |
| 2026-11-20 | 📌 | T04 review: a map-only level's header still reads "<title> route · <level>" and the filter "Only routes with faults or failures" (`page.html`). Wording the task left open; left for the owner or T12's drill. |
| 2026-11-20 | 📌 | T04 review: fake unverified against real. `page_chrome.rs` and `made_up_level.rs` write map-only JSON by hand; no game writes one until T02 and T08, which should drive the page on a real dump. |
| 2026-11-20 | 📌 | `amiga-browser` has no console capture (T04): a page test catches errors by writing a script into `index.html` before the page's own, collecting `error`, `unhandledrejection` and `console.error` into `window.ERRS` (`page_chrome.rs` `CATCH`). |
| 2026-11-20 | 📌 | **Items needed is computed from the map** (T00): `load_list_world_N` stores BCD(count of `$20`-`$27` over all 38400 object-map bytes - 50). T02 counts the file; the oracle agrees in all six worlds (docs/10). |
| 2026-11-20 | 📌 | From power-on the translated program reads each route step on its own frame: a `Game` stepped alone on the same steps is a pass off in `irq_count` and a pixel off by step 590 (T06 `live_reader.rs`). |
| 2026-11-20 | 🐞 | `tools/smoke.sh` in a fresh worktree hit its 1200 s limit at plan review: Forest Dumb done at 820 s, 20 Superfrog entries unrun, another smoke beside it. Four afternoon runs hit it too. |
| 2026-11-20 | 📌 | Headless Chrome 157 answers over `--remote-debugging-pipe` (fds 3 and 4, NUL-separated). Move the pipe ends above fd 4 before `dup2`, else Chrome's end is closed and the first write breaks (plan review probe). |
| 2026-11-20 | 🐞 | DESIGN §2.7 had play as `irq_count != 0`, but it stands still, not at 0, at game over (docs/09) and SCENA (docs/14 T40). Plan review: play is `irq_count` advancing between samples (T06). |
| 2026-11-20 | 🔄 | The brief proposed the live marker and secrets as later work; the owner put both in this plan. |
| 2026-11-20 | 📌 | Headless Chrome 157 re-reads a `file://` script inserted with `?t=` every 300 ms after the file is replaced by rename (planner probe, CDP over a socket). |
| 2026-11-20 | 📌 | Tiles with more than one collision shape: W1-1 44 of 328, W2-1 27, W3-1 80, W4-1 36, W5-1 8, W6-1 13. Candidates for secrets, mostly unexplained. |
| 2026-11-20 | 📌 | Unpacked object maps, used columns: only `$34` is outside W1-1's table (W3-1 31, W4-1 10, W5-1 25, W6-1 17). Every world's exit is a 2×3 block of `$ff`. |
