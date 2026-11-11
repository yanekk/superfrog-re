# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before anything only a person can verify. Newest first, forty words a row, counted. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a term someone would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-11 | 📌 | `os-surface --worlds 1,3,6` (T01 review) lists `lea $0.l` to `$3.l` at `h0+$691e`..`$6930`, constants in play, as `[[read]]` OS memory with access `address`. Not in the committed worlds 1,2 file; not OS use. |
| 2026-11-11 | 📌 | docs/07 says the file routines open dos.library by `OpenLibrary`; they call `OldOpenLibrary` (-408), as `entry` does for graphics (T01 review, listing). docs/15 is right; docs/07 left alone. |
| 2026-11-11 | 🐞 | docs/07 says the OS's interrupts and keyboard run through the title. Measured (T01): none is taken, autovectors 2 and 3 are the program's (`load_list_title` ends in `system_off`), so the title's Esc route is live; not run. |
| 2026-11-11 | 📌 | Not in docs/07 (T01): `entry` opens graphics.library, calls `OpenFont` topaz (result read, never used) and graphics `FreeSprite` (-414) where `CloseLibrary` was meant; `h0+$51a` writes `ciab_prb` to latch the motor off. T26 answers these. |
| 2026-11-11 | 📌 | `world_index` cannot be poked before `load_world`: `title_code_world` writes it in the same step. T01 sets the immediate at `h0+$778e` (the no-code branch `h0+$778c`) instead; T33's `set` meets the same step. |
| 2026-11-11 | 🔄 | Owner first chose retiring Muse and all four model tools, then the same session chose Haiku through the subscription for `generate.py` and `reshape.py`, after a trial (DESIGN §2.13). |
| 2026-11-11 | 📌 | `claude -p --model haiku --max-turns 1 --output-format json` works from a script here: model `claude-haiku-5-5`, 1.7 s, $0.003 list. Logged in through claude.ai. macOS has no `timeout`; bound runs in Python. |
| 2026-11-11 | 🐞 | Smoke in a fresh worktree (cold build) hit `tools/smoke.py`'s 1 200 s limit, no test failed: forest-dumb done at 783 s, Superfrog unrun. Owner chose to start the build anyway (plan review). Translated-program tests go in full. |
| 2026-11-11 | 📌 | `generate.py`, `reshape.py` and `translator_trial.py` import Muse's `translate.py` for the judge lock, `JUDGE_SECONDS`, `flow_text` and the model call; T17 moves these out before T18 replaces `translate.py`. |
| 2026-11-11 | 📌 | The OS is live through the title (`system_on`, the OS keyboard handler, `irq_count` still) and every file loads through `dos.library` (`docs/07-executable-map.md`). The machine answers these; T01 measures exactly which. |
