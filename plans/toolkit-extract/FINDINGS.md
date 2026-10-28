# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** once this file is over 60 rows or 15 KB. Never drop a ✅ row or its
date, or a term somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-28 | 📌 | Planning: `sf-app/build.rs` `GAME` hashes sf-core, sf-assets, sf-paula, sf-disk only; amiga-chips and amiga-clock changes do not change the save-state build tag. T01 adds them (DESIGN §3.3). |
| 2026-10-28 | 📌 | Planning: `tests/purity.rs` reads sources from `engine/crates/<name>` and panics on a missing directory; `tools/architecture.py` `DOMAIN` is `engine/crates/`, `engine/conform/`. Both must learn `toolkit/` before any crate moves. |
| 2026-10-28 | 📌 | Planning: `sf-oracle` trace keys print `DiskSlot::D1` with `{:?}`; patch text feeds the vAmiga build hash and the oracle `BUILD`. Changing either invalidates every cached trace and sync point. |
| 2026-10-28 | 📌 | Planning: no oracle test boots without a disk; `Amiga::power_on` always inserts `cfg.boot`. The shim has `sf_amiga_eject_disk`, unbound in `ffi.rs`. |
| 2026-10-28 | 📌 | Planning: `sf-conform/src/bug.rs::read_zip` and `sf-app/src/recorder.rs::read_zip` duplicate each other because sf-app may not depend on sf-conform. |
| 2026-10-28 | 📌 | Planning: the app binary is `superfrog` (`sf-app/Cargo.toml` `[[bin]]`); sf-e2e builds it with `cargo build -p sf-app --bin superfrog`. |
