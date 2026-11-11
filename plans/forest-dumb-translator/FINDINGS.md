# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before anything only a person can verify. Newest first, forty words a row, counted. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a term someone would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-11 | 🔄 | Owner first chose retiring Muse and all four model tools, then the same session chose Haiku through the subscription for `generate.py` and `reshape.py`, after a trial (DESIGN §2.13). |
| 2026-11-11 | 📌 | `claude -p --model haiku --max-turns 1 --output-format json` works from a script here: model `claude-haiku-5-5`, 1.7 s, $0.003 list. Logged in through claude.ai. macOS has no `timeout`; bound runs in Python. |
| 2026-11-11 | 📌 | Smoke is over budget before this plan: 786 to 1 122 s on 2026-11-11 (`~/.cache/superfrog-re/smoke-times.tsv`), two of five failed. Translated-program tests go in full. |
| 2026-11-11 | 📌 | `generate.py`, `reshape.py` and `translator_trial.py` import Muse's `translate.py` for the judge lock, `JUDGE_SECONDS`, `flow_text` and the model call; T17 moves these out before T18 replaces `translate.py`. |
| 2026-11-11 | 📌 | The OS is live through the title (`system_on`, the OS keyboard handler, `irq_count` still) and every file loads through `dos.library` (`docs/07-executable-map.md`). The machine answers these; T01 measures exactly which. |
