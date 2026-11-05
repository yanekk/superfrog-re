# Findings log

**What the build taught.** Newest first, forty words a row, counted. Whoever appends, compacts, over 60 rows or 15 KB. Never drop a ✅ row or a term somebody would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-05 | 🔄 | Owner kept DESIGN §2.7: Esc quits, no pause key, though the original's play acts on P (pause) and Esc (give up, game over). The engine models both for cases (T00). |
| 2026-11-05 | 📌 | T00: `game_frame` ends after the frame boundary in 35 of 3 000 moving frames, so step-end state can be mid-pass. Compare at the pass end, `INTREQ=$4010` at `h0+$1f02` (T06). |
| 2026-11-05 | 📌 | T00: `collision_map` (`h3+$fa50`) is game state built by the foreground edge redraws; the play screen is picture state only; the character is hardware sprites 0-3. `docs/09-play-measured.md`. |
| 2026-11-05 | 🔄 | Read-back decisions (owner): finisher copies conversation logs (`.pir/rules/on-finish.md` step 1); T49 translates T21's object routines before T22; generator tasks share the porting size limit; T23, T26 keep the split-by-case escape line. |
| 2026-11-05 | 🔄 | Owner set every Muse call to `worker`, the batch included (caps $0.10 a routine, $5 a batch), and asked for a read-back of the amendment. |
| 2026-11-05 | 🔄 | Owner adopted translation by Muse and small sessions (brief): T32 rig split from T06, T33 to T40 translation track, T41 to T48 frame-matching per case. pir deletes `.parallel/control/conversations/` only on Remove; DESIGN §5.3 copies them first. |
| 2026-11-03 | 🔄 | Owner approved the first game's lessons before the build: `toolkit/docs/porting-routines.md`, DESIGN §4.1 and §4.2, two T00 questions, and T31, a sixth vAmiga patch counting executed addresses. Every sync cache rebuilds once after T31. |
| 2026-11-01 | 🐞 | Plan review: no Rust unpacker existed, only `tools/unpack.py`; `fd-tools` only extracts. T30 added before T05 (owner). |
| 2026-11-01 | 📌 | Plan review, measured: Accessibility and Screen Recording granted, screen unlocked, headless Chrome runs a local page. Fresh-copy smoke: 904 s, over its 900 s budget, fails at 1 200 s. |
| 2026-11-01 | 🔄 | Owner replaced the planner's always-on recorder with Superfrog's Start Level modal, one level in its selector, fields added as discovered. |
| 2026-11-01 | 📌 | `sf-e2e/tests/plan_routes.rs` is labelled smoke but its only test is `#[ignore]`d, so smoke only compiles it. Left alone. |
| 2026-11-01 | 📌 | Nothing pins a Superfrog planned route across a refactor: its determinism tests compare runs within one build. T02 adds the pin. |
| 2026-11-01 | 📌 | Smoke took 503 to 887 s of 900 s in recent runs (`~/.cache/superfrog-re/smoke-times.tsv`). T01 makes room. |
