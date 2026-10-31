# 08 — Packers

Forty-seven of the 61 files are packed, by two packers. Every packed file unpacks with
`venv/bin/python games/forest-dumb/tools/unpack.py --all`, into
`games/forest-dumb/build/unpacked/disk<N>/<name>` (git-ignored). Later tools read the game's data from
there. It needs `fd-tools extract` to have run, because the unpacking code comes from the disks.

## `ESL!`: 46 files, all on disk 2

All 48 of disk 2's data files except `_BALLA` and `_END`. The format is PowerPacker's with the `PP20`
magic replaced:

| Bytes | Holds |
|---|---|
| 0..4 | `ESL!` |
| 4..8 | offset widths, `09 09 09 09` in every file (PowerPacker's "fast" setting) |
| 8..end-4 | the bit stream, read backwards from the end, as big-endian longwords low bit first; a value's first bit read is its top bit |
| end-4..end-1 | unpacked size, 24-bit big-endian |
| end-1 | bits to skip before the stream starts |

Evidence it is PowerPacker: that layout, every file a multiple of 4 bytes, and the routine's token
structure. Every file yields the size in its tail and ends in the stream's first byte, with at most one
bit unread (39 files leave one); `unpack.py` refuses a file that does not.

The main program loads a file at `$437c8`, tests for the magic with `cmpi.l #'ESL!',$437c8` and calls
its unpacker at hunk 0 offset `$1d16`. Hunk 0 starts at file offset `$30`. `unpack.py` runs that
routine, from `$1d28`, under Musashi. `test_unpack.py` holds a second decoder written from the format;
the two agree byte for byte on all 46 files.

Why the game's routine: the crates.io PowerPacker decoders are `format198x-commodore-amiga-powerpacker`
(GPL) and `newtua-amiga` (LGPL), both copyleft, which DESIGN §5 rules out (checked 2026-10-31), and no
PyPI package turned up under `powerpacker` or `pp20`. A Rust port later has the routine and the test
decoder to check against.

## `S404`: `BillClintonToDupek` on disk 1

The crack's intro, a self-unpacking executable (StoneCracker 4; magic at file offset `$200`). `unpack.py`
runs the stub's own loop under Musashi and stops before it allocates memory. The output (172 988 bytes,
the size in the S404 header) is the stub's internal hunk list, not a loadable executable: a hunk count
minus one (6), sizes, data and relocations, ending `ffff`. The crack's text reads clearly in it.

## Not packed: 14 files

No signature at offset 0, nor in the first 4 KiB of an executable. Signatures checked are the
`SIGNATURES` table in `unpack.py`: PowerPacker, RNC 1 and 2, Imploder and clones, LZX, XPK, CrunchMania,
StoneCracker, Turbo Packer, Pack-Ice. Entropy below is bits per byte; three files needed more than the
scan:

- `_BALLA` (7.15): a Protracker module, `M.K.` at offset 1080, title `_balla`. Sample data accounts for
  the entropy.
- `Kwach4`: an IFF ILBM picture. Its `BMHD` says ByteRun1, the format's own compression, which `Show`
  handles; not a packer in this sense.
- `FOREST_DUMB_FOREVER`, `Show`, `Installer`: hunk executables with readable library names and strings.

## Every file

| File | Bytes | First bytes | Entropy | Packer | Unpacked bytes | Unpacked SHA-1 |
|---|---|---|---|---|---|---|
| `disk1/BillClintonToDupek` | 62896 | `000003f300000000` | 7.91 | StoneCracker 4 | 172988 | 063e70bea954eee66a6b72c5df6d45b59633a6d1 |
| `disk1/Disk.info` | 388 | `e310000100000000` | 3.73 | none |  |  |
| `disk1/FOREST_DUMB_FOREVER` | 521640 | `000003f300000000` | 2.96 | none |  |  |
| `disk1/FOREVER!` | 20 | `464f524553545f44` | 3.55 | none |  |  |
| `disk1/FOREVER!.info` | 932 | `e310000100000000` | 4.01 | none |  |  |
| `disk1/Installer` | 4736 | `000003f300000000` | 5.30 | none |  |  |
| `disk1/Instaluj` | 32 | `496e7374616c6c65` | 4.04 | none |  |  |
| `disk1/Instaluj.info` | 810 | `e310000100000000` | 3.83 | none |  |  |
| `disk1/Kwach4` | 46000 | `464f524d0000b3a8` | 4.70 | none |  |  |
| `disk1/Show` | 9508 | `000003f300000000` | 6.25 | none |  |  |
| `disk1/f.s` | 96 | `636f70790a64656c` | 4.34 | none |  |  |
| `disk1/s/startup-sequence` | 54 | `53686f77202d6f20` | 4.72 | none |  |  |
| `disk2/!EAGLE_SOFT!` | 400 | `0a20202020202020` | 3.10 | none |  |  |
| `disk2/StworkiNew.mask` | 4076 | `45534c2109090909` | 4.36 | ESL! (PowerPacker format) | 44800 | c6804919b14aaf9d56ad7b231b791e60bf5b9fbd |
| `disk2/StworkiNew.rawm` | 11476 | `45534c2109090909` | 7.14 | ESL! (PowerPacker format) | 44800 | 63b9268af97c5dcca1646aff3d3a77cbbe826c6e |
| `disk2/_BALLA` | 158840 | `5f62616c6c610000` | 7.15 | none |  |  |
| `disk2/_CZOLOWKA` | 13964 | `45534c2109090909` | 7.60 | ESL! (PowerPacker format) | 31200 | 1f84cebcc91d29a3629726ba29a467d972bbb364 |
| `disk2/_ELEMENTY.MASK` | 816 | `45534c2109090909` | 5.27 | ESL! (PowerPacker format) | 6400 | 3708752877d588385ad80ea3a10d1ff633e9d2b3 |
| `disk2/_ELEMENTY.RAWM` | 3044 | `45534c2109090909` | 7.48 | ESL! (PowerPacker format) | 6400 | 9b77502270b9fe855785b6029f6c9eb22f2f713e |
| `disk2/_END` | 61440 | `0060000000000000` | 2.36 | none |  |  |
| `disk2/_Gen1-1` | 33884 | `45534c2109090909` | 7.57 | ESL! (PowerPacker format) | 52480 | 816af139bf7dfc041418073762fe927a3d4105ca |
| `disk2/_Gen1-1.Mapa` | 460 | `45534c2109090909` | 7.06 | ESL! (PowerPacker format) | 1568 | 8e5e0ab94483cf73372ee3c880b5770402e33a52 |
| `disk2/_Gen2-1` | 34560 | `45534c2109090909` | 7.57 | ESL! (PowerPacker format) | 53600 | dbd11a9ea127d7e3c348839d0ba0183dcedc02af |
| `disk2/_Gen2-1.Mapa` | 380 | `45534c2109090909` | 6.91 | ESL! (PowerPacker format) | 1280 | 225590c3d02074cf34cc84d9b01d3ee043d79f00 |
| `disk2/_Gen3-1` | 36388 | `45534c2109090909` | 7.60 | ESL! (PowerPacker format) | 48960 | bcf9cfbfd045275b229533a356fdd38545e89065 |
| `disk2/_Gen3-1.Mapa` | 140 | `45534c2109090909` | 5.87 | ESL! (PowerPacker format) | 416 | 08c45d3cf9228231c2d1a701dffa06b12ce84d2f |
| `disk2/_Gen4-1` | 31540 | `45534c2109090909` | 7.58 | ESL! (PowerPacker format) | 43200 | 94d0cc543a80066af36b75c8b3295ac6eb775464 |
| `disk2/_Gen4-1.Mapa` | 168 | `45534c2109090909` | 5.94 | ESL! (PowerPacker format) | 640 | 77673e8b558e7943081b53539f7aa61feb726392 |
| `disk2/_Gen5-1` | 24984 | `45534c2109090909` | 7.57 | ESL! (PowerPacker format) | 35840 | 6d6d8e16b4cb4ef1d9a42778aa2845c0d6cbe16b |
| `disk2/_Gen5-1.Mapa` | 160 | `45534c2109090909` | 5.97 | ESL! (PowerPacker format) | 512 | a91036e32cb3528592c5ca616a08e06e618d46c3 |
| `disk2/_Gen6-1` | 25044 | `45534c2109090909` | 7.56 | ESL! (PowerPacker format) | 36320 | 48874537b37763905e8d6aa3f39fbb66b2c75f3a |
| `disk2/_Gen6-1.Mapa` | 292 | `45534c2109090909` | 4.17 | ESL! (PowerPacker format) | 2880 | 369cb35ef83053ff4ec8972945c1da78ddaddcdd |
| `disk2/_Mapa1-1` | 21164 | `45534c2109090909` | 7.56 | ESL! (PowerPacker format) | 53760 | 1630f0935b9a227b7bcb12598c44cd92ba9b4a0d |
| `disk2/_Mapa1-1.Mapa` | 3624 | `45534c2109090909` | 7.04 | ESL! (PowerPacker format) | 23040 | 3e027a8626670e3d9b52ab5aa353415d058e92d5 |
| `disk2/_Mapa2-1` | 21176 | `45534c2109090909` | 7.61 | ESL! (PowerPacker format) | 53760 | d4259e7fd6de4890296f2de022b2d98a10ede6e8 |
| `disk2/_Mapa2-1.Mapa` | 4400 | `45534c2109090909` | 7.48 | ESL! (PowerPacker format) | 23040 | b4e7e87578fc10702c3b83bec2d3cc40f6e92f2c |
| `disk2/_Mapa3-1` | 21176 | `45534c2109090909` | 7.66 | ESL! (PowerPacker format) | 53760 | b2b4126a10cb6572bd818b4819f23d9a21e4a2b0 |
| `disk2/_Mapa3-1.Mapa` | 3344 | `45534c2109090909` | 6.89 | ESL! (PowerPacker format) | 23040 | 04b906529fbf24904f9a616d51d497714af774f0 |
| `disk2/_Mapa4-1` | 20904 | `45534c2109090909` | 7.64 | ESL! (PowerPacker format) | 53760 | f96a73d6741f04ac9fdbaa37aedd65e60fb85f83 |
| `disk2/_Mapa4-1.Mapa` | 3312 | `45534c2109090909` | 6.87 | ESL! (PowerPacker format) | 23040 | 148bcef8cd9118eab738a8522a589c7a5d464dac |
| `disk2/_Mapa5-1` | 15624 | `45534c2109090909` | 7.56 | ESL! (PowerPacker format) | 53760 | 6978f662fae4cbb937b861c22ed726ad5392dfbf |
| `disk2/_Mapa5-1.Mapa` | 3204 | `45534c2109090909` | 6.78 | ESL! (PowerPacker format) | 23040 | 05340ba5833963667bab05c6cc86d2ff4bf5b7f6 |
| `disk2/_Mapa6-1` | 18984 | `45534c2109090909` | 7.56 | ESL! (PowerPacker format) | 61440 | 8f39b17d1a25bccc1145f7572e07f1031bcb823d |
| `disk2/_Mapa6-1.Mapa` | 3728 | `45534c2109090909` | 6.33 | ESL! (PowerPacker format) | 30720 | cbde79e7503d185d6c968c5045bb0878b9072d57 |
| `disk2/_MapaBobow1` | 3268 | `45534c2109090909` | 4.70 | ESL! (PowerPacker format) | 38400 | 2d7e22fc30f0914a75410e6336cd0bd90708d730 |
| `disk2/_MapaBobow2` | 3584 | `45534c2109090909` | 5.14 | ESL! (PowerPacker format) | 38400 | 951afadfccf6041693cdd8a01b331f1e66e79354 |
| `disk2/_MapaBobow3` | 3508 | `45534c2109090909` | 5.05 | ESL! (PowerPacker format) | 38400 | dfb1de878827cf03db5c81f6fbf8624f9f83cd45 |
| `disk2/_MapaBobow4` | 3180 | `45534c2109090909` | 4.47 | ESL! (PowerPacker format) | 38400 | cbf8ea2d2b29946a458105fd75aea3addfecf7d3 |
| `disk2/_MapaBobow5` | 3340 | `45534c2109090909` | 4.78 | ESL! (PowerPacker format) | 38400 | c717e7134aa08ca7b5cf62254bf717442c7d20ce |
| `disk2/_MapaBobow6` | 4148 | `45534c2109090909` | 5.95 | ESL! (PowerPacker format) | 38400 | 280fa77888e35f66ec66ded2316b28092c2b29d5 |
| `disk2/_NAPIS` | 7232 | `45534c2109090909` | 7.46 | ESL! (PowerPacker format) | 18720 | 4d1f22f602a57a4cf9b4c60abc38961a65a39712 |
| `disk2/_OPCJE` | 1796 | `45534c2109090909` | 7.46 | ESL! (PowerPacker format) | 4320 | 92105d6771890fe617210de69898e786f6526fa7 |
| `disk2/_Plomien1` | 1088 | `45534c2109090909` | 7.19 | ESL! (PowerPacker format) | 1280 | 50ddf9075bce35b20d514d280cc61e2899b70eca |
| `disk2/_Plomien2` | 836 | `45534c2109090909` | 7.19 | ESL! (PowerPacker format) | 1280 | 422f5d7dfeca597a63e464fddea658baf976e10a |
| `disk2/_Plomien3` | 816 | `45534c2109090909` | 7.15 | ESL! (PowerPacker format) | 1280 | faf6f69241ca6b23bb3acef7576debabff519a2e |
| `disk2/_WYBIJACZKA` | 280 | `45534c2109090909` | 6.63 | ESL! (PowerPacker format) | 640 | 5935b4c6b7e198c951b89a5a65495527f1e8c4a7 |
| `disk2/_WYBIJACZKA2` | 388 | `45534c2109090909` | 6.77 | ESL! (PowerPacker format) | 640 | 15ff32064d084f92fc65c36f38d8ebef315d1b8f |
| `disk2/_WYBIJACZKA3` | 400 | `45534c2109090909` | 6.88 | ESL! (PowerPacker format) | 640 | 8e03862d5e14872b6a8999665a2a1a60bca244c2 |
| `disk2/_ZNIKAJKA1` | 644 | `45534c2109090909` | 7.10 | ESL! (PowerPacker format) | 1920 | d7db39e75d6a4b29290ad323426d5eb048b0bfc5 |
| `disk2/_ZNIKAJKA2` | 708 | `45534c2109090909` | 7.12 | ESL! (PowerPacker format) | 1920 | 5ad6a9b5014aa7a2fdea89560b26d53467c277e3 |
| `disk2/_ZNIKAJKA3` | 696 | `45534c2109090909` | 7.08 | ESL! (PowerPacker format) | 1920 | 60ba34f4873b28d7f4b642589635a191d70eb590 |
