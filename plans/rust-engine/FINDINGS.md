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
| 2026-10-12 | 🐞 | T31: docs/10 and the T08 row had W6's map sets backwards. One drive plays d4_c14–c17 (`alt`); any connected df1–df3 gives c02–c05 (`probe_drives`: 0 = connected). T13's render-w6-2..4 rendered alt; now reached with df1. |
| 2026-10-12 | 🔄 | Owner, T31: the final level is reached by writing `level_state` = 1 on W6-4's first frame (no code, cheat or trainer leads there). Its score and lives are not a playthrough's. `Amiga::write_memory` exists for this alone. |
| 2026-10-12 | 📌 | T31: with an empty df1 connected the boot reaches the title 219 frames sooner. With disks 2–4 in df1–df3 the crack stalls on ACCESSING DISK past frame 20 000; not investigated (one empty df1 suffices). |
| 2026-10-12 | 📌 | T13 review: owner added T31 render-remaining-levels. T13 renders 25 of 29 levels; W6's alt maps 2–4 need DF1–DF3 (oracle has df0 only) and the final level has no level code (password entries 24–27 all lead to the bonus level). |
| 2026-10-12 | 📌 | T13 review: `late_redraw` excuses a differing pixel wherever an alternative differs from end-of-frame, not where it matches the original (strict matching fails 10 of 25 cases on genuine mixed frames). A renderer defect confined to changed regions is skipped, not caught. |
| 2026-10-11 | 🔄 | Owner, T13: the two display buffers (h3+$10600, $22f30) are engine state; the play field renders from them, the map only for widescreen. T14–T18 must port the original's tile, animation and bob blits into them. Save states grow ~150 KB. |
| 2026-10-11 | 📌 | T13: ~4% of level frames have late redraws (copper, panel or shown buffer rewritten after the beam). Transplant render tests list and skip them; stepping conformance (T14 on) must still match them, e.g. by applying blits in beam order. |
| 2026-10-11 | 📌 | T13: LAYOUT now carries the level's display (copper list, panel, both buffers, map, camera, ~230 KB). A vars-comparing case in a level diverges until T14's mechanics write them. |
| 2026-10-11 | 📌 | T13: no colour cycling in levels. Level palettes change only in the start fade (h0+$153a4), exit fade ($155a0) and boss flash ($151b2). T08's "COLOR06 cycling, inferred" was not seen in 8 500 frames of W1-1. |
| 2026-10-11 | 📌 | T13: oracle reaches every W1–W6 level and WB-1 by level code (sync::reach); W6's alt maps need a second drive and the final level has no code, so neither is rendered-tested. |
| 2026-10-11 | 📌 | T13, for the bob port: blitter ignores bit 0 of a recorded destination; buffer 0's draw x survives only in its seam sub-record (+$8c), the upper one is already the next pass's. Sprites only show on lines with a bitplane fetch. |
| 2026-10-11 | 📌 | T11 review, at merge: T12 landed `LAYOUT` without the driver's variables, and no task doc maps them (docs/11 lists them; `sf_core::audio::vars` has the blocks). T14, the first to step the driver from `Game`, is the natural owner. |
| 2026-10-11 | 🐞 | T11 review: at a fade's end `music_stop` leaves `a1` at `sfx_steal`, so the original's shadow copy writes RAM `$13024..$1305e`, not Paula. The driver wrote Paula. Fixed; `fades_match_the_original` locks it. Only that caller uses `a1` afterwards. |
| 2026-10-11 | 📌 | T11 review: fake unverified against real: Moira hardware mode records a byte write to a custom register as that byte on both halves. The driver never makes one; nothing checks it against vAmiga. |
| 2026-10-11 | 📌 | T11: `Cpu68k::set_hardware(true)` runs routines that drive Paula: writes recorded in `hw_writes`, VHPOSR steps per read, CIAs read 0. Routines waiting on DMA or interrupts still time out. `Cpu68k::apply` keeps a call's memory changes. |
| 2026-10-11 | 📌 | T11: Paula pointers are absolute, so files must sit where the game puts them: a world's module right after its bank's effects (`sfx_bank_init` sets h0+$9ec), the title's MUSC at the bank's own place. |
| 2026-10-11 | 📌 | T11: the title's main loop writes DMACON $8020/$8400 and INTENA $4000/$c000 pairs around loading. The audio comparison filters them; whole-frame Paula comparison (T12 on) must expect them from non-sound code. |
| 2026-10-11 | 📌 | T11, left alone: no `sf-core::layout` (T12) – driver variables are `sf_core::audio::vars` and docs/11. Final level's sound bank unchecked. The mixer's chip view needs the stream buffer (`stream_source`) and loaded files (T22). |
| 2026-10-11 | 📌 | T08: level palette = 32 colours after `LnLP` (+$11300), proven on W1-1's copper (COLOR06 differs: cycling, inferred). `render_levels.py`'s W6 palette is L2ET read misaligned (wrong colours); its LB/LW use W6 tiles. docs/06 corrected. |
| 2026-10-11 | 📌 | T08: W6 has two map sets, both `L6MA1`–`4`: d4_c02–05 (one drive) and d4_c14–17 (DF1–3 with disk 1, h0+$a144); maps 2–4 differ. docs/LEVEL-INDEX's "W6-5..7" are these copies; left unedited (scope). |
| 2026-10-11 | 📌 | T08: the game's ANIM player (h0+$ab02 open, $ab82 next) is pure: on the routine oracle it plays END1 in 0.5 s. Hall font at h0+$378ca, charset h0+$322e. Neither is in `re/symbols.toml` yet (scope). |
| 2026-10-11 | 📌 | T08 open: MPIC/FRUT palettes come from the copper (T20/T19); `LnLP`'s trailing 672/1 680 bytes and `LnMS`'s meaning unexplained; intro banks d3_c05–08 kept raw, layout ambiguous (T20). Not reached by oracle: ET, FRUT/FRUC/FRUM, W2–W6 LP. |
| 2026-10-11 | 📌 | T08: the purity lexer counts a `("name", 1, 18)` tuple array as an integer table. So the chunk catalogue keeps only the game's file names in source; disk and sector come from its `file_table`. |
| 2026-10-11 | 📌 | T12: `rng_seed` (h0+$1607a) has no `rts`; it runs on into `rng_next`, so seeding also advances the state once. `Rng::seeded` does both; symbols.toml note corrected; docs/08 "RNG" does not say it. |
| 2026-10-11 | 📌 | T12: title eye-blink timers h0+$1368/$136a count down per frame and at -2 call `rng_range($4b)`. With `boot-to-title`, the RNG holds still from frame 4 405 to 4 461: the title-rng case's window. |
| 2026-10-11 | 📌 | T12: sync-point snapshots are cached in `~/.cache/superfrog-re/sync/`, keyed by vAmiga build, ROM and disk file hashes and the script. Delete to force a fresh run; diff PNGs go to `$TMPDIR/sf-conform/`. |
| 2026-10-11 | 🐞 | T07 review: vAmiga's `Defaults::fallbacks()` fills a static map unlocked on first use; machines powered on in parallel could read it half-filled ("Invalid key: ROM_PATH", seen once). Fixed: `sf_amiga_new` sets up under a mutex. No other set-up path. |
| 2026-10-11 | 🔄 | Owner, lag: conformance runs feed the original's 25 Hz pass schedule (from the oracle) to the engine; VBL work runs at one fixed point in a late pass; leftover divergence is reported as lag and re-synced. Normal play never lags. |
| 2026-10-11 | 🔄 | Owner, RNG: the seed is part of the start. Transplants copy `rng_state` (h0+$160d4, 8 bytes); power-on conformance runs take the oracle's seed for that script; normal play seeds from the host clock. |
| 2026-10-11 | 🔄 | Owner: T16–T18 re-split at plan review, before T16 starts. Proposed: one task per world (W1 12, W2 17, W3 18, W4 23, W5 11, W6 11 new types); T18 = final level + witch; bonus level's 15 types to T19. |
| 2026-10-11 | 📌 | T07 lag: level logic is 25 Hz (`level_frame_loop` waits for 2 VBLs); frog, camera, clock run 50 Hz in the VBL. The pass overran on 71 of 2 311 W1-1 play frames, 32 of 721 demo frames; title screens never. |
| 2026-10-11 | 📌 | RNG seeded once at h0+$7722 from `$bfd802` (CIA-B TOD, hsyncs since power-on). Clicking the crack intro 100 frames later shifts all RAM by 100 frames except `rng_state` and the stack. |
| 2026-10-11 | 📌 | Object placement is in the T7MP BODY: the 6-bit field is a per-world object marker (docs/09), so `.sflevel` needs only `t7mp`. docs/04 "Still open" and docs/07's solid rule (W3/W4 markers 24–28) are now wrong; left unedited (scope). |
| 2026-10-11 | 📌 | Stragglers: the game's `file_table` reads a sector past the carve for all three. d1_c16 (W1 music) matches in RAM bar its relocated sample table; d2_c39 (bonus loading picture) and d4_c16 (W6-3) need T21's long run. |
| 2026-10-11 | 📌 | Disk 3 boots the story intro (d3_c02 → d3_c04), which then boots disk 1; d1_c03 never loads them; d3_c01 is unused. Whether the engine plays the intro is T20/T21's call. Music modules are "P41A" (The Player 4.1A) for T11. |
| 2026-10-11 | 📌 | `Amiga::cpu()` (shim only, no core patch): a step ends with the CPU at the level-3 handler's entry, so the interrupted main-program PC is the long at SSP+2. `exe_map_claims` adds ~45 s (three oracle runs) to the test block. |
| 2026-10-11 | 📌 | T10 review: `sf-paula` puts two full-scale channels at exactly ±32 768, so the band-limited step's ringing overshoot clamps on full-scale content (probe peaked at 32 768). Left as is; output headroom is T22's call. |
| 2026-10-11 | 📌 | Frames are not all 313 lines: the game sets then clears BPLCON0 LACE (frame 1 332), LOF sticks short, so the title music runs 312-line frames, 50.08 Hz. DESIGN §2.5 says 49.92; sf-core/T22 pacing must model LOF. |
| 2026-10-11 | 📌 | An oracle step's writes and taps run from just after its frame's start to just after the next one's: a position going backwards marks the next frame's first clocks. `sf-paula` and the T10 test carry them over. |
| 2026-10-11 | 📌 | vAmiga's own mix: 2 048 samples of pre-fill, point-sampled (no anti-aliasing), synthesised once per frame 50 lines behind. Ours differs by 0.18 relative RMS on the title music, all high-frequency; T10 tolerance 0.25. |
| 2026-10-11 | 📌 | Not modelled, not asked by T10: the A500's analog low-pass (~4.4 kHz) and the LED filter. `sf-paula` outputs the raw DAC mix, band-limited. Whether the app needs them is T22/T30's call. |
| 2026-10-11 | 📌 | vAmiga builds Moira with `MOIRA_VIRTUAL_API false`, memory hooks in its `CPU.cpp`; T06 compiles a second copy with `-Dvamiga=sf_moira` and two stub headers. Calls start through `reset()`, with `read16OnReset` handing it the entry and stack. |
| 2026-10-11 | 🐞 | T05 review sweep: `sf_conform::disks::python_decompress` uses scratch dir `sf-atn-ref-<pid>`, removed before use; two threads calling it at once would delete each other's disks. One caller per process today. Not fixed. |
| 2026-10-11 | 📌 | vAmiga snapshots omit the joystick, mouse-button (2-frame debounced `Sticky`) and key-down state, and ports "detect" their device on first use. The shim pins mouse/joystick devices and re-seats held input on restore (patch 0003). |
| 2026-10-11 | 📌 | Texture column for lores DIW position h is `4 * (h - 0x24)`: Denise's counter starts each line at HBLANK_MIN×2. Title window is DIWSTRT 0x2C81, 320×256; the crack intro uses a wider hires window. |
| 2026-10-11 | 📌 | T05, worker-driven: port-1 click held at frames 1 000–1 004 gives the title by 4 500 (seen: frog in cape, ©1993 Team 17); a click at 300 is ignored. Release build ~430–490 frames/s. |
| 2026-10-11 | 📌 | Stepping vAmiga without its thread: set `Thread::state` by hand (switchState throws unless launched), mark HALTED before `~VAmiga`, disable the unread message queue. `oracle.sh` core build: 2 min. |
| 2026-10-11 | 📌 | sf-oracle locates disks itself (`OracleConfig::locate`), duplicating `sf_conform::disks`: depending on sf-conform would cycle once sf-conform uses the oracle. Merge if a shared shell crate appears. |
| 2026-10-11 | 📌 | T00 review: after `.vasnap` restore, RAM and Paula match for 3 199 frames, but the 912×313 texture differs on row 312 only (vblank filler `0x444444` vs `0x222222`), every other frame. Compare the display window only, as T05 plans. |
| 2026-10-11 | 📌 | T00 review, worker-driven probe: with no input, frame 11 200 is still the crack intro; a port-1 click at frame 3 000 gives the title at 6 350–6 700. Two 11 300-frame runs identical. KS 2.04 boots; Kick.rom refused. |
| 2026-10-11 | 📌 | T00, worker-driven: captured frame 11 200 (KS 1.3, power-on) shows the Superfrog title as the original: frog in cape, "©1993 Team 17 Software Ltd", "Lucozade is a trademark". Attract mode, tables, credits, World 1-1 play also seen. |
| 2026-10-11 | 📌 | Oracle speed, M4, one core: ~314 frames/s from power-on, ~400 from a snapshot, ~250 hashing RAM and picture every frame; four processes at once ~95 each. Nobody drains audio, so `AUDBUF_OVERFLOW` warnings flood; T05 drains or disables it. |
| 2026-10-11 | 📌 | Picture: set `Opt::MON_PALETTE` to `Palette::RGB`, and again after loading a snapshot (it restores config); then 12-bit OCS colour = each texel channel `>> 4`, EHB included. Default palette is lossy. Texture is 912×313, 2 texels per lores pixel. |
| 2026-10-11 | 📌 | Paula writes need a core patch: a hook at the top of `Memory::pokeCustom16` (CPU and Copper both pass there), logging 0x96, 0x9A, 0x9C, 0x9E, 0xA0–0xDE with beam position. RAM needs none: `mem.chip`, `mem.slow`. |
| 2026-10-11 | 📌 | Stepping without vAmiga's thread: `Emulator::initialize()`, then per frame `update()` (applies queued JOY/KEY/MOUSE commands) and `main.computeFrame()` (runs to vsync). Spike reached privates by `#define private public`; T05 needs a friend or API patch. |
| 2026-10-11 | 📌 | Input: the crack intro skips on a port-1 mouse click (fire does not); port-2 fire opens the title menu and starts World 1-1, loaded from disk 1 alone; key P (raw 0x19) pauses within 2 frames. |
| 2026-10-11 | 📌 | Reproducible: A500 OCS, 512K chip + 512K slow, RTC none, RAM zeroed, warp never. Two power-on runs of 12 001 frames matched every frame (RAM, 912×313 picture, ordered Paula writes); `.vasnap` restore in a new process continues identically. |
| 2026-10-11 | 📌 | Kickstart: `kick1.3.rom` (34.5, SHA-1 `90933936cce43ca9bc6bf375662c076b27e3c458`) boots disk 1 to the title; `KICK204.ROM` (37.350, `02843c42…`) too. AROS 2026-08-20 and `Kick.rom` hit the game's "requires 1MB" error; A1200 3.1 stays black. |
| 2026-10-11 | 📌 | vAmiga pinned at `899da5ddd872dfb2e9e35e80247cbb2b36086524`: unmodified Core builds with cmake 4.4.4 in 6 min (-j10). Link `libVACore`, `librvlib`, `libutlib`, `libxdms`, `-lz`, CoreMIDI, CoreFoundation. Spike build plus hook patch kept in `~/.cache/superfrog-re/vamiga/899da5dd-spike-*`. |
| 2026-10-11 | 📌 | Stragglers `d1_c16`, `d2_c39`, `d4_c16`: the 46-byte trailer at `comp_size` runs past the carved sector. With the next sector appended both decoders agree; `d4_c16` is a 300×84 T7MP level. Carvers unchanged; T08/T09 must extend. |
| 2026-10-11 | 📌 | The purity test scans pure crates' `tests/` too: code there that runs Python or touches files belongs in `sf-conform` (`python_carve`, `python_decompress`). |
| 2026-10-11 | 📌 | capstone m68k prints a pc-relative operand as opcode+2+disp, wrong by 2 after `movem`'s mask or `btst #n`. T02 review fixed `re_listing.py`; sweep found no other disassembler in the repo. |
| 2026-10-11 | 📌 | T02: Ghidra adopted. Ghidra 12.0 (brew formula, not cask) and the ghidra-amiga extension were already installed, contrary to DESIGN §5. Headless import takes 6–30 s and needs JAVA_HOME (openjdk@21). |
| 2026-10-11 | 🐞 | amitools: `HunkReader.read_mem` breaks on bytes (use `read_file_obj` with BytesIO). `HunkRelocate` raises IndexError on `d1_c03` and `d3_c04`, which declare four hunks but carry two. Fixed by adding empty BSS hunks; no own parser. |
| 2026-10-11 | 📌 | Ghidra's Amiga loader refuses `d1_c03`/`d3_c04` ("No memory blocks") until the absent hunks 2–3 are appended as empty HUNK_BSS. The game's loader counts HUNK_ENDs per declared hunk, so it reads past those files' end: T07 to explain. |
| 2026-10-11 | 📌 | `atn_decompress` is in every executable: byte-identical in `d1_c01`, `d3_c01`, `d3_c02`; `d1_c03`/`d3_c04` use one `bne.w` (2 bytes longer). `d3_c04` also has the stage-1.5 hunk loader at h0+$3d0. |
| 2026-10-11 | 📌 | `unittest discover -s .` does not enter `tools/` unless it is a package. T02 added an empty `tools/__init__.py`, so `tools/test_*.py` now run. |
| 2026-10-11 | 📌 | Homebrew's rustup is keg-only and disables `rustup self uninstall`; `install-rust.sh` runs `brew link --force rustup` so bare `cargo` works. Way back is `brew uninstall rustup && rm -rf ~/.rustup`, not §5.3's command. |
| 2026-10-10 | 📌 | `~/Documents` is blocked by macOS privacy protection for every pir and Claude shell, even when the person runs a handed command. The Kickstart ROMs were copied by the owner to `~/amiga-roms/`; use that path. |
| 2026-10-10 | 📌 | vAmiga (commit `899da5dd`, 2026-09-30) has a headless target `VAHeadless` in `Core/CMakeLists.txt`; Core is MPL-2.0, Moira CPU MIT, app GPL. |
| 2026-10-10 | 📌 | Fresh-venv `pip install -r requirements.txt` works on Python 3.14.8 (machine68k wheel included); extract+unpack of all four disks takes 0.5 s and still gives 103/106 chunks. |
| 2026-10-10 | 📌 | Five hunk executables: `d1_c01` and `d3_c01` (15 KB each), `d1_c03` (331 KB), `d3_c02` (182 KB), `d3_c04` (343 KB). Roles unknown; T07 maps them. |
