# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before
anything only a person can verify. A ✅ row is the entire record that something was seen working for real.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**What goes here:** what the next session would otherwise rediscover · what you noticed and
left alone under the scope rule · every answer that came back from the user's own hands, dated.
**What does not:** a decision and its reasoning (commit message) · anything code or a test now states.

**Whoever appends, compacts.** Over 60 rows or 15 KB: merge repeats, drop rows now enforced by
code or a test (naming where), never drop a ✅ row or its date, never drop what someone would grep for.
A finding that binds every session moves to `CLAUDE.md` as one line, and the row here becomes a pointer.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-13 | 🐞 | T32 review, open for T43: in normal play the engine's intro hands over to the game untouched (probe: frame 9 282, no input); `END` has no fire wait, though `Start::Disk3`'s doc says fire is pressed. |
| 2026-10-13 | 📌 | T32 added T42 story-intro-scenes, T43 story-intro-handover, T44 story-intro-skips (first numbered T36–T38, which T15 took; owner renumbered 2026-10-13) (owner: rewind to the last working milestone, split the rest). All block T21. T32 stops at frame 539; T42's doc lists what its scan left. |
| 2026-10-13 | 📌 | T32 for T42: the runner's load-line search fits on the picture, so a fade-timing change in a load frame moves the line found and the next frames. A draft cycle-exact fade-step model did that; reverted. |
| 2026-10-13 | 📌 | T32 deviation from DESIGN §2.1: `Game::step` uses a new `Pacing::Unpaced` (the engine's own timing) instead of `OnTime`; only the story intro reads it. DESIGN §2.1 still says `OnTime`. |
| 2026-10-12 | 🔄 | Owner, T32: conformance feeds the intro whether the original is still in a long job at each blank (`Pacing::Late`/`OnTime` from its call chain, as level lag); normal play uses the engine's own timing model. |
| 2026-10-12 | 📌 | T32: the intro's sound is the game's T11 driver at shifted addresses (`AudioDriver::intro_from`); music start/stop differ slightly. d3_c07/c08 are four raw sector spans (`intro_file_table` h0+$cf26), kept as `INTRO_SPAN_INDEX` chunks. |
| 2026-10-13 | 🔄 | Owner, T15: stop and split; T36–T38 added (picture/lag/entry, restart, tally). Draft on local branch `t15-flow-wip`. |
| 2026-10-13 | 📌 | T15: an undrawn bob's step pointer is 0, so despawn reads width/height from ExecBase ($C00276): `low_vectors`. Items lag W1-1's pass 86/700 frames. |
| 2026-10-12 | 📌 | T14 review: the lag re-sync loads the original's whole memory, so a divergence landing on a lag frame is erased: `frog-run-w1-1` stayed green with fake walls not digging. A case crossing lag needs a re-sync-free check (level_entry's fake-wall test). |
| 2026-10-12 | 📌 | T14 review sweep, not fixed: `any_frog` (frog_routines.rs) keeps the frog at x, y ≥ 48: the frog routines' edge handling near column or row 0 is never generated. |
| 2026-10-12 | 📌 | T14: the final level is world 6 with `final_level_flag`; its LW tiles and heights replace W6's. `Bus::b` picks Rom slot 8 by the flag; any reader of world files must too. |
| 2026-10-12 | 📌 | T14: W5-1's shuttle drifted the frog into a shaft where it sinks (`frog_sprite_dy` counts) and dies at 832, the screen closing; that death path is T15's. `frog-shuttle-w5` keeps it clear. |
| 2026-10-12 | 📌 | T35: CPU work spanning frames (`ending::CpuWork`): main program from line 22, 3.6 cycles a word per fetched plane past four; fits all 129 ANIM decodes within 2 lines of line 255. Same model untested elsewhere (T20's `busy` is flat). |
| 2026-10-12 | 📌 | T35: `music_stop` from line 299 ends on line 1 next frame (masked); a load frame now drops writes carried from the last frame. MusicFadeWait models it; TitleExit's game-start fade (T21) goes through it untested. |
| 2026-10-12 | 📌 | T35, for T14: `load_hero` (hero frames h3+$4c0b0, table h0+$19ddc) is ported in `front::hero` and runs at boot; it clears the level map. The original reseeds the RNG after the end sequence from CIA-B TOD. |
| 2026-10-12 | 🐞 | T20 review, fixed: fire skipping the title intro diverged a frame later; the engine's restore blit took 102 lines, the original's 161–308. `Cx::blit_wait` charges bitplane fetch slots; `front-skip-*` cases, one in the test block. |
| 2026-10-12 | 📌 | T20 review sweep, not fixed: CPU jobs (`Cx::busy`: text drawing, unpacking, masked jobs) take no display DMA contention either; no case diverges on it. |
| 2026-10-12 | 📌 | T20 review: after a palette fade the original's main program resumes about line 42, the engine's at `VBL_END_LINE` (20). Blit timing absorbs it; a slot-exact blit model (`word_times`) exposes it. |
| 2026-10-12 | 📌 | T20 for T21: the progress map (`progress_map_show` h0+$3de8, `progress_map_vbl`) is shown during level loads, so T21 ports it; the VBL records it unported (`h0+$4112`). The options' hi-score save writes the disk: unported. |
| 2026-10-12 | 📌 | T20: the original's frame ends when the CPU takes the interrupt; a priority blit or masked job past line 312 moves it (copper's line-0 writes and window go with the earlier frame). `clr.w` to a custom register writes the prefetch word, then 0. |
| 2026-10-12 | 🐞 | T20, left open (owner): at the original's frame 6416 the credits' scroll blit and the beam cross inside one row, 4 pixels differ; `front-attract` stops at 2 270 frames. T33 runs this stretch again. |
| 2026-10-12 | 🔄 | Owner, T20: the engine starts at the game program's start (h0 `init_and_script`), not power-on; Kickstart and the crack intro are not reproduced. Disk loads are not modelled frame by frame: the runner skips the original's load frames and re-syncs. |
| 2026-10-12 | 🐞 | T31: docs/10 and the T08 row had W6's map sets backwards. One drive plays d4_c14–c17 (`alt`); any connected df1–df3 gives c02–c05 (`probe_drives`: 0 = connected). T13's render-w6-2..4 rendered alt; now reached with df1. |
| 2026-10-12 | 🔄 | Owner, T31: the final level is reached by writing `level_state` = 1 on W6-4's first frame (no code, cheat or trainer leads there). Its score and lives are not a playthrough's. `Amiga::write_memory` exists for this alone. |
| 2026-10-12 | 📌 | T31: with an empty df1 connected the boot reaches the title 219 frames sooner. With disks 2–4 in df1–df3 the crack stalls on ACCESSING DISK past frame 20 000; not investigated (one empty df1 suffices). |
| 2026-10-12 | 📌 | T22 review: sf-e2e frame-0/400 and audio equality pass vacuously today: PowerOn is the black Placeholder scene and the app mixes zero chip, so a gutted presenter or mixer stays green. They gain teeth once T14/T20 draw and sound. |
| 2026-10-12 | 📌 | T22 and its review, worker-driven: OS layer green from WezTerm under SF_E2E_OS=1 (posted Left reached steps 45–47, 320×288 capture, AX "Quit superfrog" closed it). Early captures came back fully transparent, later ones real. |
| 2026-10-12 | 📌 | T22, macOS 26: `screencapture` is refused from the terminal and System Events times out on an ungranted Automation prompt, so sf-e2e captures via CGWindowListCreateImage (JXA) and presses menus via AX. Window-server bounds sit 3 pt inside winit's outer size. |
| 2026-10-12 | 📌 | T22, left alone: `Game` exposes no chip memory or frame length, so the app mixes against zero chip and paces 313-line frames (T14 must add both). Fire is Space, arrows the stick (T23 remaps). No lock file yet (T23). |
| 2026-10-12 | 📌 | T13 review: `late_redraw` excuses a differing pixel wherever an alternative differs from end-of-frame, not where it matches the original (strict matching fails 10 of 25 cases on genuine mixed frames). A renderer defect confined to changed regions is skipped, not caught. |
| 2026-10-11 | 🔄 | Owner, T13: the two display buffers (h3+$10600, $22f30) are engine state; the play field renders from them, the map only for widescreen. T14–T18 must port the original's tile, animation and bob blits into them. Save states grow ~150 KB. |
| 2026-10-11 | 📌 | T13: no colour cycling in levels. Level palettes change only in the start fade (h0+$153a4), exit fade ($155a0) and boss flash ($151b2). T08's "COLOR06 cycling, inferred" was not seen in 8 500 frames of W1-1. |
| 2026-10-11 | 📌 | T13, for the bob port: blitter ignores bit 0 of a recorded destination; buffer 0's draw x survives only in its seam sub-record (+$8c), the upper one is already the next pass's. Sprites only show on lines with a bitplane fetch. |
| 2026-10-11 | 📌 | T11 review: fake unverified against real: Moira hardware mode records a byte write to a custom register as that byte on both halves. The driver never makes one; nothing checks it against vAmiga. |
| 2026-10-11 | 📌 | T11: the title's main loop writes DMACON $8020/$8400 and INTENA $4000/$c000 pairs around loading. The audio comparison filters them; whole-frame Paula comparison (T12 on) must expect them from non-sound code. |
| 2026-10-11 | 📌 | T08: level palette = 32 colours after `LnLP` (+$11300), proven on W1-1's copper (COLOR06 differs: cycling, inferred). `render_levels.py`'s W6 palette is L2ET read misaligned (wrong colours); its LB/LW use W6 tiles. docs/06 corrected. |
| 2026-10-11 | 📌 | T08: W6 has two map sets, both `L6MA1`–`4`: d4_c02–05 (one drive) and d4_c14–17 (DF1–3 with disk 1, h0+$a144); maps 2–4 differ. docs/LEVEL-INDEX's "W6-5..7" are these copies; left unedited (scope). |
| 2026-10-11 | 📌 | T08: hall font at h0+$378ca, charset h0+$322e; not in `re/symbols.toml` yet (the ANIM player is, T35). |
| 2026-10-11 | 📌 | T08 open: MPIC/FRUT palettes come from the copper (T20/T19); `LnLP`'s trailing 672/1 680 bytes and `LnMS`'s meaning unexplained; intro banks d3_c05–08: d3_c04 unpacks them itself (T32, `front::story`). Not reached by oracle: ET, FRUT/FRUC/FRUM, W2–W6 LP. |
| 2026-10-11 | 📌 | T08: the purity lexer counts a `("name", 1, 18)` tuple array as an integer table. So the chunk catalogue keeps only the game's file names in source; disk and sector come from its `file_table`. |
| 2026-10-11 | 📌 | T12: title eye-blink timers h0+$1368/$136a count down per frame and at -2 call `rng_range($4b)`. With `boot-to-title`, the RNG holds still from frame 4 405 to 4 461: the title-rng case's window. |
| 2026-10-11 | 🔄 | Owner, lag: conformance runs feed the original's 25 Hz pass schedule (from the oracle) to the engine; VBL work runs at one fixed point in a late pass; leftover divergence is reported as lag and re-synced. Normal play never lags. |
| 2026-10-11 | 🔄 | Owner, RNG: the seed is part of the start. Transplants copy `rng_state` (h0+$160d4, 8 bytes); power-on conformance runs take the oracle's seed for that script; normal play seeds from the host clock. |
| 2026-10-11 | 🔄 | Owner: T16–T18 re-split at plan review, before T16 starts. Proposed: one task per world (W1 12, W2 17, W3 18, W4 23, W5 11, W6 11 new types); T18 = final level + witch; bonus level's 15 types to T19. |
| 2026-10-11 | 📌 | T07 lag: level logic is 25 Hz (`level_frame_loop` waits for 2 VBLs); frog, camera, clock run 50 Hz in the VBL. The pass overran on 71 of 2 311 W1-1 play frames, 32 of 721 demo frames; title screens never. |
| 2026-10-11 | 📌 | RNG seeded once at h0+$7722 from `$bfd802` (CIA-B TOD, hsyncs since power-on). Clicking the crack intro 100 frames later shifts all RAM by 100 frames except `rng_state` and the stack. |
| 2026-10-11 | 📌 | Object placement is in the T7MP BODY: the 6-bit field is a per-world object marker (docs/09), so `.sflevel` needs only `t7mp`. docs/04 "Still open" and docs/07's solid rule (W3/W4 markers 24–28) are now wrong; left unedited (scope). |
| 2026-10-11 | 📌 | Stragglers d1_c16, d2_c39, d4_c16: the 46-byte trailer runs a sector past the carve; with it both decoders agree (d4_c16 is a T7MP level). d1_c16 matches in RAM bar its relocated samples; d2_c39, d4_c16 need T21's long run. |
| 2026-10-11 | 📌 | Disk 3 boots the story intro (d3_c02 → d3_c04), which then boots disk 1; d1_c03 never loads them; d3_c01 is unused. T32 plays it (`Start::Disk3`, docs/13). Music modules are "P41A" (The Player 4.1A) for T11. |
| 2026-10-11 | 📌 | `Amiga::cpu()` (shim only, no core patch): a step ends with the CPU at the level-3 handler's entry, so the interrupted main-program PC is the long at SSP+2. `exe_map_claims` adds ~45 s (three oracle runs) to the test block. |
| 2026-10-11 | 📌 | T10 review: `sf-paula` puts two full-scale channels at exactly ±32 768, so the band-limited step's ringing overshoot clamps on full-scale content (probe peaked at 32 768). Left as is; output headroom is T22's call. |
| 2026-10-11 | 📌 | Frames are not all 313 lines: the game sets then clears BPLCON0 LACE (frame 1 332), LOF sticks short, so the title music runs 312-line frames, 50.08 Hz. DESIGN §2.5 says 49.92; sf-core/T22 pacing must model LOF. |
| 2026-10-11 | 📌 | Not modelled, not asked by T10: the A500's analog low-pass (~4.4 kHz) and the LED filter. `sf-paula` outputs the raw DAC mix, band-limited. Whether the app needs them is T22/T30's call. |
| 2026-10-11 | 🐞 | T05 review sweep: `sf_conform::disks::python_decompress` uses scratch dir `sf-atn-ref-<pid>`, removed before use; two threads calling it at once would delete each other's disks. One caller per process today. Not fixed. |
| 2026-10-11 | 📌 | sf-oracle locates disks itself (`OracleConfig::locate`), duplicating `sf_conform::disks`: depending on sf-conform would cycle once sf-conform uses the oracle. Merge if a shared shell crate appears. |
| 2026-10-11 | 📌 | Oracle speed, M4, one core: ~314 frames/s from power-on, ~400 from a snapshot, ~250 hashing RAM and picture every frame; four processes at once ~95 each. Nobody drains audio, so `AUDBUF_OVERFLOW` warnings flood; T05 drains or disables it. |
| 2026-10-11 | 📌 | Stepping vAmiga without its thread: `Emulator::initialize()`, then per frame `update()` and `main.computeFrame()`; set `Thread::state` by hand, mark HALTED before `~VAmiga`, disable the unread message queue. `oracle.sh` core build: 2 min. |
| 2026-10-11 | 📌 | Input, worker-driven (T00, T05): the crack intro skips on a port-1 click held 1 000–1 004 (title by 4 500; one at 300 is ignored; none: still the intro at 11 200); port-2 fire opens the menu and starts W1-1; P pauses within 2 frames. |
| 2026-10-11 | 📌 | Kickstart: `kick1.3.rom` (34.5, SHA-1 `90933936cce43ca9bc6bf375662c076b27e3c458`) boots disk 1 to the title; `KICK204.ROM` (37.350, `02843c42…`) too. AROS 2026-08-20 and `Kick.rom` hit the game's "requires 1MB" error; A1200 3.1 stays black. |
| 2026-10-11 | 📌 | T02: Ghidra adopted. Ghidra 12.0 (brew formula, not cask) and the ghidra-amiga extension were already installed, contrary to DESIGN §5. Headless import takes 6–30 s and needs JAVA_HOME (openjdk@21). |
| 2026-10-11 | 📌 | Ghidra's Amiga loader refuses `d1_c03`/`d3_c04` ("No memory blocks") until the absent hunks 2–3 are appended as empty HUNK_BSS. The game's loader counts HUNK_ENDs per declared hunk, so it reads past those files' end: T07 to explain. |
| 2026-10-11 | 📌 | `atn_decompress` is in every executable: byte-identical in `d1_c01`, `d3_c01`, `d3_c02`; `d1_c03`/`d3_c04` use one `bne.w` (2 bytes longer). `d3_c04` also has the stage-1.5 hunk loader at h0+$3d0. |
| 2026-10-11 | 📌 | Homebrew's rustup is keg-only and disables `rustup self uninstall`; `install-rust.sh` runs `brew link --force rustup` so bare `cargo` works. Way back is `brew uninstall rustup && rm -rf ~/.rustup`, not §5.3's command. |
| 2026-10-10 | 📌 | `~/Documents` is blocked by macOS privacy protection for every pir and Claude shell, even when the person runs a handed command. The Kickstart ROMs were copied by the owner to `~/amiga-roms/`; use that path. |
| 2026-10-10 | 📌 | vAmiga (commit `899da5dd`, 2026-09-30) has a headless target `VAHeadless` in `Core/CMakeLists.txt`; Core is MPL-2.0, Moira CPU MIT, app GPL. |
