# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before anything only a person can verify. Newest first, forty words a row, counted. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a term someone would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-11 | 📌 | **T02 code-seen:** 48 runs, 6 380 instructions, 135 entries (program-entry 1, symbol 134, vector 0, trace 0); every computed target is a named symbol. `world_index` poked before Start is reset to 0, so world starts use `next_world`. |
| 2026-11-11 | 🔄 | Owner first chose retiring Muse and all four model tools, then the same session chose Haiku through the subscription for `generate.py` and `reshape.py`, after a trial (DESIGN §2.13). |
| 2026-11-11 | 📌 | `claude -p --model haiku --max-turns 1 --output-format json` works from a script here: model `claude-haiku-5-5`, 1.7 s, $0.003 list. Logged in through claude.ai. macOS has no `timeout`; bound runs in Python. |
| 2026-11-11 | 🐞 | Smoke in a fresh worktree (cold build) hit `tools/smoke.py`'s 1 200 s limit, no test failed: forest-dumb done at 783 s, Superfrog unrun. Owner chose to start the build anyway (plan review). Translated-program tests go in full. |
| 2026-11-11 | 📌 | `generate.py`, `reshape.py` and `translator_trial.py` import Muse's `translate.py` for the judge lock, `JUDGE_SECONDS`, `flow_text` and the model call; T17 moves these out before T18 replaces `translate.py`. |
| 2026-11-11 | 📌 | The OS is live through the title (`system_on`, the OS keyboard handler, `irq_count` still) and every file loads through `dos.library` (`docs/07-executable-map.md`). The machine answers these; T01 measures exactly which. |
