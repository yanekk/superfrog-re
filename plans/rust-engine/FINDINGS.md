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
| 2026-10-11 | 📌 | Frames are not all 313 lines: the game sets then clears BPLCON0 LACE (frame 1 332), LOF sticks short, so the title music runs 312-line frames, 50.08 Hz. DESIGN §2.5 says 49.92; sf-core/T22 pacing must model LOF. |
| 2026-10-11 | 📌 | An oracle step's writes and taps run from just after its frame's start to just after the next one's: a position going backwards marks the next frame's first clocks. `sf-paula` and the T10 test carry them over. |
| 2026-10-11 | 📌 | vAmiga's own mix: 2 048 samples of pre-fill, point-sampled (no anti-aliasing), synthesised once per frame 50 lines behind. Ours differs by 0.18 relative RMS on the title music, all high-frequency; T10 tolerance 0.25. |
| 2026-10-11 | 📌 | Not modelled, not asked by T10: the A500's analog low-pass (~4.4 kHz) and the LED filter. `sf-paula` outputs the raw DAC mix, band-limited. Whether the app needs them is T22/T30's call. |
| 2026-10-11 | 🐞 | Fixed in T10: the shim drained host audio with `copyMono(8192)`, an underflow every frame that padded the stream. It now copies exactly what is there. |
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
