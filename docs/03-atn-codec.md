# 03 — The `ATN!` codec

`ATN!` is a custom Team 17 cruncher. It is **not** recognised by `ancient`
(Teemu Suutari's decompression library) or other public tools, so there is no
off-the-shelf decoder.

## How we decode it

Two decoders, one the reference for the other (DESIGN §2.4 of `plans/rust-engine`):

- **The product decoder** is a hand port to Rust, `engine/crates/sf-disk/src/atn.rs`
  (`sf_disk::decompress`). The shipped game contains no emulator.
- **The reference** is `tools/atn.py`, which runs **the game's own routine** on a
  real 68000 core (`machine68k`, Musashi) and is exact by construction. The
  Rust port is tested byte-for-byte against it (`sf-disk/tests/atn_reference.rs`).

The routine lives in the disk-1 stage-1.5 loader, raw code at file offset
`0x400..0x1600`, entry at `+0x2DA` (`atn_decompress` in `re/symbols.toml`). It is
identified by its first real instruction: `cmpi.l #$41544E21,(a0)+` ("ATN!").
`tools/atn.py` loads that segment at `0x1000`, places a carved chunk at
`0x400000`, points `a0`→source and `a1`→dest, and calls `+0x2DA`; a `STOP` at
the fake return address halts the CPU on `rts`. Success is the routine's own
return code, `d0 = -1`.

Result: **103 / 106 chunks decode as carved, and the Rust port is identical on
all 103.**

### The three stragglers: the trailer crosses a sector

`d1_c16`, `d2_c39` and `d4_c16` fail as carved because their 46-byte trailer
(below) starts at `comp_size` and runs past the sector the carver stops at: the
carver ends a chunk at `12 + comp_size` rounded up to 512, which leaves only
38, 44 and 30 bytes for the trailer. The rest is in the next disk sector. With
that sector appended, **all three decode** in both the emulator and the Rust
port, identically, to exactly `decomp_size` bytes:

| Chunk | Carved, Rust says | With next sector |
|---|---|---|
| `d1_c16` | `Truncated` | 40 514 bytes |
| `d2_c39` | `Truncated` | 70 464 bytes |
| `d4_c16` | `Truncated` | 53 588 bytes, a **T7MP level** (300 × 84 `BODY`) |

**Checked against the game (T07, `sf-oracle/tests/exe_map_claims.rs`):**

- The game's `file_table` (docs/08) gives each file's sector count, and for all three it reads
  past the carver's last sector. They are W1's music `L1MU` (`d1_c16`), the bonus level's loading
  picture `LBLP` (`d2_c39`) and level W6-3 `L6MA3` (`d4_c16`).
- After the World 1 load, `d1_c16` sits in RAM at `$73EF6` and equals the Rust output with the next
  sector appended, byte for byte. The only exception is its first `$C0` bytes: the module's sample
  table, whose offsets the music replayer turns into absolute pointers.
- `d2_c39` and `d4_c16` load only after the bonus level and W6-2 are reached. No scripted run gets
  that far yet, so they are not compared in RAM. T21's whole-game run reaches both.

The carvers (`tools/atn.py`, `sf_disk::carve`) still stop at the old boundary. The game's sector
counts in `file_table` are the true file lengths (T08/T09).

## Algorithm (as ported)

All integers big-endian. Chunk = `"ATN!" | decomp_size | comp_size | stream…`,
trailer at `chunk + comp_size`, 46 bytes:

| Trailer bytes | Use |
|---|---|
| 0..12 | copied over the 12-byte header (longs in reverse order: `0..4`→`8..12`, `4..8`→`4..8`, `8..12`→`0..4`); the stream's last-read bytes |
| 12..16 | `d2`: the first literal run's length |
| 16..18 | `d3`: bit buffer (low byte). If bit 15 is clear, the stream starts one byte lower (`comp_size - 1`) |
| 18..46 | table `T`, 28 bytes: words `T[0..8]` short offset bases, `T[8..16]` long offset bases (signed), bytes `T[16..28]` offset bit counts |

Source `a3` reads backward from the stream end down to the chunk start; output
`a4` writes backward from `decomp_size` down to 0.

- **Bits:** `add.b d3,d3` gives the top bit; when the byte becomes 0, refill
  `d3 = *--a3` and `addx.b d3,d3` (the old carry becomes the new low bit, the
  new byte's top bit is the bit returned). A sentinel-bit scheme, MSB first.
- **Loop:** copy `d2` literal bytes (`*--a4 = *--a3`). Stop when `a4` reaches
  the start; success needs `a3` exactly at the chunk start too. Otherwise:
- **Match length / class** (unary prefix): `0`→2 (class 0), `10`→3 (1),
  `110`→4 (2), `1110`→5 (3), `11110`+3 bits→6..13 (3), `11111`+a whole byte read
  from the source→that byte, 0 meaning 256 (3).
- **Next literal run `d2`:** `0` → `bits(L[class])`; `11` →
  `bits(L[class+8]) + B[class]`; `10` → `bits(L[class+4]) + 2`, with
  `B = 06 0a 0a 12` (`+0x42A`) and `L = 01 01 01 01 02 03 03 04 04 05 07 0e`
  (`+0x42E`).
- **Match offset:** `0` → base 0, `n = T[16+class]`; `11` → base
  `T.w[8+2·class]`, `n = T[16+class+8]`; `10` → base `T.w[2·class]`,
  `n = T[16+class+4]`. `extra = bits(n)` (n = 0 reads 256 bits, keeping the low 32).
- **Copy:** `a2 = a4 + 1 + base + extra`; `len` times `*--a4 = *--a2`
  (byte by byte, so overlapping runs repeat).

The Rust port refuses what the original would do silently: writing below the
output start (it decodes on without writing to report the stream's true length,
`LengthMismatch`), a match reading outside the output (`Overrun`), and reading
before the chunk start (`LengthMismatch`).

## Bonus: the stage-2 *backward bit-LZ* (different codec)

The boot self-decompressor at load address `0x40024` is a distinct, simpler
codec (3-long header `[srclen, destlen, checksum]`, 32-bit bit buffer consumed
LSB-first, XOR checksum). It was hand-ported and **verified** (checksum XORs to
0; output begins with valid m68k `bra.w`). It only unpacks the loader, not game
data, but it was the first foothold into the boot chain.
