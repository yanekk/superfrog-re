# Findings log

**What the build taught.** Newest first, forty words a row, counted. The long version is in the commit message. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-07 | ✅ | **Owner: "Go"** on the trial (T08), picked after reading the gate reading: 84 of 85 judgeable pass with no hand edit (98.8%), 0 code writes on level 1's path. No note for the second plan. |
| 2026-11-07 | 📌 | **T07 last run:** `{"judgeable":85,"pass":84,"pass_pct":98.8,"pass_with_hints":84,"unjudgeable":2,"unsupported":0,"untranslated":0,"write_into_code":1}`. Gate met with no hint; `translator-hints.toml` still holds none. |
| 2026-11-07 | 📌 | **T07 categories** of T06's 10 failures: Ghidra's P-code, corrected in the translator, 9 (`abcd`/`sbcd`); needs stage 2, 1 (`h0_6852`); fixed otherwise 0; needs a hint 0; unexplained 0. Two sound routines stay unjudgeable. |
| 2026-11-07 | 🐞 | For stage 1's Moira check: Ghidra 12.0's `abcd`/`sbcd` call undefined `bcdAdjust`, `abcd` drops its result, Z is set on non-zero, `-(a7)` steps 1. `pcode_rust.py` emits them via `pcode_rt::bcd` (Moira's arithmetic). `nbcd` stays unsupported. |
| 2026-11-07 | 🐞 | For stage 1's Moira check: Ghidra 12.0's `subflags` sets X on `cmp`, `cmpa`, `cmpi`, `cmpm`; the 68000 leaves X. Broke three routines through `abcd` after `cmpi`. `pcode_rust.py` drops that copy (`without_compare_x`). |
| 2026-11-07 | 📌 | `h0_6852` fails 34 of 2 000 states on `write-into-code: h0+$68f4`: its generator gives slot `d7` up to 33, the game 0 to 4 (`h0_681c`), so the slot write reaches code. Needs stage 2; T05 saw no such write. |
| 2026-11-07 | 📌 | **T06 first real run:** `{"judgeable":85,"pass":75,"pass_pct":88.2,"pass_with_hints":75,"unjudgeable":2,"unsupported":9,"untranslated":0,"write_into_code":1}`. Nine fail on `abcd`/`sbcd` (CALLOTHER unsupported), one `write-into-code: h0+$68f4` (`h0_6852`). Two minutes; no timeout, 120 s kept. |
| 2026-11-07 | 🐞 | T01 marks a relocated op output `"slot": -1` (272 of 1 148 relocs); T03 read it as the last input and crashed on the game. Fixed in T06 with the owner's yes (`pcode_rust.py`, test in `test_pcode_rust.py`). |
| 2026-11-07 | 🐞 | Open, for T06/T07: endless recursion (`bsr` to itself) overflows the Rust stack and aborts (SIGABRT), not `Stop::Hang`: the step counter is per call. Needs a depth cap and a large-stack judge thread. |
| 2026-11-07 | 📌 | Fake unverified against real: T03's fixtures are hand-written P-code. Register offsets and the `bsr`/`rts`/`dbcc` forms match Ghidra 12.0's `68000.sinc`; the export format itself waits for T01. |
| 2026-11-07 | 📌 | For T04: after an entry returns, `PC` holds the address its `rts` popped, for the judge to check. The tree exports `HUNK_SIZES` for `Image::sizes`. A 9 000-instruction tree builds in 51 s optimised, 4 s not. |
| 2026-11-07 | 📌 | A return to an address that is neither a block of the caller nor a translated entry stops `untranslated`, including code that drops its return address to return to its caller's caller. T07 may meet it. |
| 2026-11-07 | 🐞 | T04 review sweep: `judge::check_regs` (`judge/mod.rs:60`) gives a Muse port a7 = STACK while the oracle enters the original at STACK - 4 holding its return address; a port reading the stack literally would differ by 4. Not fixed. |
| 2026-11-07 | 📌 | T05 review: DESIGN §4's code-writes row says instructions the listing does not know are not watched; `code_writes.rs` watches every even hunk address instead, so that limit no longer holds. Left for the owner's DESIGN edit. |
| 2026-11-07 | 📌 | **Code-writes run (T05):** `fd-conform code-writes` over 33 level 1 cases, unfrozen: `{"total":{"cases":33,"frames":80964,"changed":0}}`. No executed instruction changed, against the loaded program or frame to frame. `first-case` skipped (title). |
| 2026-11-07 | 📌 | T04 left `fd-conform` without an opt-level override: it would slow every build of the crate, and a translated tree's judge speed is unmeasured. T06's first real run shows whether `[profile.dev.package.fd-conform]` is needed under the 120 s limit. |
| 2026-11-07 | 📌 | `pcode-rt`'s ops are `#[inline]`, so they compile at the calling crate's opt-level: the `pcode-rt` override does not speed generated code. T04 should give the crate building `tree.rs` (`fd-conform`, unoptimised today) its own override. |
| 2026-11-07 | 📌 | **T01 on the game:** 87 entries, 3 962 instructions, every entry's reach equal to `translate.toml`. Of 4 254 `HUNK_RELOC32` entries, 1 002 fall inside exported instructions, each on an operand; 0 unexplained in-range operands, 0 unresolved computed jumps. |
| 2026-11-07 | 🐞 | Ghidra 12.0 overrides a `bra.w` into another function as `CALL_RETURN` (`h0+$4ea2`), and the Amiga loader's pointer marks block decoding ("conflicting data", 32 on the game). `ExportPcode.java` reads the unoverridden flow and clears marks inside reached instructions. |
| 2026-11-07 | 📌 | **Baseline:** T00 merged `pir/forest-dumb-level-1` at `defe7d6b` cleanly (merge `9e6ee954`). `fd-judge` builds, `--test judge` passes 2/2, and Muse's port of `h0+$5462` (`char_sprite_position`) passes 2000/2000, 4 paths. |
| 2026-11-07 | 🐞 | Plan review: Ghidra 12.0's Amiga loader records no relocations (`getRelocationTable()` empty) for `FOREST_DUMB_FOREVER`, whose hunks carry 4 254 `HUNK_RELOC32` entries. T01 reads them from the file (owner, 2026-11-07). |
| 2026-11-07 | 📌 | Ghidra 12.0 headless auto-analysis disassembles only 3 837 instructions of `FOREST_DUMB_FOREVER`; entries must be seeded from `re/symbols.toml`. Hunk 0 loads at `$21f000` in Ghidra, `$c077d8` on the oracle. P-code flags are registers `0x44`-`0x47`. |
