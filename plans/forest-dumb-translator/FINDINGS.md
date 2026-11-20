# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before anything only a person can verify. Newest first, forty words a row, counted. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a term someone would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-20 | 📌 | **T42 close, promoted:** every reading is in `games/forest-dumb/docs/14-translator.md`; each open 🐞 and 📌 row below T42 is there under "Open after the plan"; Ghidra's sinc defects, `clr`, cycle paths, `cargo fmt` and timing lessons in `toolkit/docs/porting-routines.md` §2, §5, §6. |
| 2026-11-20 | 📌 | **T42 smoke, not run** (pir runs it after the merge): last pass before the plan 786 s (`e320eeb7`); the plan added about 45 s of smoke plus toolkit `contention.rs`'s eighteen boots. Listed in 14-translator.md, "Smoke". |
| 2026-11-20 | 🔄 | **Owner, T41: KONIEC quits the app** as Esc does, no zip: the original returns to AmigaDOS (`quit_to_dos`), the power-on stack holding no return address, so it showed `untranslated: $000000`. `TranslatedGame::exited`; other jumps to 0 still stop. |
| 2026-11-19 | 🔄 | **Owner, T56: exact paths, T58 added** (blocks T40, T42). Exact per-routine totals passed only contact-route and enemy-contact; bus waits hang on each instruction's place. Routines are walked through generated shapes; T58 takes `char_hit_check` and `char_update`. |
| 2026-11-19 | 🔄 | **T54 added** (owner, T50; blocks T42): T50 left 40 of 42 level cases passing translated, the W1-1 routes to pass 4160; instruction-boundary checks cost about 7% (death-spikes 46.2 s against 43.3 s). |
| 2026-11-19 | 🔄 | **Owner, T51:** a load's OS stretch to `system_off` lines up as the load's (`frames::OsTail`); `beam_sample_unread` left out of memory; a frame-end byte passes one frame on. World starts 5, 6 pass; T53 added for 2, 3, 4. |
| 2026-11-18 | 🔄 | **Owner, T33:** `world_index` is set as `load_world` is entered on both sides, not via `next_world`: a `set` row's `when` makes toolkit `Amiga::poke_at` (shim exec hook, BUILD unchanged) and `TranslatedGame::write_at`. |
| 2026-11-12 | 🔄 | **Owner, T47:** a write made by reading a write-only register (the `clr`'s) is compared by register only (`frames::LEFTOVER`). Its word is the bus's last, on the oracle a DMA slot's; matching it needs slot-exact DMA. |
| 2026-11-12 | 🔄 | **Owner exempted `CODE` from the table bar** (T34), a deliberate exception to DESIGN §3.1: only in `pcode_rust.py` output, only `const CODE: &[(u32, u32, u32)]` of (hunk, start, end) triples. It is the translator's index, not game data. |
