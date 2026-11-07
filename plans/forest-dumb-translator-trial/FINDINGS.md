# Findings log

**What the build taught.** Newest first, forty words a row, counted. The long version is in the commit message. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-07 | 📌 | T04 left `fd-conform` without an opt-level override: it would slow every build of the crate, and a translated tree's judge speed is unmeasured. T06's first real run shows whether `[profile.dev.package.fd-conform]` is needed under the 120 s limit. |
| 2026-11-07 | 📌 | `pcode-rt`'s ops are `#[inline]`, so they compile at the calling crate's opt-level: the `pcode-rt` override does not speed generated code. T04 should give the crate building `tree.rs` (`fd-conform`, unoptimised today) its own override. |
| 2026-11-07 | 📌 | **Baseline:** T00 merged `pir/forest-dumb-level-1` at `defe7d6b` cleanly (merge `9e6ee954`). `fd-judge` builds, `--test judge` passes 2/2, and Muse's port of `h0+$5462` (`char_sprite_position`) passes 2000/2000, 4 paths. |
| 2026-11-07 | 🐞 | Plan review: Ghidra 12.0's Amiga loader records no relocations (`getRelocationTable()` empty) for `FOREST_DUMB_FOREVER`, whose hunks carry 4 254 `HUNK_RELOC32` entries. T01 reads them from the file (owner, 2026-11-07). |
| 2026-11-07 | 📌 | Ghidra 12.0 headless auto-analysis disassembles only 3 837 instructions of `FOREST_DUMB_FOREVER`; entries must be seeded from `re/symbols.toml`. Hunk 0 loads at `$21f000` in Ghidra, `$c077d8` on the oracle. P-code flags are registers `0x44`-`0x47`. |
