# Findings log

**What the build taught.** Newest first, forty words a row, counted. Whoever appends, compacts, over 60 rows or 15 KB. Never drop a ✅ row or a term somebody would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-05 | 📌 | T33 live call, worker action: `translate.py 'h0+$5462' --gen char_sprite_position` passed on try 1: 1 629 tokens in, 7 897 out (7 131 reasoning), $0.0017, 67 s, 4 paths. |
| 2026-11-05 | 📌 | T33: `char_hit_check` (`h0+$3b2a`) is 183 instructions with callees, so `translate.py` answers `too-big` though the spike passed it 3 of 3. T34 splits it or the owner moves the 180 line. |
| 2026-11-05 | 📌 | T33: the routine oracle returns from line-mode blits (the shim skips them) and has no fill mode, so "returns on no state" misses them. `RigReport::unmodelled_blits` counts them; `translate.py` answers `no-oracle`. |
| 2026-11-05 | 📌 | T04, worker-driven: `picture_in(0x2F81, 0x29C1)` at frame 6500 opened as PNG: sky, moon, ledge, character from sprites 0-3, black gap lines 226-241, whole status bar. Title window is 320x238; Kickstart's 320x308 from line 5. |
| 2026-11-05 | 📌 | T03 review: a cached reach leaves `Amiga::df0()` `None` where a boot leaves `Some(1)`, since `restore` does not set it. Nothing in fd-conform reads it yet; a task that does must not trust it after `reach_level_1`. |
| 2026-11-05 | 📌 | T03: sync snapshots now end in a SHA-1 and are `.sync2`, so Superfrog sync points rebuild once per machine: `level_entry.rs` (smoke) took 597 s cold. Two boots give equal RAM, not byte-equal vAmiga snapshots. |
| 2026-11-05 | 🐞 | T50 added (owner): sf-e2e `menus.rs` OS test `os_layer_the_menu_bar_holds_every_item…` is red on base; it expects File to hold only "Import Disks…", the app also has Export Level and Play Level File…. |
| 2026-11-05 | 📌 | T12, worker-driven: OS layer of controls, recorder_start_level, recorder_save_window, recorder_bar, persist green on the moved windows. `recorder_bar` OS test failed once at load 15 ("second recording's bar: not within 30 s"), passed on rerun. |
| 2026-11-05 | 🔄 | Owner kept DESIGN §2.7: Esc quits, no pause key, though the original's play acts on P (pause) and Esc (give up, game over). The engine models both for cases (T00). |
| 2026-11-05 | 📌 | T00: `game_frame` ends after the frame boundary in 35 of 3 000 moving frames, so step-end state can be mid-pass. Compare at the pass end, `INTREQ=$4010` at `h0+$1f02` (T06). |
| 2026-11-05 | 📌 | T00: `collision_map` (`h3+$fa50`) is game state built by the foreground edge redraws; the play screen is picture state only; the character is hardware sprites 0-3. `docs/09-play-measured.md`. |
| 2026-11-05 | 📌 | T30 review, read-only: `game_file` treats any `read_file` error as absent, so a damaged file block reads as "no such file" rather than its read error. Left; only a damaged zip reaches it. |
| 2026-11-05 | 📌 | T01 smoke after the move: 761 s ok, cold worktree, load 9 on 10 cores. Before: 887 s warm, 904 s fresh copy. A cold smoke compile took about 8 GB; free disk fell to 14 GB. |
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
