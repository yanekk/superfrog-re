# T03 — amiga-disk

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Create `amiga-disk` with the parts of `sf-disk` any Amiga game needs: the zip reader and writer
(the owner's disks come zipped; the recorder writes bug zips), SHA-1, and the ADF image size. What
names Superfrog's four disks, its packer and its carver stay in `sf-disk`. The oracle's build script
and the recorder depend on this crate.

## Design sections this implements

DESIGN §2.2 row `amiga-disk` (first), §2.3, §3.1, §3.3.

## Files

- New: `toolkit/crates/amiga-disk/` with `src/zip.rs`, `src/sha1.rs`, `src/lib.rs` (`ADF_SIZE`),
  `tests/common/zipwrite.rs` (the helper `zip.rs`'s unit tests include by path).
- `engine/crates/sf-disk/src/{lib,adf,zip,sha1}.rs` (re-exports; `adf.rs` keeps `DiskSlot`, `Adf`,
  `DiskSet`), `sf-disk/Cargo.toml`.
- `sf-oracle/Cargo.toml` build-dependency (`amiga-disk` for `sha1_hex`; its value must not change).
- Purity lists, `sf-app/build.rs` `GAME`, `docs/architecture/disks-and-assets.md` `owns` split,
  moved-path table.

## Interface

```rust
// amiga_disk (pure)
pub const ADF_SIZE: usize = 901_120;
pub mod zip { pub fn read_entries(..); pub fn write_stored(..); /* as today */ }
pub mod sha1 { pub fn sha1(..) -> [u8; 20]; pub fn sha1_hex(..) -> String; }
```

## Tests

- [ ] `zip.rs` and `sha1.rs` unit tests pass in the toolkit, with no disk.
- [ ] `sf-disk`'s `real_disks.rs`, `atn_reference.rs`, `atn_fuzz.rs` stay in sf-disk and pass.
- [ ] `sha1_hex` of the patch set equals `tools/test_oracle_sh.py`'s Python hash (build.rs still
      finds the existing vAmiga build).
- [ ] A bug zip written before the move reads back byte-equal through `read_entries`.

## Done when

- [ ] `amiga-disk` holds zip, SHA-1 and `ADF_SIZE`; `sf-disk` keeps every Superfrog part.
- [ ] Test block green; DESIGN §4's per-task runs green.
