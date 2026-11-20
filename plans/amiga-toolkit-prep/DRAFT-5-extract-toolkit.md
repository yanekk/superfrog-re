# Draft 5 — extract the toolkit into its own repository

**Status:** draft, 2026-10-27. Not reviewed. **2026-10-28: phase B and decisions 1–3 are superseded by
[draft 6](DRAFT-6-monorepo-and-knowledge.md): the toolkit stays in this repository (monorepo).** Input for `pir plan`, after drafts 3 (A+B) and 2 have
merged. Owner decisions 2026-10-27: extract after the keyboard and clock steps; the toolkit lives in
its own new repository. Game two (Forest Dumb Forever) is the step after this one and is not in it.

## The change

Move every part of the engine that carries to another Amiga game out of `engine/` into a
toolkit with no Superfrog code, data or names, and make Superfrog use it. Two phases, so pir can
build and check every move inside this repo before anything leaves it:

- **Phase A, in this repo:** a second Cargo workspace `toolkit/` beside `engine/`. Each toolkit
  crate moves there; `engine/` depends on it by path (`../toolkit/crates/…`). pir's test line gains
  `cd toolkit && cargo test -q --workspace`. Superfrog plays and conforms exactly as before.
- **Phase B, the split:** `toolkit/` becomes its own repository with the history of its files
  (`git subtree split --prefix=toolkit`, built into git; `git filter-repo` is not installed here), `engine/` switches its path
  dependencies to that repository pinned by commit, and `toolkit/` is deleted here.

## What moves (inventory from `docs/architecture/`; the plan confirms each by reading)

| Toolkit crate | From | Notes |
|---|---|---|
| `amiga-chips` | `engine/crates/amiga-chips` (draft 4) | as is |
| `amiga-paula` | `engine/crates/sf-paula` | renamed; rated `as-is` |
| the clock crate | draft 2's crate | as is |
| `amiga-input` | draft 3's `CiaKeyboard`, `sf-core/src/input.rs` (`Joy`, `KeySet`, `FrameInput`, the token format) | the keyboard adapter leaves sf-core |
| `amiga-disk` | `sf-disk`'s ADF, zip and SHA-1 parts | `identify.rs`, `atn.rs`, `carve.rs` stay Superfrog's |
| `amiga-oracle` | `sf-oracle`'s harness: `amiga.rs`, `ffi.rs`, `routine.rs`, `trace.rs`, the shim, `build.rs`, the five vAmiga patches, `tools/setup/oracle.sh` | `config.rs` split: the generic part takes a disk list and a Kickstart path; the four Superfrog disks, their names and the `SUPERFROG_*` variables stay in `sf-oracle` |
| `amiga-conform` | `sf-conform`'s `compare.rs` (bar `PANEL_ROWS`), `report.rs`, `sound_events.rs`, `channels.rs`, `script.rs` (bar the `C` token), `feed.rs`, `routine.rs` (`Rig` made generic over the port's context type), `faithful.rs`, `purity.rs` | about 3.3k lines per the verification chapter; cases, sync points, milestones, planners stay |
| `amiga-e2e` | `sf-e2e/src` | the binary name and flags become parameters |
| `amiga-app` | `sf-app`'s run loop, pacing, audio, picture and filters, input and pads, capture, script layer (`script.rs`, including `fast-until`, the unpaced fast-forward, and its loop in `main.rs`), atomic writes, lock | menus, level files, Start Level, import and save windows stay Superfrog's; the split is confirmed by reading |
| `amiga-record` (decision 5) | the bug recorder's generic half: `sf_core::recording`'s per-frame inputs, seed, build tag, stop point, zip of `recording.txt` + screenshot (`ZIP_RECORDING`, `ZIP_SCREENSHOT`, `MAX_FRAMES`); `sf-app/src/recorder.rs`'s `Recorder` and `Replay`; `sf-conform/src/bug.rs`'s headless replay loop and picture check | the start point (`LevelSetup`, Start Level), `Location` and `LevelId` naming of zips, the recording bar's Superfrog wording and the put-back into the original by `LAYOUT` stay Superfrog's, behind a game-defined start-point type |

Copyright: nothing from any disk or ROM enters the toolkit. `tools/check_no_disk_data.py` and the
purity test move with it. The GPL filter ports (`crt_geom.wgsl`, `crt_easymode.wgsl`) keep their
headers wherever they land.

## Testing without a game

The toolkit's own tests cannot use Superfrog's disks. Oracle and determinism tests that drive
Superfrog's inputs today (`sf-oracle/tests/determinism.rs`, `blits.rs`) stay in Superfrog; the
toolkit gets game-free equivalents: the owner's Kickstart 1.3 booted with no disk (to the
insert-disk screen) twice must give identical frames, and a snapshot restored mid-boot must
continue identically. The synthetic tests (`faithful.rs`, `purity.rs`, `compare.rs` units, chip and
Paula unit tests) need nothing. A test that needs the Kickstart reads its path from an environment
variable and skips with a message when it is absent, as today.

## Dry fit (owner, 2026-10-24: agreed by silence)

Last task of phase A: a throwaway program in `toolkit/` that depends only on toolkit crates (checked
with `cargo tree`) blits a pattern, runs the copper and display path to a `Picture`, steps the clock
across a frame and plays a Paula note, with a test asserting each. Booting game two on the oracle
is the next step's first task, not this one's.

## Decisions (open, for the owner)

1. **Where the new repository is hosted.** (a) Private GitHub repository under the owner's account,
   Superfrog pins it by commit in `engine/Cargo.toml` (`git = …, rev = …`), recommended: pir
   worktrees and any machine can fetch it. (b) Local only, Superfrog points at a sibling checkout:
   breaks in pir worktrees, whose relative paths differ. Creating and pushing the repository is an
   `ask` action. If nothing is said: (a), with the prompt asking before the push.
2. **Its name.** Recommended `amiga-toolkit`. If nothing is said: that.
3. **Licence.** This repo has none; the toolkit would start with none and stay private. If nothing
   is said: none, private.
4. **The route planner (`sf-plan`, 6 650 lines) stays out of this extraction.** Its search is
   generic in shape (archive of cells, return, explore, graph shortest path, guides, shortening),
   but `core/` is written in Superfrog's terms: `facts.rs` (frog cell, coins, switches), `coins.rs`,
   `danger.rs` (heart cost), `map.rs` (maps built from the frog's moves), `moves.rs`, `walk.rs`;
   only `graph.rs` and `guide.rs` (245 lines) name nothing of the game. Making it generic means
   designing a game interface (state, step, facts, move set) against one game, and it cannot be
   tested on game two until that game plays a level in the engine. Recommended: generalise it then,
   with Forest Dumb Forever (a Superfrog-like platformer, so likely a close fit) as its second
   user. The older planners under `sf-conform/examples/` are superseded by `sf-plan` and stay.
   If nothing is said: out.
5. **The bug recorder's generic half goes in** (`amiga-record`, row above). Unlike the planner its
   interface is small (a start point the game defines), Superfrog keeps using it so it is tested
   now, and game two needs a recorder as soon as the owner plays it. Cost: one task, a redesign of
   about a third of the recorder's 2 000 lines around that start-point type. If nothing is said:
   in, as task T6b after T6.
6. **The route viewer (`sf-routeview`, 1 900 lines) stays out, with the planner.** It exists to show
   planner routes (`verdict.rs` reads `sf-bench` results, `main.rs --planner`), and its data comes
   from Superfrog's replay (`dump.rs`: frog record offsets, coin, enemy and hurt events). The page
   (`page.html`) hard-codes Superfrog's categories, cell formula and title; `json.rs`, `site.rs`
   and `guide.rs` (about 270 lines) are generic. Generalising it means making the categories data,
   designed against one game, and it shows nothing for game two until game two plays levels and has
   routes. Recommended: generalise it together with the planner then. If nothing is said: out.

## Not changing

Superfrog's behaviour: every `sf-conform` case, `sf-e2e` and the app run as before; no tolerance
touched. `docs/architecture/` chapter bodies stay frozen; `owns` entries follow the moved files
(`tools/test_architecture_docs.py`), and toolkit files leave the count once phase B removes them.

## Tasks (rough)

Phase A: T1 the `toolkit/` workspace, `amiga-chips`, `amiga-paula` and the clock crate moved,
`engine/` on path dependencies, pir's test line extended; T2 `amiga-input`; T3 `amiga-disk`; T4
`amiga-oracle` with the game-free tests; T5 `amiga-conform`; T6 `amiga-e2e` and `amiga-app`; T7 the
dry fit, `toolkit/README.md` and the README section of this repo. Phase B: T8 the split, the push
(`ask`), `engine/` on the pinned git dependency, `toolkit/` removed, pir's test line back to
`engine/`. T2–T6 can run in parallel after T1.

## Done when

- The toolkit repository builds and tests on its own, with no Superfrog name in its code and no
  disk or ROM data in its history.
- Superfrog builds against the pinned toolkit commit; the test line and `sf-conform -- all` (~2 h)
  are green; `sf-e2e` `whole_game.rs` passes.
- `measure.sh` gets a final column (its M4 then counts the toolkit's crates); J2 asked.
