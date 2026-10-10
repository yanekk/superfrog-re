# T01 — rust-workspace

**Phase:** 1 · **Depends on:** — · **Weight:** light

## Goal

Install Rust and lay down the Cargo workspace every later task builds in, with the two guards that
protect the design from the start: the purity test (DESIGN §3.1) and the no-disk-data check (§2.3).
After this, `cargo test -q --workspace` in `engine/` is part of the test block and passes.

## Design sections this implements

DESIGN §2.3, §3.1, §3.2, §5 (test command, disk location).

## Files

- `tools/setup/install-rust.sh` — rustup via brew, stable toolchain, idempotent.
- `engine/Cargo.toml` (workspace), `engine/rust-toolchain.toml` (pin stable version measured here).
- `engine/crates/{sf-disk,sf-assets,sf-core,sf-paula,sf-app,sf-oracle,sf-conform,sf-e2e}/` with
  `Cargo.toml` and an empty `lib.rs`/`main.rs` each.
- `engine/crates/sf-conform/tests/purity.rs`
- `engine/crates/sf-conform/src/disks.rs` — test-side disk locator.
- `tools/check_no_disk_data.py`, `tools/test_check_no_disk_data.py` (runs it in the Python suite), and
  `tools/__init__.py` if needed: `unittest discover -s .` only descends into packages, so check the test
  is actually collected (the run must say `Ran 1+ tests`, not `NO TESTS RAN`).
- `.gitignore` — add `engine/target/`.
- `README.md` — a short "Rust engine" section pointing at `plans/rust-engine/`.

## Interface

```rust
// sf-conform::disks
pub fn disk_dir() -> Result<PathBuf, String>   // SUPERFROG_DISKS, else <git common dir>/..; Err names both places looked
```

```
tools/check_no_disk_data.py [--disks DIR]   # exit 1 listing file:offset for any 128-byte run shared with a decompressed chunk
```

The check indexes every decompressed chunk (produced into a temp dir with `tools/atn.py`'s carver and decompressor; `tools/unpack.py` writes only into `build/`) at 64-byte
stride and scans each tracked file at 1-byte stride, which finds any copied run of 127 bytes or more.

## Tests

- [ ] purity: a fixture source using `std::fs`, `Instant`, `HashMap`, `f32`, or a 17-element integer
      array in a pure crate is flagged; the real pure crates pass.
- [ ] purity: `sf-paula/src/resample.rs` alone is exempt from the float and array rules (DESIGN §3.1); the same code in any other pure file is flagged.
- [ ] purity: `sf-app`'s dependency tree contains no `sf-oracle`, `sf-conform` or `sf-e2e`.
- [ ] purity: a pure crate depending on a crate outside the allowlist is flagged.
- [ ] `disk_dir()` honours `SUPERFROG_DISKS`, falls back to the main checkout, and errs clearly.
- [ ] the no-disk-data check flags a temp file containing 200 bytes copied from a chunk and passes
      the real tree.

## Done when

- [ ] `rustc --version` works on this Mac and the version is pinned in `rust-toolchain.toml`.
- [ ] Both test-block lines pass from a fresh worktree after setup.
- [ ] The purity test fails on each forbidden pattern and the failure message says "move the code, do not relax the test".

## Outside actions

- Install Rust — `worker`
- Download crates during `cargo build` — `worker`
