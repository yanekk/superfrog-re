# Playbook: rebuilding an Amiga game

The steps the first game took, in order, each with what it produced and the lesson for the next game.
Lines that cite that game's files or name it carry `learned on:`, the one place a tier 2 file may name
a game (root CLAUDE.md, rule 3). A lesson here has one user so far: it is what worked once, not a law.
A second game that confirms a lesson or breaks one corrects this file in the commit that shows it.

## Starting a game

1. Copy `toolkit/template/` to `games/<game>/` and fill its placeholders.
2. Add `games/<game>/test.sh`, the game's test command. The selector (`tools/test.sh`) fails on a game folder without one.
3. Keep `games/<game>/smoke.toml`, copied from the template: it labels each of the game's test files smoke or full, and `tools/smoke.sh` fails on a game folder without one.
4. Write a first plan, its slug starting with the game's name, that walks the opening steps below.

## 1. Disks and how the game boots

- learned on: Superfrog. Produced `games/superfrog/docs/01-disk-format.md`, `games/superfrog/docs/02-boot-chain.md`, `games/superfrog/docs/05-chunk-inventory.txt`, `games/superfrog/tools/extract_disks.py`, `games/superfrog/tools/unpack.py`, `games/superfrog/engine/crates/sf-disk/src/carve.rs`.
- learned on: Superfrog. Its disks are not AmigaDOS: a custom trackloader reads back-to-back sector-aligned chunks, found by disassembling the boot block.

Lesson: read the boot block before assuming anything about the disk's layout, because that decides which tools apply. Disk images stay out of git and everything derived from them stays out too, from the first commit.

## 2. The packer

- learned on: Superfrog. Produced `games/superfrog/docs/03-atn-codec.md`, `games/superfrog/tools/atn.py`, `games/superfrog/engine/crates/sf-disk/src/atn.rs`.
- learned on: Superfrog. No public tool knew its `ATN!` cruncher, so `atn.py` runs the game's own 68k routine under Musashi, and the Rust decoder is tested against that.

Lesson: when the packer is unknown, the game's own routine run under an emulated CPU is the reference, and the port is tested against it rather than against someone's description. Check first whether a known packer applies.

## 3. The file formats

- learned on: Superfrog. Produced `games/superfrog/docs/04-t7mp-level-format.md`, `games/superfrog/docs/06-graphics-and-rendering.md`, `games/superfrog/docs/07-secret-passages.md`, `games/superfrog/docs/10-asset-formats.md`, `games/superfrog/tools/t7mp.py`, `games/superfrog/tools/render_levels.py`, `games/superfrog/tools/export_annotated.py`, `games/superfrog/engine/crates/sf-assets/`.
- learned on: Superfrog. Rendering every level to a picture and addressing a tile as `<level-id>:<col>,<row>` gave the owner and the sessions one way to point at a spot (`games/superfrog/docs/LEVEL-INDEX.md`).

Lesson: prove a format by rendering everything it holds, and settle early on how a person points at a place in the game. Number the notes in discovery order (`toolkit/template/docs/README.md`).

## 4. The executable map

- learned on: Superfrog. Produced `games/superfrog/docs/08-executable-map.md`, `games/superfrog/docs/09-object-system.md`, `games/superfrog/docs/11-sound.md`, `games/superfrog/docs/12-display.md`, `games/superfrog/docs/13-front-end.md`, `games/superfrog/docs/RE-WORKBENCH.md`, `games/superfrog/re/symbols.toml`, `games/superfrog/tools/re_listing.py`, `games/superfrog/tools/re_ghidra_import.py`.
- Generic tools from this step: `toolkit/tools/hunk_listing.py`, `toolkit/tools/ghidra_headless.py`, `toolkit/tools/ghidra/ApplySymbols.java`, `toolkit/tools/setup/install-ghidra.sh`.

Lesson: map the executables once, so each porting task starts from a shared map instead of finding the main loop again. Commit the names and notes; regenerate the listings and decompilation outside the repo, because they derive from copyrighted code.

## 5. The oracle and the first conformance case

- learned on: Superfrog. Produced `plans/rust-engine/tasks/T00-oracle-spike.md`, `games/superfrog/engine/crates/sf-oracle/`, `games/superfrog/engine/crates/sf-conform/`, `games/superfrog/engine/conform/README.md`, and the case `games/superfrog/engine/conform/cases/title-rng.toml`.
- Generic parts: `toolkit/crates/amiga-oracle/`, its patches in `toolkit/crates/amiga-oracle/patches/`, `toolkit/crates/amiga-conform/`, `toolkit/tools/setup/oracle.sh`.
- learned on: Superfrog. Its oracle spike ran before the map, and the harness was built before any game code (`plans/rust-engine/PLAN.md`); the steps here are in the order of the subject, not of that schedule.

Lesson: run the oracle spike first, as a gate: does the emulator boot the disks headless, repeat itself exactly, take input per frame and expose memory, picture and sound? Without it "faithful" is a claim nobody can check. Start on the faithful bar (`knowledge/self-testing-faithful-bar.md`), not frame-exact, which cost days. A first case is small: `toolkit/template/conform/case-stub.txt`.

## 6. Porting routine by routine against it

- learned on: Superfrog. Produced `games/superfrog/engine/crates/sf-core/`, its memory map `games/superfrog/engine/crates/sf-core/src/layout.rs`, and routine tests such as `games/superfrog/engine/crates/sf-conform/tests/collision_routines.rs`.
- Generic part: `toolkit/crates/amiga-oracle/src/routine.rs`, which runs one routine of the original alone on the CPU.
- learned on: Superfrog. One level with only the frog came before items, enemies and bosses; scripted runs from sync points came before a whole game from power-on (`plans/rust-engine/PLAN.md`).

Lesson: compare one routine, then one scene, then the whole game, and keep the core pure so a full run of behaviour is checkable in milliseconds. The rules each porting task names are in `docs/porting-routines.md`. Porting effort scales with the game's size; the machinery carries, the content does not (`toolkit/docs/reuse-for-another-game.md`).

## 7. The app

- learned on: Superfrog. Produced `games/superfrog/engine/crates/sf-app/`, `games/superfrog/engine/crates/sf-e2e/`.
- Generic parts: `toolkit/crates/amiga-app/`, `toolkit/crates/amiga-e2e/`.
- learned on: Forest Dumb Forever, confirming it. Its app took both whole, its windows moving into `amiga-app` when it needed them, and its level was checked by `games/forest-dumb/engine/crates/fd-e2e/tests/level_drill.rs` and `drill_os.rs`, never by the owner playing.

Lesson: prove the game headless before opening a window, then wire the window, sound and menus onto a core already known to match the original. Drive the built app end to end by script and through the real window system, so nobody has to play it to check it.
A level's drill plays its conformance stretches through the app as key presses with the recorder on, and judges each recording against the original with the bug compare (§8). Both games' apps pass opposite keys held together to the game, which no joystick can: a choice still open.

## 8. The recorder

- learned on: Superfrog. Produced `plans/bug-recorder/DESIGN.md`, `games/superfrog/engine/crates/sf-app/src/recorder.rs`, `games/superfrog/docs/BUG-FIX.md`, `games/superfrog/engine/crates/sf-conform/tests/bug_zips.rs`, `games/superfrog/docs/milestone-2-bug-recorder.md`.
- Generic part: `toolkit/crates/amiga-record/`.
- learned on: Forest Dumb Forever, confirming it. Its recordings are put on the original by pass, not by frame (`games/forest-dumb/engine/crates/fd-conform/src/bug.rs`). A pass reading in the frame of the one before ends what a compare can judge: the whole level's route, recorded, was judged to step 1 340 of 22 464. The compare's engine wrapper lacked one of the runner's hand-overs, so a recording differed where its case agreed, until the drill found it.

Lesson: a person who sees something wrong saves one zip and describes nothing further. A session replays it headless, compares it with the original, and answers with a fix and a new case or with a report that the original does the same. Spike first whether a recording made in the app replays on the original from a matching start, since everything after rests on that answer.
Retime a recording by the original's passes, and expect a long one to be judged only to its first lagged read: bug recordings are best short. Every hand-over the case runner gives the engine must reach the recording's engine too.

## 9. The planner

- learned on: Superfrog. Produced `plans/smart-planner/DESIGN.md`, `games/superfrog/engine/crates/sf-plan/`, `games/superfrog/engine/crates/sf-bench/`, `games/superfrog/engine/crates/sf-routeview/`, `games/superfrog/tools/handover.py`, `games/superfrog/tools/recording_guide.py`, `games/superfrog/docs/planner-stall-ask-owner.md`.
- learned on: Superfrog. One level's seed that failed the same way on every run finished first time with a 44-second recording of the owner playing the stuck stretch.

- learned on: Forest Dumb Forever. Produced `games/forest-dumb/engine/crates/fd-e2e/tests/finish_level.rs`, the planned route played in the real app from launch to the level's end (about 40 s, debug), and `finish_short.rs`, its smoke part. A whole level's PNG capture was 1.7 GB at 33 ms a frame, so the app gained the first game's `--capture-hashes` and runs unpaced (`fast-until`) to just before the end.

Lesson: build the measuring instrument before the thing measured: the benchmark and an early sample of the old tools came before the planner. When a route stalls, show where and ask the person to play that stretch, instead of adding hints per level. The rule stays with the game until the planner is generalised.
A level's final proof is its route as key presses in the real app, the state equal to headless every frame: give the app a hashes-only capture and an unpaced mode before writing it (both games needed them).

## What the second game forced into the toolkit

- learned on: Forest Dumb Forever. An AmigaDOS (OFS) `.adf` reader, `toolkit/crates/amiga-disk/src/ofs.rs` (2026-10-31), the first toolkit change: it loads its files through `dos.library`.
- learned on: Forest Dumb Forever. For its first level, the app's windows (`amiga-app`), the route planner (`amiga-plan`) and the map page (`amiga-routeview`), each moved when it first needed them. What stays per game, measured: `toolkit/docs/reuse-for-another-game.md`.
