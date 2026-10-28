# T01 — toolkit-workspace

**Phase:** 1 · **Depends on:** — · **Weight:** heavy

## Goal

Record M1's baseline, then give the toolkit its home. Before any change, run `sf-conform -- all` on
the task's base and record every case's verdict, so every later red is compared with a measured
starting point (READINESS: no full run since the keyboard merge). Then create the `toolkit/`
workspace, move the three crates that already pass M4 (`amiga-clock`, `amiga-chips`, `sf-paula`
renamed `amiga-paula`) and `Picture` into it, and build the checks every later move lands in: the
toolkit's purity test, the name check, the architecture docs' moved-path table, `measure.sh` and the
test line.

## Design sections this implements

DESIGN §2.1, §2.2 (rows `amiga-clock`, `amiga-chips`, `amiga-paula`), §2.3, §2.4, §3.1, §3.3, §3.4,
§3.5, §4, §5.

## Files

- New: `toolkit/Cargo.toml`, `toolkit/rust-toolchain.toml`, `toolkit/Cargo.lock`,
  `toolkit/.gitignore` (`target/`), `toolkit/crates/{amiga-clock,amiga-chips,amiga-paula}/` (by
  `git mv`), the toolkit purity test and name check (in a small test-only crate
  `toolkit/crates/toolkit-checks/` or under an existing crate's `tests/`, worker's choice).
- `engine/Cargo.toml` (profiles renamed, comment), `engine/Cargo.lock`, every engine crate manifest
  that names `sf-paula`, `amiga-chips` or `amiga-clock`, and every `use sf_paula` (about 80 sites in
  sf-core, sf-conform, sf-app, sf-e2e, sf-oracle).
- `engine/crates/sf-core/src/render/view.rs` (`Picture`, `Split`, `to_rgb12` re-exported from
  `amiga_chips`), `sf-core`'s tests (the original-data copper and blit tests moved here).
- `engine/crates/sf-conform/tests/purity.rs`, `engine/crates/sf-app/build.rs`,
  `engine/crates/sf-oracle/shim/shim.cpp` (comment only).
- `tools/architecture.py`, `tools/test_architecture_docs.py`, `docs/architecture/*.md` front matter
  `owns` only, `plans/amiga-toolkit-prep/measure.sh`, `.pir/settings.json`.

## Interface

```toml
# toolkit/Cargo.toml
[workspace]
resolver = "3"
members = ["crates/*"]
[workspace.package]
version = "0.1.0"
edition = "2024"
publish = false
[profile.dev.package.amiga-clock]   # and amiga-chips, amiga-paula: opt-level = 2, as in engine/
```

```rust
// amiga_chips
pub struct Picture { pub left: i32, pub top: u16, pub width: u16, pub height: u16,
                     pub pixels: Vec<u8>, pub palettes: Vec<[u16; 32]>, pub splits: Vec<Split> }
pub struct Split { pub y: u16, pub x: u16, pub index: u8, pub colour: u16 }
pub fn to_rgb12(p: &Picture) -> Vec<u16>;
// sf_core::render::view keeps View, black, widened, narrowed, DEFAULT_WINDOW; Picture methods
// that use View become free functions or an extension trait in sf-core.

// amiga_paula: AudioDma::booted(dmacon: u16), Paula::booted(host_rate: u32, dmacon: u16);
// sf-core passes 0x8200 through one named constant.
```

`.pir/settings.json` `test` gains `"if [ -d toolkit ]; then cd toolkit && cargo test -q --workspace; fi"`,
identical to DESIGN's block.

## Tests

- [ ] Baseline first: `cargo run -q --release -p sf-conform -- all` on the unchanged base, every
      case's verdict in a FINDINGS row (run in parts if a time limit cuts it).
- [ ] `Picture` serialises to the same bincode bytes before and after the move.
- [ ] Name check: fails on a synthetic file containing `Superfrog`, `sf_core`, `Team 17`; passes on
      a `learned on:` line; passes on the real `toolkit/` tree.
- [ ] Toolkit purity: the pure toolkit crates' tree is within the allowlist; a forbidden import in a
      pure crate's source is caught (as the engine test's own self-tests do).
- [ ] Engine purity still passes, with the toolkit crates on `ALLOWED` and scanned at their paths.
- [ ] `test_architecture_docs.py`: every toolkit file owned once; a cited old path resolves through
      the moved-path table; an unknown old path still fails.
- [ ] `Paula::booted(rate, 0x8200)` gives the same mixer state as today's `booted(rate)`.
- [ ] The copper and blit tests that used the game's data pass in sf-core; the toolkit's copies use
      synthetic data.
- [ ] `measure.sh` prints the same M2 counts as before the move, and `PASS` for the three toolkit
      crates.

## Done when

- [ ] The baseline verdicts are in FINDINGS; the test block (three lines) is green, with the
      toolkit line's run time recorded.
- [ ] `toolkit/` holds the three crates and `Picture`; `cargo tree` from `toolkit/` shows no `sf-*`
      crate; the name check, both purity tests and the docs check pass.
- [ ] DESIGN §4's per-task sf-conform runs are green, and the build tag hashes the toolkit crates.
