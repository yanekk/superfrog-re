# T04 — asset-readers

**Phase:** 2 · **Depends on:** T03 · **Weight:** medium

## Goal

Move the standard Amiga program loader (hunk executables) and the IFF picture, palette and animation
readers out of `sf-assets` into `amiga-disk`, without their ties to Superfrog's names. Game two's
program is a hunk executable and its art is likely IFF; these readers are proven on Superfrog today,
so moving them is cheap (owner, 2026-10-28).

## Design sections this implements

DESIGN §2.2 row `amiga-disk` (modules), §7 (the readers decision).

## Files

- New: `toolkit/crates/amiga-disk/src/{hunk,iff,palette,anim}.rs` (names the worker's), and an error
  type of its own.
- `engine/crates/sf-assets/src/sound.rs` (`first_hunk` reads through `amiga_disk::hunk`).
- `engine/crates/sf-assets/src/{exe,picture,palette,anim,error}.rs`, `sf-assets/Cargo.toml`.
- `docs/architecture/disks-and-assets.md` `owns`, moved-path table, `sf-app/build.rs` `GAME` if needed.

## Interface

The readers take bytes and return the parsed form; where today they carry `ChunkId`, `World` or a
T7MP `Tag` (for error messages and picture placement), the toolkit form takes no such type and
sf-assets wraps it: its error carries the chunk, the toolkit's carries the byte offset and what was
wrong.

`amiga_disk::hunk` is the toolkit's one hunk reader (owner, 2026-10-28, plan review): it exposes
the block walk (header, each block's type, hunk sizes and memory flags, contents, RELOC32 lists)
that `parse` is built on, so T05's `Cpu68k::load_exe` and sf-assets' `sound.rs::first_hunk` read
through it instead of walking the longs themselves.

```rust
// amiga_disk::hunk
pub struct Hunk { pub kind: HunkKind, pub data: Vec<u8>, pub relocs: .. }
pub fn parse(bytes: &[u8]) -> Result<Vec<Hunk>, ReadError>;
// amiga_disk::iff
pub fn ilbm(bytes: &[u8]) -> Result<Bitmap, ReadError>;  // plus Palette, Anim as today's
```

Exact shapes follow today's functions; a reader whose output changes is a bug.

## Tests

- [ ] Every existing sf-assets test passes unchanged (they decode the real disks: the readers'
      output is unchanged on every Superfrog file).
- [ ] Toolkit unit tests on synthetic bytes: a minimal hunk file (code, data, reloc32, end); a
      2-plane ILBM with BODY uncompressed and ByteRun1; a truncated file reports its offset.
- [ ] `pic_progress_map` and any Superfrog-only placement stay in sf-assets.

## Done when

- [ ] The four readers are in `amiga-disk`, named nothing of the game, with synthetic tests.
- [ ] `sf-assets` decodes every disk file as before; test block and DESIGN §4 runs green.
