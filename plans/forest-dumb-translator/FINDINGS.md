# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before anything only a person can verify. Newest first, forty words a row, counted. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a term someone would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-11 | 🐞 | `translate.py` turns a Muse call error on every try into `fail`, not a missing-model result: the class T17's review fixed in generate and reshape (`no-model`). Left alone; T18 deletes Muse's translate.py. |
| 2026-11-11 | 📌 | `claude -p --tools ""` from a temp folder (claude 2.1.322): no tools, 631 input tokens, no user CLAUDE.md in context. `--max-turns` works though `claude --help` does not list it. |
| 2026-11-11 | 📌 | First real `generate.py` on Haiku: `h0+$513c` covered, 4 of 4 branches, 2 000 returns, 2 tries (the first left its name out of `ALL`), 22 s, $0.011 list. `claude -p` keeps no conversation; retries go as one transcript prompt. |
| 2026-11-11 | 📌 | `docs/11-translation.md` still says generate and reshape call Muse with translate.py's key and spend cap; T17 changed only `tools/README.md`. Left for T20 (docs-and-packs). |
| 2026-11-11 | 🐞 | `Cpu68k::call` with an odd entry runs the instruction at the even address below it (probe: call(0x2001) ran moveq at 0x2000); Moira's reset ignores bit 0. Fixed in `step` (T05 review), left open in `call`. |
| 2026-11-11 | 🔄 | Owner first chose retiring Muse and all four model tools, then the same session chose Haiku through the subscription for `generate.py` and `reshape.py`, after a trial (DESIGN §2.13). |
| 2026-11-11 | 📌 | `claude -p --model haiku --max-turns 1 --output-format json` works from a script here: model `claude-haiku-5-5`, 1.7 s, $0.003 list. Logged in through claude.ai. macOS has no `timeout`; bound runs in Python. |
| 2026-11-11 | 🐞 | Smoke in a fresh worktree (cold build) hit `tools/smoke.py`'s 1 200 s limit, no test failed: forest-dumb done at 783 s, Superfrog unrun. Owner chose to start the build anyway (plan review). Translated-program tests go in full. |
| 2026-11-11 | 📌 | `generate.py`, `reshape.py` and `translator_trial.py` import Muse's `translate.py` for the judge lock, `JUDGE_SECONDS`, `flow_text` and the model call; T17 moves these out before T18 replaces `translate.py`. |
| 2026-11-11 | 📌 | The OS is live through the title (`system_on`, the OS keyboard handler, `irq_count` still) and every file loads through `dos.library` (`docs/07-executable-map.md`). The machine answers these; T01 measures exactly which. |
