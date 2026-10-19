# Superfrog RE — start here

Reverse-engineering **Superfrog** (Team 17, 1993, Amiga) to build a **map editor**.
The four Superfrog disk `.zip`s are the only source input; everything else is
derived by the tools.

> **Disk images are copyrighted and are NOT committed** (git-ignored). Put your own
> `Superfrog (1993)(Team 17)(Disk N of 4)...zip` files in the repo root before running
> the tools.

## Orientation
- The project lives on **`main`** (landed via PR #1 from branch `re-env-to-main`).
  Just work on `main` / a feature branch — there is no `superfrog-re-env` branch.
- `build/` (extracted ADFs, decompressed chunks, rendered PNGs) is git-ignored and
  regenerated. Before any render/secret/annotate tool: `python tools/extract_disks.py`
  then `python tools/unpack.py`. Full setup in [`README.md`](README.md).
- Findings are in [`docs/`](docs/) (read 01→07); [`README.md`](README.md) has the
  quickstart, tool list, and status.

## What's known (don't re-derive)
- **Disks** are a custom trackloader: back-to-back sector-aligned `ATN!` chunks.
- **`ATN!` is a custom cruncher.** The Rust port in `engine/crates/sf-disk/src/atn.rs`
  is the product decoder; `tools/atn.py` (the game's own 68k routine, emulated under
  Musashi) is the reference it is tested against. Both decode 103/106 chunks as carved;
  the other 3 need the next sector for their trailer (`docs/03`).
- **Levels** are `T7MP` files: `[tag][u32 len][data]`. `BODY` = `300×84` grid of
  16-bit cells = `tile index (low 10 bits) | flags (high 6 bits)`.
- **Graphics:** 840 per-tile `16×16` 5bpp tiles per world; the 32-colour OCS palette
  sits at offset `0x11300` in each world's sprite bank. See `docs/06`.
- **Collision:** a cell is solid iff `flag & 0b11000 == 0b11000`; **flag 38 = fake
  wall = secret passage** (`tools/find_secrets.py`, 144 found). See `docs/07`.
- **The Rust engine plays the whole game** (`engine/`, plan `plans/rust-engine/`;
  T213, the app's note timing, still open): `sf-core` is the game, `sf-app` the macOS app (`cd engine && cargo run
  --release -p sf-app`; "Play it" in the README). It is **faithful, judged by machine**
  (DESIGN §2.1): `sf-conform` runs the original on the vAmiga oracle and compares state,
  picture, sound and milestones; `sf-e2e` drives the real app, the whole game included.
  Read `plans/rust-engine/FINDINGS.md` before touching a mechanic.
- **Levels from files:** the app plays a `.sflevel` (JSON of the level's original files,
  DESIGN §2.10) in place of a disk level, and exports any disk level to one.

## Tile addressing (how the user points at spots)
`tools/export_annotated.py` makes per-level images with a col/row grid + level id.
Address a tile as **`<level-id>:<col>,<row>`** (0-indexed, top-left origin), e.g.
`W1-1:94,51`. Level-id ↔ chunk map and every passage's id are in
[`docs/LEVEL-INDEX.md`](docs/LEVEL-INDEX.md). When the user says "missing secret
passage at `<id>`", resolve id → level chunk, then inspect that tile's index+flag.

## Next up
Map editor, a new plan: view + **edit** `BODY`, saved as a `.sflevel` the engine plays
(File ▸ Play Level File…), so an edit is tried in the faithful engine with no repack.
Writing back to an `.adf` still needs an `ATN!` *compressor* — only the decompressor
exists (see `docs/03`). Also still open: the rarer collision flags (hazards / one-way /
water) and flag-0-through-terrain passages (deliberately excluded from the secret count
as ambiguous).

## Conventions
Do RE work on a feature branch and open a PR to `main`. Never commit the
copyrighted disk images (they are git-ignored).
Commit tools/docs (small); leave regenerable `build/` artifacts out of git.
