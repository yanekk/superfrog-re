# Forest Dumb Forever (tier 3)

Rules and an index of what is known about the game. Paths are from the repo root unless they are
links. A fact true of this game goes here even if it might hold elsewhere (root CLAUDE.md, rule 1).
Start from `toolkit/PLAYBOOK.md`, whose opening steps fill most of this file.

## What the game is

Forest Dumb Forever, © 1996 L.K. Avalon (read off the title screen, 2026-10-31), developer Eagle Soft: an Amiga platformer in the manner of
the first game. This project reverse-engineers it and rebuilds it in Rust on the toolkit. Level 1 is built
and judged against the original (plan `plans/forest-dumb-level-1/`); the app below opens in it. Source of everything below: `plans/amiga-toolkit-prep/NORTH-STAR.md`, J1.

## Its disks and where they live

Two AmigaDOS (OFS) `.adf` images in zips, kept outside the repo and never committed.
Lookup and file names: [docs/01-disk-location.md](docs/01-disk-location.md).
Layout: [docs/02-disk-format.md](docs/02-disk-format.md). Every file with size and SHA-1:
[docs/04-file-inventory.txt](docs/04-file-inventory.txt), written by `cargo run -p fd-tools -- extract` in
`engine/`, which extracts into `build/`. What is packed and how to unpack it (`ESL!` is PowerPacker's
format): [docs/08-packers.md](docs/08-packers.md).
Boot chain, the crack's programs and why the game asks for disk 2:
[docs/03-boot-chain.md](docs/03-boot-chain.md).
The main program's map, its file table and the first case's variable:
[docs/07-executable-map.md](docs/07-executable-map.md); names in [re/symbols.toml](re/symbols.toml), listing from
`tools/re_listing.py`.

Decided 2026-10-30: the cracked `[cr WT]` set is game two's source. The faithful bar's measuring stick
is this cracked release, whose executable may differ from the original's. An uncracked copy, if one
turns up, is the thing to compare it against.

## Tests

- Every new test file gets a label in [smoke.toml](smoke.toml); something crucial also gets a smoke test and an `[[area]]`. The full rule is under Conventions in the root CLAUDE.md.

## What's known (don't re-derive)

Inspected 2026-10-27 from the WHDLoad package and the disk set. Not yet reproduced by a tool in this
folder.

- Loads its files through `dos.library` from a floppy labelled `!EAGLE_SOFT!`: AmigaDOS files, no
  trackloader. The images' volume name is `FOREVER`; `!EAGLE_SOFT!` is a marker file on disk 2.
- Six worlds of level files: `_Mapa1-1` … `_Mapa6-1`, with `.Mapa`, `_Gen*`, `_MapaBobow*`.
- Executable `FOREST_DUMB_FOREVER`, 521 640 bytes, stored unpacked on disk: an AmigaDOS hunk
  executable of 5 hunks, hand-written 68k hitting the hardware, no AMOS or Blitz signatures.
- None of the Team 17 shared items apply: no ATN!, no T7MP, no Player 4.1A.
- Needs 368 KB chip and 0.5 MB other memory, which the oracle's A500 configuration provides.

## Addressing

`h4+$195c9` is byte `$195c9` of the main program's hunk 4, as in
[docs/07-executable-map.md](docs/07-executable-map.md), which gives each hunk's base on the oracle.

A spot is `<level-id>:<col>,<row>`, 0-indexed, top-left origin, 16-pixel cells, e.g. `W1-1:10,20`. Level
ids and their files: [docs/LEVEL-INDEX.md](docs/LEVEL-INDEX.md). `tools/render_levels.py --spot W1-1:10,20`
prints what is there; the pictures are in `build/renders/`.

## Playing it

`cd games/forest-dumb/engine && cargo run --release -p fd-app` opens the app straight in `W1-1`, silent.
Keys: Up (jump) on Space, Fire on Left Option, the arrows; Game ▸ Controls… rebinds them and a game pad.
Esc or ⌘Q quits. The disks are found as above, or `--disks DIR`; settings are in
`~/Library/Application Support/Forest-Dumb-RE/settings.toml` (`--home DIR` or `FOREST_DUMB_HOME`
elsewhere). Its flags and seatbelts (`--script`, `--capture`, `--max-frames`, `--timeout`) are in
`fd-app/src/args.rs`; `fd-e2e` drives it end to end (`cargo test -p fd-e2e`).

## Seeing a run on the map

`cargo run --release -p fd-routeview -- --script FILE` in `engine/` plays an input script
(`engine/conform/scripts/*.txt`) from W1-1's start and writes level 1's map with the character's path
to `games/forest-dumb/build/route-viz/index.html` (`--out DIR` elsewhere), which opens from disk. It
holds disk graphics: keep it under `build/`.

## Planning a route

`cargo run --release -p fd-plan -- level W1-1` in `engine/` plans W1-1 on the engine (`--rounds N`,
`--budget-mins N`, `--guide waypoints.txt`), writes the route as an input script to
`build/routes/W1-1.txt` (`--out`), draws it on the map page as `fd-routeview` does (`--page`), and plays
it on the original, nothing frozen, saying for how many passes the two agree and whether the route ends
where the planner's engine ended it. The whole level is planned (T27): `--rounds 2100` reaches the
level's end in round 2 095, about 4 minutes, with no life lost on the engine alone; on the original
it ends elsewhere. The one route both sides finish with no life lost (T84,
`engine/crates/fd-plan/tests/routes/W1-1.txt`, the route the app plays) is that route searched again
with both sides in the loop; both are committed and guarded, and
[docs/13-full-route.md](docs/13-full-route.md) says how to find them again. A shorter search stalls
with coins left and exits 1.
`fd-e2e`'s `finish_level.rs` plays the one route in the real app, by key presses, from launch to
"Level 1 complete" and fire's restart, the state equal to headless every frame (T28; full, about 40 s:
`cargo test -q -p fd-e2e --test finish_level -- --ignored`); `finish_short.rs`, its first 600 steps, is
in smoke. `level_drill.rs` (T29) is the level's drill: each enemy, mechanic and death stretch and the
route played in the app with the recorder on, every recording judged on the original by `bug compare`
(full, about 15 min: `-- --ignored`), and what a person tries unasked (smoke). Opposite keys held
together reach the game both, which no joystick can: a known gap (the plan's FINDINGS).

## Bug-fix procedure

A bug zip the owner recorded in the app: [docs/BUG-FIX.md](docs/BUG-FIX.md). Replay it
(`fd-conform bug replay`), compare it with the original (`fd-conform bug compare`, retimed by pass),
keep a difference as a case that fails before the fix.

## Tools: read the README before the source

Each tool has a short README saying how to call it. Read it, in one step, before opening the tool's
source; open the source only for what the README lacks.

- [engine/crates/fd-conform/README.md](engine/crates/fd-conform/README.md): the conformance runner's Rust API for tests (cases, routine judge and rig, generators, coverage, bug replay); the case format is in [engine/conform/README.md](engine/conform/README.md).
- [engine/crates/fd-plan/README.md](engine/crates/fd-plan/README.md), [fd-app](engine/crates/fd-app/README.md), [fd-e2e](engine/crates/fd-e2e/README.md), [fd-routeview](engine/crates/fd-routeview/README.md), [fd-tools](engine/crates/fd-tools/README.md), [fd-assets](engine/crates/fd-assets/README.md): the route planner, the app and its scripts and keys, end-to-end driving, the map page, the extract CLI, the asset readers.
- [tools/README.md](tools/README.md): every Python script here (listing, call tree, translate, reshape, generate, task pack, renders).
- [../../tools/README.md](../../tools/README.md): the repo's smoke and full tests, `smoke.toml` labels and `[[area]]`, the tier-name check.
- The toolkit crates' READMEs: `toolkit/crates/<crate>/README.md` (`amiga-oracle`, `amiga-conform`, `amiga-plan`, `amiga-app`, `amiga-input`, `amiga-routeview`).

## Notes

- [docs/05-file-formats.md](docs/05-file-formats.md): the level files, byte by byte.
- [docs/06-graphics-and-rendering.md](docs/06-graphics-and-rendering.md): tiles, sprites, the palette, the renderer.
- [docs/LEVEL-INDEX.md](docs/LEVEL-INDEX.md): every level id, its files and its render.

- [docs/frame-length.md](docs/frame-length.md): the measured frame length, 313 lines, dated.
- [docs/09-play-measured.md](docs/09-play-measured.md): W1-1 play measured: sync frame, passes, input, display, sprites, state variables, level end, the character's routines.
- [docs/10-object-system.md](docs/10-object-system.md): what each object value of W1-1 does, measured; how the freeze went; contact (creatures, water, tiles) measured.
- [docs/11-translation.md](docs/11-translation.md): translation by Muse: `tools/translate.py` (how to call it, what a pass proves), `tools/generate.py` (Muse writes a generator, judged by branch coverage) and the inventory of routines to translate, sized and given tasks in [re/translate.toml](re/translate.toml) by `tools/call_tree.py`.
- [docs/12-level-start-draw.md](docs/12-level-start-draw.md): `level_init`'s first view (tiles, shapes) and what the plane's slack holds at the sync point (the title's music, `_BALLA`), measured.
- [docs/13-full-route.md](docs/13-full-route.md): the routes through the whole of W1-1 (the one route for both sides, the planner's, T48's on the original), how each is guarded and found again.
- [toolkit/docs/porting-routines.md](../../toolkit/docs/porting-routines.md): read before porting or reading any routine of the original; the first game's porting lessons as rules.
- [engine/conform/README.md](engine/conform/README.md): the conformance runner `fd-conform` and its case format.
- [docs/ROADMAP.md](docs/ROADMAP.md): backlog, the owner's sequence of plans, level 1 done.
- [docs/level-1-brief.md](docs/level-1-brief.md): the brief for the level 1 plan.
- [docs/translator-brief.md](docs/translator-brief.md): the brief for the translator plan; owner chose translated-first, hand-written where chosen (2026-11-06).
- [spikes/muse-spark/README.md](spikes/muse-spark/README.md): spike 2026-11-04, Muse Spark contributor matches Opus translating routines up to ~180 instructions (blitter and Paula included) at 1/15-1/30 the cost; fails at ~300+.

## The first case

`cases/first-case.toml` compares the engine's `mt_counter` (`fd-core`) with the original's from power-on,
state only, on every frame of the idle title. Run it from `engine/` with
`cargo run -p fd-conform -- run cases/first-case.toml`; it also runs in `test.sh`.

## Backlog

- Level 1 (`forest-dumb-level-1`) is done; the next plans are in [docs/ROADMAP.md](docs/ROADMAP.md).
