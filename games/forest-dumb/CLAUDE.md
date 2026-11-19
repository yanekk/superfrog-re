# Forest Dumb Forever (tier 3)

Rules and an index of what is known about the game. Paths are from the repo root unless they are
links. A fact true of this game goes here even if it might hold elsewhere (root CLAUDE.md, rule 1).
Start from `toolkit/PLAYBOOK.md`, whose opening steps fill most of this file.

## What the game is

Forest Dumb Forever, © 1996 L.K. Avalon (read off the title screen, 2026-10-31), developer Eagle Soft: an Amiga platformer in the manner of
the first game. This project reverse-engineers it and rebuilds it in Rust on the toolkit. Level 1 is built
and judged against the original (plan `plans/forest-dumb-level-1/`); the app below opens in it. Source of everything below: `plans/amiga-toolkit-prep/NORTH-STAR.md`, J1.

## Turning the original's code into Rust

Translate with the translator first: `python3 -I games/forest-dumb/tools/translate.py --task T{nn}` (or
`'hN+$off'`, or a symbol) prints one judge line a routine and writes its Rust to
`build/translate/<hN-off>/translated.rs`. A `pass` is the engine for that routine; a `fail` or `stop` is
a translator defect, fixed by mnemonic or P-code pattern, never by a hint or an edit to generated Rust;
hand-written Rust replaces a translated routine only where the owner chose, checked against it through
the seam. Haiku writes generators (`no-gen`) and drafts chosen rewrites first
(`generate.py`, `reshape.py`; owner 2026-11-12, translator T44), by hand after a miss. The procedure: [docs/11-translation.md](docs/11-translation.md), "The
procedure"; a task's pack (`tools/task_pack.py`) carries it.

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

`cd games/forest-dumb/engine && cargo run --release -p fd-app` runs the translated program from power-on,
with sound (level 1's effects wait on translator T57): the disk 2 prompt (fire), the title (fire is Start), the menus, the levels, level 1 on its hand
engine through the seam. `-- --level 1` opens on level 1's first frame, reached by running the program
headless through the title; `-- --engine hand` opens straight in `W1-1` on the hand engine, as before,
for comparison. A stop of the translated program freezes the picture, names the stop over it and
saves a bug zip of the play from its start (`docs/BUG-FIX.md`, "A stop zip"). The translation is generated on this machine (`python3 -I
games/forest-dumb/tools/translate.py --build`); without it the app says so and exits 1.
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
- [tools/README.md](tools/README.md): every Python script here; `tools/translate.py` first (the translator's one command), then listing, call tree, the translator's parts, generate and reshape (Haiku, the first try for generators and rewrites), task pack, renders.
- [../../tools/README.md](../../tools/README.md): the repo's smoke and full tests, `smoke.toml` labels and `[[area]]`, the tier-name check.
- The toolkit crates' READMEs: `toolkit/crates/<crate>/README.md` (`amiga-oracle`, `amiga-conform`, `amiga-plan`, `amiga-app`, `amiga-input`, `amiga-routeview`).

## Notes

- [docs/05-file-formats.md](docs/05-file-formats.md): the level files, byte by byte.
- [docs/06-graphics-and-rendering.md](docs/06-graphics-and-rendering.md): tiles, sprites, the palette, the renderer.
- [docs/LEVEL-INDEX.md](docs/LEVEL-INDEX.md): every level id, its files and its render.

- [docs/frame-length.md](docs/frame-length.md): the measured frame length, 313 lines, dated.
- [docs/09-play-measured.md](docs/09-play-measured.md): W1-1 play measured: sync frame, passes, input, display, sprites, state variables, level end, the character's routines.
- [docs/10-object-system.md](docs/10-object-system.md): what each object value of W1-1 does, measured; how the freeze went; contact (creatures, water, tiles) measured.
- [docs/11-translation.md](docs/11-translation.md): the procedure (translate with `tools/translate.py` first, judge, rewrite by hand only where the owner chose, through the seam), what a pass proves, the committed generators, the inventory in [re/translate.toml](re/translate.toml) (`tools/call_tree.py`), and level 1's model procedure under History.
- [docs/12-level-start-draw.md](docs/12-level-start-draw.md): `level_init`'s first view (tiles, shapes) and what the plane's slack holds at the sync point (the title's music, `_BALLA`), measured.
- [docs/13-full-route.md](docs/13-full-route.md): the routes through the whole of W1-1 (the one route for both sides, the planner's, T48's on the original), how each is guarded and found again.
- [toolkit/docs/porting-routines.md](../../toolkit/docs/porting-routines.md): read before porting or reading any routine of the original; the first game's porting lessons as rules.
- [engine/conform/README.md](engine/conform/README.md): the conformance runner `fd-conform` and its case format.
- [docs/ROADMAP.md](docs/ROADMAP.md): backlog, the owner's sequence of plans, level 1 done.
- [docs/level-1-brief.md](docs/level-1-brief.md): the brief for the level 1 plan.
- [docs/13-translator-trial.md](docs/13-translator-trial.md): the translator trial (stage 0): 84 of 85 judgeable routines pass with no hand edit, no write into code on level 1's path; how to rerun it; owner's go (2026-11-07).
- [docs/translator-brief.md](docs/translator-brief.md): the brief for the full translator plan (stages 1 to 7), rewritten after the trial (2026-11-11): translated-first, hand-written where chosen, and the translator as new sessions' default first pass.
- [docs/14-translator.md](docs/14-translator.md): the translator's readings, dated: stage 1's gate (all 291 instruction kinds match Moira in every column, cycles included, 2 000 states each; `insn_gate.rs`); writes into code (no write on any game path; generated states the original writes into code on are `self-modifying`, not judged); stage 2's whole-tree gate (100 of 100 routines with a generator pass, nothing unsupported or unresolved, no hint; `translator_trial.py --whole`); the routines on `pcode-machine` (88 of 100, the 12 others the oracle's beam and blit, pinned; `--whole --machine`); worlds 2 to 6 from the title, `world_index` set at `load_world` (5 of 5 since T53: the OS's stretch after a load lined up as the load's, play judged at pass ends).
- [docs/15-os-surface.md](docs/15-os-surface.md): every OS call, OS memory access, interrupt and CIA register the program uses, measured on the original (`fd-conform os-surface`, data in [re/os-calls.toml](re/os-calls.toml)), and what it relies on each for.

## The first case

`cases/first-case.toml` compares the engine's `mt_counter` (`fd-core`) with the original's from power-on,
state only, on every frame of the idle title. Run it from `engine/` with
`cargo run -p fd-conform -- run cases/first-case.toml`; it also runs in `test.sh`.

## History

- Level 1 (2026-11-05 to 11-11) was translated by Muse, a model: literal ports, then engine code
  (`reshape.py`), judged on the rig. The translator replaced it (translator plan, DESIGN §2.13); the
  procedure is under History in [docs/11-translation.md](docs/11-translation.md).
- [spikes/muse-spark/README.md](spikes/muse-spark/README.md): spike 2026-11-04, Muse Spark contributor matches Opus translating routines up to ~180 instructions (blitter and Paula included) at 1/15-1/30 the cost; fails at ~300+.

## Backlog

- Level 1 (`forest-dumb-level-1`) is done; the next plans are in [docs/ROADMAP.md](docs/ROADMAP.md).
