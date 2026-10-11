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
| 2026-10-11 | 📌 | capstone m68k prints a pc-relative operand as opcode+2+disp, wrong by 2 after `movem`'s mask or `btst #n`. T02 review fixed `re_listing.py`; sweep found no other disassembler in the repo. |
| 2026-10-11 | 📌 | T02: Ghidra adopted. Ghidra 12.0 (brew formula, not cask) and the ghidra-amiga extension were already installed, contrary to DESIGN §5. Headless import takes 6–30 s and needs JAVA_HOME (openjdk@21). |
| 2026-10-11 | 🐞 | amitools: `HunkReader.read_mem` breaks on bytes (use `read_file_obj` with BytesIO). `HunkRelocate` raises IndexError on `d1_c03` and `d3_c04`, which declare four hunks but carry two. Fixed by adding empty BSS hunks; no own parser. |
| 2026-10-11 | 📌 | Ghidra's Amiga loader refuses `d1_c03`/`d3_c04` ("No memory blocks") until the absent hunks 2–3 are appended as empty HUNK_BSS. The game's loader counts HUNK_ENDs per declared hunk, so it reads past those files' end: T07 to explain. |
| 2026-10-11 | 📌 | `atn_decompress` is in every executable: byte-identical in `d1_c01`, `d3_c01`, `d3_c02`; `d1_c03`/`d3_c04` use one `bne.w` (2 bytes longer). `d3_c04` also has the stage-1.5 hunk loader at h0+$3d0. |
| 2026-10-11 | 📌 | `unittest discover -s .` does not enter `tools/` unless it is a package. T02 added an empty `tools/__init__.py`, so `tools/test_*.py` now run. |
| 2026-10-11 | 📌 | Homebrew's rustup is keg-only and disables `rustup self uninstall`; `install-rust.sh` runs `brew link --force rustup` so bare `cargo` works. Way back is `brew uninstall rustup && rm -rf ~/.rustup`, not §5.3's command. |
| 2026-10-10 | 📌 | `~/Documents` is blocked by macOS privacy protection for every pir and Claude shell, even when the person runs a handed command. The Kickstart ROMs were copied by the owner to `~/amiga-roms/`; use that path. |
| 2026-10-10 | 📌 | vAmiga (commit `899da5dd`, 2026-09-30) has a headless target `VAHeadless` in `Core/CMakeLists.txt`; Core is MPL-2.0, Moira CPU MIT, app GPL. cmake is not installed (brew has 4.4.4). |
| 2026-10-10 | 📌 | Fresh-venv `pip install -r requirements.txt` works on Python 3.14.8 (machine68k wheel included); extract+unpack of all four disks takes 0.5 s and still gives 103/106 chunks. |
| 2026-10-10 | 📌 | Five hunk executables: `d1_c01` and `d3_c01` (15 KB each), `d1_c03` (331 KB), `d3_c02` (182 KB), `d3_c04` (343 KB). Roles unknown; T07 maps them. |
