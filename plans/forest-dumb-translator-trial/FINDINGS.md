# Findings log

**What the build taught.** Newest first, forty words a row, counted. The long version is in the commit message. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
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
