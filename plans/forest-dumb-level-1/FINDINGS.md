# Findings log

**What the build taught.** Newest first, forty words a row, counted. Whoever appends, compacts, over 60 rows or 15 KB. Never drop a ✅ row or a term somebody would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-05 | 📌 | T38: the rig's VHPOSR steps every read, so raster-wait `beq`s (`h0+$265e`, `$3aec`) never go taken; BLTBUSY set hangs `h0+$2a1c`. Write-only cells (`h0+$4710`, slot flags in `h0_2e0e`) are not poked, so an omitted clear passes where the program's byte is already 0. |
| 2026-11-05 | 📌 | T36: the rig has no instruction trace, so `gen_camera.rs` shows branch coverage with literal witness models that must match the original on every state. `h0_513c` clamps `camera_y` `$8000..$853e` to `$53f`, not 0 (`cmpi.w` overflow). |
| 2026-11-05 | 📌 | T57 added (owner) by T50 review: `os_layer_scale_and_fullscreen_through_the_menu_bar_and_its_shortcut` red at another wait each run (3×, fullscreen) on 1920×1080, base-identical code; T50 closed on its own menu test. |
| 2026-11-05 | 🐞 | T50: sf-e2e `os_layer_scale_and_fullscreen_through_the_menu_bar_and_its_shortcut` fails three runs of three, each at another wait (3×, fullscreen, ⌃⌘F), on a 1920×1080 display. Timing or screen size, not a stale assertion; left alone. |
| 2026-11-05 | 📌 | T33 live call, worker action: `translate.py 'h0+$5462' --gen char_sprite_position` passed on try 1: 1 629 tokens in, 7 897 out (7 131 reasoning), $0.0017, 67 s, 4 paths. |
| 2026-11-05 | 📌 | T33: `char_hit_check` (`h0+$3b2a`) is 183 instructions with callees, so `translate.py` answers `too-big` though the spike passed it 3 of 3. T34 splits it or the owner moves the 180 line. |
| 2026-11-05 | 📌 | T33: the routine oracle returns from line-mode blits (the shim skips them) and has no fill mode, so "returns on no state" misses them. `RigReport::unmodelled_blits` counts them; `translate.py` answers `no-oracle`. |
| 2026-11-05 | 📌 | T18, worker-driven: old and new route page in headless Chrome gave byte-identical PNGs; with the heat layer on, each page alternates between two images per run. page.html still keys on names `tile coin`, `level exit`, `wN ` prefix. |
| 2026-11-05 | 📌 | T51 to T56 added (owner): T34 counted T08 21 routines, T24 17, T35 20, T38 21, over §4.3's 15; `redraw_edge_a..d` run from `main_loop`, outside `game_frame`, 14 routines. Siblings take them; shares in `games/forest-dumb/docs/11-translation.md`. |
| 2026-11-05 | 🐞 | T34: `re_listing.py`'s linear sweep loses step after data in code; 7 of `game_frame`'s call targets fall inside an instruction. `tools/call_tree.py` decodes from each reached address. `h0+$513c` clamps `camera_y` to `$53f`, not T00's `$52f`. |
| 2026-11-05 | 📌 | T34: `char_probe` (199), `h0_5e44` (200), `h0_62b6` (224), `redraw_edge_c` (181) are over 180 in their own flow, so T33 answers `too-big`; T08, T52, T25 and T55 translate them by hand. |
| 2026-11-05 | 📌 | T31 sync rebuild under oracle hash `c453a4b584dc`: both games' oracle smoke tests green; `level_entry` 523 s cold, 15 s warm. A cold smoke (761 s before) would overrun 900 s; T31 warmed the shared cache. Core build 115 s. |
| 2026-11-05 | 📌 | T31 speed, Kickstart idle screen, CPU time, busy machine (load 6 to 13): 611 frames/s before patch 0006, 606 with no watch list, 562 with 200 addresses. Owner accepted these in place of an idle figure (T31 review). |
| 2026-11-05 | 📌 | T17 review: under load 10–20, wall time of the exit pin swung 20–38 s per run; user CPU time (`/usr/bin/time -p` on the test binary) stayed within 2%, so A/B timing on a busy machine uses CPU time. |
| 2026-11-05 | 📌 | T17: every Superfrog map cache key changed (facts.rs, map.rs, toolkit graph.rs hashed), so maps rebuild once. Pin bench (route_pin with exit pin, release) 38.9 s before, 33.5 and 35.4 s after, all under load 6–10, not idle. |
| 2026-11-05 | 📌 | T05: world 1's load list loads `_WYBIJACZKA` to `h4+$9618` twice, never `$9898`, which keeps program bytes. Play fades its palette in until frame 6331. The view's top-left is level pixel (`camera_x + 17`, `camera_y + 1`). |
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
