# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** once this file is over 60 rows or 15 KB. Never drop a ✅ row or its
date, or a term somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-25 | 🐞 | Plan review: `cargo test -p sf-conform -- --ignored story` selects one test (`channels.rs`'s story music). The name filter misses all nine ignored tests in `tests/story.rs`, whose names lack "story"; `--test story -- --ignored` runs them. |
| 2026-10-25 | 📌 | Plan review: `level/mod.rs` `scheduled_blit` decodes BLTSIZE as `s >> 6`, a copy the draft and `measure.sh` (pattern `size >> 6`) do not see. |
| 2026-10-25 | 🔄 | Owner added a level 1-1 smoke test beyond the draft: `complete-w1-1.toml` joins both tasks' sf-conform run (DESIGN §4). The full `sf-conform all` stays out. |
| 2026-10-25 | 🔄 | Draft 4 fences off `level/bobs.rs` yet deletes `objects::blit`, which its `masks` calls twice. Owner: switch those two calls to `Blit::run`, nothing else there (DESIGN §7). |
| 2026-10-25 | 📌 | `sf-oracle`'s `AmigaMemory::hash` uses its private `hash_bytes`, which also checksums trace files; once the type lives in sf-core it cannot keep an inherent `hash`. The digest must stay bit-identical for cached traces. |
| 2026-10-25 | 📌 | `measure.sh` baseline on this branch's base matched NORTH-STAR: blitter 4, BLTSIZE 9, chip memory 3, is_paula 2, `PASS sf-paula (838 lines)`. |
