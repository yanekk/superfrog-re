# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before anything only a person can verify. Newest first, forty words a row, counted. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a term someone would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-11 | 📌 | **T00 seam: mount `game_frame` whole.** Frame mount (transplant in, `game_frame_on`, write back what differs from memory) passes 2 000/2 000 `game_frame` states, 47 bytes changed a pass (max 71). T35 mounts by address; T36 mounts only `h0+$1fec`. |
| 2026-11-11 | 📌 | Per routine also passes: `char_sprite_position` and `camera_update` mounted alone, and both inside translated `game_frame`, 2 000/2 000 each with no registers compared. All registers fail: ports leave scratch d0-d3, d7 (487/2 000 at `game_frame`'s exit). |
| 2026-11-11 | 🐞 | A write-back compares with memory's bytes, not the typed state before: `camera_update`'s `crossed` flags are bools, so a poked `$6c` stayed and 288/2 000 failed. Camera needed four values outside `LAYOUT` (`h0+$4d76`, `$4d7a`, `$51f8`, `frame_flag`). |
| 2026-11-11 | 📌 | Round trip (write back whole, transplant again) over `camera-right-left`'s first 100 steps: 0 equal. Not transplanted: `edges.pos` (`h0+$1e66`), pointers `p840`, `p83c`, the `input_log` bytes. Screen and plane share `h3+$fa10..$fa4f`; 4 bytes disagree. `game_frame` reads none. |
| 2026-11-11 | 📌 | Whole write-back is 129 500 bytes (display 75 101, object map 38 400, plane 15 152, outside-`LAYOUT` extras 539, `LAYOUT` 207, input log 101); the frame mount stores only what changed. |
| 2026-11-11 | 🔄 | Owner first chose retiring Muse and all four model tools, then the same session chose Haiku through the subscription for `generate.py` and `reshape.py`, after a trial (DESIGN §2.13). |
| 2026-11-11 | 📌 | `claude -p --model haiku --max-turns 1 --output-format json` works from a script here: model `claude-haiku-5-5`, 1.7 s, $0.003 list. Logged in through claude.ai. macOS has no `timeout`; bound runs in Python. |
| 2026-11-11 | 🐞 | Smoke in a fresh worktree (cold build) hit `tools/smoke.py`'s 1 200 s limit, no test failed: forest-dumb done at 783 s, Superfrog unrun. Owner chose to start the build anyway (plan review). Translated-program tests go in full. |
| 2026-11-11 | 📌 | `generate.py`, `reshape.py` and `translator_trial.py` import Muse's `translate.py` for the judge lock, `JUDGE_SECONDS`, `flow_text` and the model call; T17 moves these out before T18 replaces `translate.py`. |
| 2026-11-11 | 📌 | The OS is live through the title (`system_on`, the OS keyboard handler, `irq_count` still) and every file loads through `dos.library` (`docs/07-executable-map.md`). The machine answers these; T01 measures exactly which. |
