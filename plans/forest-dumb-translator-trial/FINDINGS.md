# Findings log

**What the build taught.** Newest first, forty words a row, counted. The long version is in the commit message. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-07 | 🐞 | Plan review: Ghidra 12.0's Amiga loader records no relocations (`getRelocationTable()` empty) for `FOREST_DUMB_FOREVER`, whose hunks carry 4 254 `HUNK_RELOC32` entries. T01 reads them from the file (owner, 2026-11-07). |
| 2026-11-07 | 📌 | Ghidra 12.0 headless auto-analysis disassembles only 3 837 instructions of `FOREST_DUMB_FOREVER`; entries must be seeded from `re/symbols.toml`. Hunk 0 loads at `$21f000` in Ghidra, `$c077d8` on the oracle. P-code flags are registers `0x44`-`0x47`. |
