# Findings log

**What the build taught.** Newest first, forty words a row, counted. The long version is in the commit message. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-07 | 🐞 | Open, for T01: 68000 PC-relative modes compute `inst_start + 2 + d16` (`68000.sinc:447`), a `const` of Ghidra's address, not relocated, not `ram`. T01 must write it `hN+$off`, like `inst_next`, or `(d16,PC)` accesses go wrong. |
| 2026-11-07 | 🐞 | Open, for T04: endless recursion (`bsr` to itself) overflows the Rust stack and aborts (SIGABRT), not `Stop::Hang`: the step counter is per call. Needs a depth cap and a large-stack judge thread. |
| 2026-11-07 | 📌 | Fake unverified against real: T03's fixtures are hand-written P-code. Register offsets and the `bsr`/`rts`/`dbcc` forms match Ghidra 12.0's `68000.sinc`; the export format itself waits for T01. |
| 2026-11-07 | 📌 | T01 must write a hunk-relative varnode offset as the string `hN+$off` (or via `relocs`), including the `inst_next` constant `bsr`/`jsr` push: a raw Ghidra address there fails every call's return check. `pcode_rust.py` names a `CALLOTHER` by its optional `name`. |
| 2026-11-07 | 📌 | For T04: after an entry returns, `PC` holds the address its `rts` popped, for the judge to check. The tree exports `HUNK_SIZES` for `Image::sizes`. A 9 000-instruction tree builds in 51 s optimised, 4 s not. |
| 2026-11-07 | 📌 | A return to an address that is neither a block of the caller nor a translated entry stops `untranslated`, including code that drops its return address to return to its caller's caller. T07 may meet it. |
| 2026-11-07 | 📌 | `pcode-rt`'s ops are `#[inline]`, so they compile at the calling crate's opt-level: the `pcode-rt` override does not speed generated code. T04 should give the crate building `tree.rs` (`fd-conform`, unoptimised today) its own override. |
| 2026-11-07 | 📌 | **Baseline:** T00 merged `pir/forest-dumb-level-1` at `defe7d6b` cleanly (merge `9e6ee954`). `fd-judge` builds, `--test judge` passes 2/2, and Muse's port of `h0+$5462` (`char_sprite_position`) passes 2000/2000, 4 paths. |
| 2026-11-07 | 🐞 | Plan review: Ghidra 12.0's Amiga loader records no relocations (`getRelocationTable()` empty) for `FOREST_DUMB_FOREVER`, whose hunks carry 4 254 `HUNK_RELOC32` entries. T01 reads them from the file (owner, 2026-11-07). |
| 2026-11-07 | 📌 | Ghidra 12.0 headless auto-analysis disassembles only 3 837 instructions of `FOREST_DUMB_FOREVER`; entries must be seeded from `re/symbols.toml`. Hunk 0 loads at `$21f000` in Ghidra, `$c077d8` on the oracle. P-code flags are registers `0x44`-`0x47`. |
