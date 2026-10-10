# T03 — disk-import

**Phase:** 1 · **Depends on:** T01 · **Weight:** light

## Goal

The first stage of import in Rust: take whatever files the player offers, find the four Superfrog
ADFs (zipped or raw), identify each by SHA-1, and carve each disk into its `ATN!` chunks. Every
failure produces a message naming the disk slot. Pure: it receives bytes, not paths.

## Design sections this implements

DESIGN §2.2, §3.1, §3.2 (`sf-disk`).

## Files

- `engine/crates/sf-disk/src/{lib.rs,adf.rs,zip.rs,identify.rs,carve.rs}`
- `engine/crates/sf-disk/tests/real_disks.rs` (uses `sf-conform::disks` as a dev-dependency)
- `docs/01-disk-format.md` — add the four ADF SHA-1s.

## Interface

```rust
pub enum DiskSlot { D1, D2, D3, D4 }
pub struct Offered { pub name: String, pub bytes: Vec<u8> }        // a file the shell read
pub struct DiskSet { pub disks: [Adf; 4] }
pub enum ImportError { Missing(DiskSlot), Unknown { name: String, sha1: String }, Damaged { name: String, why: String }, Duplicate(DiskSlot) }
pub fn identify(offered: &[Offered]) -> Result<DiskSet, Vec<ImportError>>
pub struct Chunk { pub disk: DiskSlot, pub index: u16, pub offset: u32, pub decomp_size: u32, pub comp_size: u32, pub raw: Vec<u8> }
pub fn carve(adf: &Adf) -> Vec<Chunk>                               // same chunks, same order as tools/atn.py
```

The SHA-1s live as constants in `identify.rs`. A digest is not disk data (it cannot reproduce it), so
this is allowed by DESIGN §2.3.

## Tests

- [ ] the four real zips identify; the same four as raw ADFs identify.
- [ ] a zip holding two ADFs, a zip with junk files beside the ADF, a nested folder name: handled.
- [ ] one disk missing → `Missing(slot)`; a truncated ADF → `Damaged`; a flipped byte → `Unknown` with its SHA-1; the same disk twice → `Duplicate`.
- [ ] a zip that is not a zip → `Damaged` naming the file, no panic (also fuzzed with random bytes).
- [ ] `carve` gives 106 chunks with offsets and sizes equal to `docs/05-chunk-inventory.txt`.

## Done when

- [ ] All tests pass in the test block.
- [ ] Chunk inventory from Rust equals the Python carver's, checked by a test.
- [ ] Every `ImportError` variant has a human message naming the slot or file.
