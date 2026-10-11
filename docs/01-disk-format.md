# 01 — Disk format

Each `.adf` is a standard 901,120-byte Amiga double-density image (880 KB,
2 sides × 80 cyl × 11 sectors × 512 B). But they are **not** AmigaDOS disks:

| Disk | First bytes | Notes |
|------|-------------|-------|
| 1 | `44 4F 53 00` `DOS\0` | bootblock present, **no** valid root block |
| 2 | `41 54 4E 21` `ATN!` | raw data from sector 0, no bootblock |
| 3 | `44 4F 53 00` `DOS\0` | bootable; bootblock + trackloader |
| 4 | `41 54 4E 21` `ATN!` | raw data from sector 0 |

`xdftool info` reports "Invalid Root Block" / "Invalid Boot Block" — confirming a
custom **trackloader** disk (typical Team 17). The DOS bootblock on disks 1 & 3
exists only so the disk boots; its code is a custom loader (see
[`02-boot-chain.md`](02-boot-chain.md)).

## Accepted images (SHA-1)

The Rust importer (`engine/crates/sf-disk/src/identify.rs`) accepts exactly these four ADFs,
the "[cr CSL]" set, by SHA-1 of the 901,120-byte image (zipped or raw):

| Disk | SHA-1 of the `.adf` |
|------|---------------------|
| 1 | `14f177b59787c3c8a667e6c9ac78ce02874f59f7` |
| 2 | `c5528eac0c09fed318cb25fa8d75069b780e44f0` |
| 3 | `99ac304d5665991d7f9b2e792e7e8d2762ff5aa4` |
| 4 | `f5ef23973fb5d5822224738aceda0091e795382c` |

## The `ATN!` container

All game data is a back-to-back sequence of **sector-aligned `ATN!` chunks**:

```c
struct ATNChunk {          // integers are big-endian
    char magic[4];         // "ATN!"  (0x41544E21)
    u32  decomp_size;      // decompressed size
    u32  comp_size;        // size of the compressed region *including* this 12-byte header
    u8   data[...];        // compressed stream, followed by a ~46-byte trailer
    u8   pad[...];         // zero padding up to the next 512-byte sector
};
```

- The next chunk starts at `ceil(offset + comp_size, 512)` — verified against the
  measured inter-chunk gaps on every disk.
- The **trailer** appended after the compressed stream (12 bytes + a long + a word
  + seven longs = ~46 bytes) holds initialisation state for the decompressor
  (bit-reader priming and an offset-base table). It lives inside the sector
  padding, so carving must keep the whole sector-aligned chunk, not just
  `comp_size` bytes.
- On disks 1 & 3, a handful of `ATN!` byte sequences appear inside the loader
  code at non-sector offsets; the carver ignores them (not 512-aligned / fail
  size sanity).

**Totals:** 106 chunks across the four disks, ~7.4 MB decompressed.
See [`05-chunk-inventory.txt`](05-chunk-inventory.txt).
