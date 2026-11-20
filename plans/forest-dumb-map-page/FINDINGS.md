# Findings log

What the build taught. Newest first, forty words a row, counted. Flat prose, at most one bold phrase a row.
Whoever appends, compacts first when the file passes 60 rows or 15 KB. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-20 | 🔄 | The brief proposed the live marker and secrets as later work; the owner put both in this plan. |
| 2026-11-20 | 📌 | Headless Chrome 157 re-reads a `file://` script inserted with `?t=` every 300 ms after the file is replaced by rename (planner probe, CDP over a socket). |
| 2026-11-20 | 📌 | No `33fc 0282` (move.w #282) in the main program: W1-1's items-left start value is not a plain immediate word store. T00 finds its source. |
| 2026-11-20 | 📌 | Tiles with more than one collision shape: W1-1 44 of 328, W2-1 27, W3-1 80, W4-1 36, W5-1 8, W6-1 13. Candidates for secrets, mostly unexplained. |
| 2026-11-20 | 📌 | Unpacked object maps, used columns: only `$34` is outside W1-1's table (W3-1 31, W4-1 10, W5-1 25, W6-1 17). Every world's exit is a 2×3 block of `$ff`. |
