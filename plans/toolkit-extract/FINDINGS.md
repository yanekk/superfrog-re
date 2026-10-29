# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** once this file is over 60 rows or 15 KB. Never drop a ✅ row or its
date, or a term somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-29 | 📌 | T08: sf-e2e `recorder_bar` (bar under a resized/widescreen window) fails on the unchanged base under parallel-build load; `controls.rs` rebinding failed once, passed once. Window-timing flakes, not the move. |
| 2026-10-29 | 📌 | T05: vAmiga rebuilt under `~/.cache/amiga-toolkit/vamiga` in 59 s. After it frog_cases 36 took 816 s, alone 47 s, beam 5 s, story `--ignored` 9 took 309 s: no visible cache-refill cost (frog_cases was 17 min at T02). |
| 2026-10-29 | 📌 | T05: Kickstart 1.3 with df0 empty draws the insert-disk hand at frame 207 and holds it (6 colours, 320 wide); `amiga-oracle/tests/boot.rs` pins it. |
| 2026-10-29 | 📌 | T07: a smoke capture's `clock.txt` and `pacing.txt` vary with machine load (one loaded run dropped 332 frames, reruns 0); byte-compare frames, `inputs.txt`, `audio.wav`, `audio-frames.txt` only. sf-app no longer depends on wgpu, cpal or gilrs directly. |
| 2026-10-29 | 📌 | T04: the toolkit purity test scans `tests/` as well as `src/`, so a synthetic file literal over 16 integers fails it; build it from pieces. `hunk::Blocks::peek` gives the raw type long for a loader that stops before reading a block (T05). |
| 2026-10-29 | 🐞 | T10 ran `cargo fmt` over all of sf-core and amiga-chips: 75 files changed by formatting only. Review reverted them. Format only the files a task changes (`rustfmt <file>`), never the whole crate, or parallel tasks conflict. |
| 2026-10-29 | 📌 | T03: the engine purity test runs `cargo tree -p` on a toolkit crate, a non-member, so it sees the whole engine's features: png turns on miniz_oxide's `simd-adler32`. A toolkit crate pulling a shared dependency may show more there than in its own workspace. |
| 2026-10-29 | 📌 | T02: `cargo fmt --all` in `toolkit/` rewrites the T01 crates (no rustfmt.toml, code is at width 120); format only the crate you touch. Per-task runs after the move: frog_cases 36 (17 min), alone 5, beam 2, story `--ignored` 9, green. |
| 2026-10-29 | 📌 | T01 baseline, `sf-conform -- all` on b7a2f53: all 216 cases ok, none red. Per-case lines with tolerated counts in `baseline-T01.txt`; T11 compares against it. Took ~80 min in four parallel slices. |
| 2026-10-29 | 📌 | T01: the toolkit test line takes 14 s from a cold `toolkit/target`. Engine regression after the move: frog_cases 36, alone 5, beam 2, story `--ignored` 9, purity, paula_vs_oracle, sf-core, sf-app, sf-e2e smoke all green. |
| 2026-10-29 | 📌 | T01: the name check found its own test literal; the names in `toolkit-checks` are built from pieces. A later crate quoting a game name in a test must do the same. |
| 2026-10-28 | 📌 | Planning: `sf-app/build.rs` `GAME` hashes sf-core, sf-assets, sf-paula, sf-disk only; amiga-chips and amiga-clock changes do not change the save-state build tag. T01 adds them (DESIGN §3.3). |
| 2026-10-28 | 📌 | Planning: `tests/purity.rs` reads sources from `engine/crates/<name>` and panics on a missing directory; `tools/architecture.py` `DOMAIN` is `engine/crates/`, `engine/conform/`. Both must learn `toolkit/` before any crate moves. |
| 2026-10-28 | 📌 | Planning: `sf-oracle` trace keys append ` boot {:?}` for any boot slot but `DiskSlot::D1`; patch text feeds the vAmiga build hash and the oracle `BUILD`. Changing either invalidates every cached trace and sync point. |
| 2026-10-28 | 📌 | Planning: no oracle test boots without a disk; `Amiga::power_on` always inserts `cfg.boot`. The shim has `sf_amiga_eject_disk`, unbound in `ffi.rs`. |
| 2026-10-28 | 📌 | Planning: `sf-conform/src/bug.rs::read_zip` and `sf-app/src/recorder.rs::read_zip` duplicate each other because sf-app may not depend on sf-conform. |
| 2026-10-28 | 📌 | Planning: the app binary is `superfrog` (`sf-app/Cargo.toml` `[[bin]]`); sf-e2e builds it with `cargo build -p sf-app --bin superfrog`. |
