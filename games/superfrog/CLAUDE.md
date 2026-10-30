# Superfrog (tier 3)

The game: rules and an index of what is known. Paths are from the repo root unless they are links.
A fact true of Superfrog goes here even if it might hold elsewhere (root CLAUDE.md, rule 1).

## Orientation
- Reverse-engineering **Superfrog** (Team 17, 1993, Amiga) to build a **map editor**. The four
  Superfrog disk `.zip`s are the only source input; everything else is derived by the tools. Your own
  `Superfrog (1993)(Team 17)(Disk N of 4)...zip` files go in this folder (`SUPERFROG_DISKS` overrides
  it; the repo root is the fallback).
- `games/superfrog/build/` (extracted ADFs, decompressed chunks, rendered PNGs) is git-ignored and
  regenerated. Before any render/secret/annotate tool: `python games/superfrog/tools/extract_disks.py`
  then `python games/superfrog/tools/unpack.py`. Full setup in [`README.md`](../../README.md).
- Findings are in [`games/superfrog/docs/`](docs/) (read 01→07); [`README.md`](../../README.md) has the
  quickstart, tool list, and status.

## What's known (don't re-derive)
- **Disks** are a custom trackloader: back-to-back sector-aligned `ATN!` chunks.
- **`ATN!` is a custom cruncher.** The Rust port in `games/superfrog/engine/crates/sf-disk/src/atn.rs`
  is the product decoder; `games/superfrog/tools/atn.py` (the game's own 68k routine, emulated under
  Musashi) is the reference it is tested against. Both decode 103/106 chunks as carved;
  the other 3 need the next sector for their trailer (`games/superfrog/docs/03`).
- **Levels** are `T7MP` files: `[tag][u32 len][data]`. `BODY` = `300×84` grid of
  16-bit cells = `tile index (low 10 bits) | flags (high 6 bits)`.
- **Graphics:** 840 per-tile `16×16` 5bpp tiles per world; the 32-colour OCS palette
  sits at offset `0x11300` in each world's sprite bank. See `games/superfrog/docs/06`.
- **Collision:** a cell is solid iff `flag & 0b11000 == 0b11000`; **flag 38 = fake
  wall = secret passage** (`games/superfrog/tools/find_secrets.py`, 144 found). See `games/superfrog/docs/07`.
- **The Rust engine plays the whole game** (`games/superfrog/engine/`, plan `plans/rust-engine/`):
  `sf-core` is the game, `sf-app` the macOS app (`cd games/superfrog/engine && cargo run
  --release -p sf-app`; "Play it" in the README). It is **faithful, judged by machine**
  (DESIGN §2.1): `sf-conform` runs the original on the vAmiga oracle and compares state,
  picture, sound and milestones; `sf-e2e` drives the real app, the whole game included.
  Read `plans/rust-engine/FINDINGS.md` before touching a mechanic.
- **Routes are planned with `sf-plan`** (`games/superfrog/engine/crates/sf-plan`, plan `plans/smart-planner/`), the one
  level-routing tool; `plan_bonus` stays for the bonus level. README "Planning a route".
- **Levels from files:** the app plays a `.sflevel` (JSON of the level's original files,
  DESIGN §2.10) in place of a disk level, and exports any disk level to one.
- **Handed a bug zip** (`games/superfrog/bugs/bug-<level>-<date>.zip`, "fix the bug in …"): follow
  [`docs/BUG-FIX.md`](docs/BUG-FIX.md) step by step: replay it, compare it with the original, then
  a quirk report or a fix with a new conformance case (plan `plans/bug-recorder/`).
- **Seeing a route: `sf-routeview`.** Replays app-timed routes (`# timing: app`) on the engine
  and writes `games/superfrog/build/route-viz/index.html`: the level map, the frog's path, every coin, collectible
  and enemy (named, all worlds), events (coins, kills, hearts lost, jumps), a timeline, and where a
  failed route stops making progress. `cd games/superfrog/engine && cargo run --release -p sf-routeview --
  --planner` draws every smart-planner route with its verdict (failed / faults / clean, from
  `plans/smart-planner/bench/results.jsonl`); `-- route.txt [more.txt] [--guide waypoints.txt] [--out DIR]` draws any
  routes, a waypoint file overlaid. `sf-bench` redraws it after each guided set-C run
  (`--no-view` skips). Use it to show the owner a route or where one stalls instead of
  describing it; the library (`sf_routeview::dump::replay`) gives the same facts as data. The page
  holds disk graphics: keep it in `games/superfrog/build/`, never commit it.
- **When the planner struggles, ask the owner to play it** (owner, 2026-10-25). A level routing job
  whose `sf-plan` finds no exit on two seeds, or stalls at the same spot twice, stops and asks the
  owner instead of adding per-level hints or tuning. First show where it is stuck with
  `sf-routeview` (above): the failed routes, with `--guide` the guide they followed, and the
  progress-stops marks; name the spot as `<level-id>:<col>,<row>`. Then hand over one command that
  starts the app a little before it: `python3 games/superfrog/tools/handover.py ROUTE.txt <level> <col>,<row>`
  writes the app script (a route there, fast-forwarded with `fast-until`, the recorder on, every key
  released and a pause; keys from the owner's Controls mapping, the owner jumps with Space) and prints the
  owner's command. The owner plays the stretch and saves the recording with ⌘.; `python3
  games/superfrog/tools/recording_guide.py <zip>` turns it (lead-in and the owner's play, from the level's start) into the
  guide to plan again with, `sf-plan <level> --guide-ordered --guide <it>`; show the new route in
  `sf-routeview` with that guide overlaid. W6-4's seed 3 failed the same way on every run and
  finished first time with a 44-second recording of its climb. This is for routing work: the
  smart-planner benchmark's numbers stay unaided.

## Tile addressing (how the user points at spots)
`games/superfrog/tools/export_annotated.py` makes per-level images with a col/row grid + level id.
Address a tile as **`<level-id>:<col>,<row>`** (0-indexed, top-left origin), e.g.
`W1-1:94,51`. Level-id ↔ chunk map and every passage's id are in
[`docs/LEVEL-INDEX.md`](docs/LEVEL-INDEX.md). When the user says "missing secret
passage at `<id>`", resolve id → level chunk, then inspect that tile's index+flag.

## Next up
Map editor, a new plan: view + **edit** `BODY`, saved as a `.sflevel` the engine plays
(File ▸ Play Level File…), so an edit is tried in the faithful engine with no repack.
Writing back to an `.adf` still needs an `ATN!` *compressor* — only the decompressor
exists (see `games/superfrog/docs/03`). Also still open: the rarer collision flags (hazards / one-way /
water) and flag-0-through-terrain passages (deliberately excluded from the secret count
as ambiguous).

## Notes

Besides the numbered `docs/` chapters, one topic a file:

- [docs/atn-decompression.md](docs/atn-decompression.md): the `ATN!` container and its two decoders.
- [docs/t7mp-level-format.md](docs/t7mp-level-format.md): the level file and `BODY`, tiles and palette.
- [docs/collision-and-secrets.md](docs/collision-and-secrets.md): flag meanings, flag 38, tile addressing.
- [docs/milestone-2-bug-recorder.md](docs/milestone-2-bug-recorder.md): why the bug recorder exists and the owner's decisions.
- [docs/next-plan-difficulty-modes.md](docs/next-plan-difficulty-modes.md): backlog, conformance on easy (and the question of hard).
- [docs/video-ghost-trial.md](docs/video-ghost-trial.md): the YouTube trail trial and its unpushed branch.
- [docs/planner-stall-ask-owner.md](docs/planner-stall-ask-owner.md): why a stalled planner goes to the owner, and the order.
