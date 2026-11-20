---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
  - cd engine && cargo test -q --workspace
  - if [ -d toolkit ]; then cd toolkit && cargo test -q --workspace; fi
---

# toolkit-extract — Design

The scope is `plans/amiga-toolkit-prep/DRAFT-5-extract-toolkit.md`, phase A only. Phase B (its own
repository) and the draft's decisions 1–3 are superseded by `DRAFT-6-monorepo-and-knowledge.md`: the
toolkit stays in this repository as `toolkit/`. The owner settled decisions 4 and 6 (planner and
route viewer stay out) and 5 (the recorder's generic half is in) before planning; they are not
reopened. Where the code disagreed with the draft the owner decided at planning (§7, 2026-10-28).
Where this file and the draft disagree, this file wins, because §7 records why.

## 1. Purpose

Move every engine part that carries to another Amiga game into a second Cargo workspace,
`toolkit/`, that holds no Superfrog code, data or names, and make Superfrog use it by path. It is
stage 4 of `plans/amiga-toolkit-prep/README.md`; game two (Forest Dumb Forever) is the step after.

### Success criteria

- `cd toolkit && cargo test -q --workspace` is green on its own, its name check finds no game name,
  and no `sf-*` crate is in any toolkit crate's dependency tree.
- The dry fit (T11) builds from toolkit crates only and its tests pass.
- Superfrog is unchanged by machine: `sf-conform -- all` gives the same verdicts as T01's baseline,
  sf-e2e `whole_game.rs` passes, the test block is green, no tolerance touched.
- `measure.sh` counts the toolkit and the north-star log has this step's column.

### Stance

A refactor under the faithful bar (rust-engine DESIGN §2.1): zero behaviour change, `Tolerance::OWNER`
fixed, a red check is a bug in the move. The only new code is what the owner asked for in §7 (the
display piece, the no-disk boot and the game-free oracle tests) plus the seams a split needs.

## 2. Behaviour specification

### 2.1 The toolkit workspace (T01)

`toolkit/Cargo.toml` is its own workspace (`members = ["crates/*"]`, `resolver = "3"`, the same
`[workspace.package]` as `engine/`), with its own `rust-toolchain.toml` (a copy of `engine/`'s pin)
and its own `Cargo.lock`. Engine crates depend on toolkit crates by path
(`../../../toolkit/crates/<name>` from a crate's manifest). Reason: draft 6 keeps one repository with
path dependencies, so a toolkit change and its use land in one commit and one review.

Profile overrides are read from the root manifest of the workspace being built, so each
`[profile.dev.package.<toolkit crate>] opt-level = 2` stays in `engine/Cargo.toml` (under the new
name) and is copied into `toolkit/Cargo.toml`; without the copy the toolkit's own tests run
unoptimised.

### 2.2 What moves, per crate

| Toolkit crate | From | Stays Superfrog's | Task |
|---|---|---|---|
| `amiga-clock` | `engine/crates/amiga-clock` | — | T01 |
| `amiga-chips` | `engine/crates/amiga-chips`, plus `Picture`, `Split`, `to_rgb12` from `sf-core/src/render/view.rs` | `View`, `widened`/`narrowed`/`black`, `DEFAULT_WINDOW` (widescreen, the title screen's window) | T01 |
| `amiga-paula` | `engine/crates/sf-paula`, renamed | DMACON `$8200` boot state becomes a parameter (§2.3) | T01 |
| `amiga-input` | `sf-core/src/input.rs` (`Joy`, `KeySet`, `FrameInput`, the token format); `keyboard.rs`'s `key_byte` and `CiaKeyboard` | `kbd_poll`, `KbdHost`, `polls()`, `KBD_TOGGLE`/`KEY_DOWN`/`KEY_UP`; `pass_clock.rs` | T02 |
| `amiga-disk` | `sf-disk`'s `zip.rs` (with `tests/common/zipwrite.rs`), `sha1.rs`, `ADF_SIZE` | `DiskSlot`, `Adf`, `DiskSet`, `identify.rs`, `atn.rs`, `carve.rs` | T03 |
| `amiga-disk` (modules) | `sf-assets`' `exe.rs` (hunk reader; the oracle's `load_exe` and `sound.rs::first_hunk` read through it, owner at plan review), `picture.rs` (IFF ILBM), `palette.rs`, `anim.rs` (IFF ANIM) | `ChunkId`, `World`, `Tag` naming, `pic_progress_map` | T04 |
| `amiga-oracle` | `sf-oracle`'s `amiga.rs`, `ffi.rs`, `routine.rs`, `trace.rs`, `build.rs`, `shim/`, `patches/`, `VAMIGA_COMMIT`; `tools/setup/oracle.sh`; the generic part of `config.rs` | `locate()`, the four disks, `SUPERFROG_*`, `identify`, `main_checkout`, `disk1_in` | T05 |
| `amiga-conform` | the game-free half of `sf-conform` (§2.6) | cases, sync, milestones, memory-variable comparison, level outcomes, `routine.rs` Rig, `lib.rs` runner, `main.rs` | T06 |
| `amiga-app` (library) | `sf-app`'s `atomic_write.rs`, `audio.rs`, `pacing.rs`, `pad.rs`, `lock.rs`, `video.rs` with the shaders, the generic part of `capture.rs`, `input.rs`'s Amiga key table and `KeyMap` | `main.rs` run loop, `script.rs`, menus, windows, `import`, `levelfile`, `home`, `progress`, `states`, `settings`, `controls_window`, `recording_bar` | T07 |
| `amiga-e2e` | `sf-e2e/src` (`app.rs`, `lib.rs`, `os.rs`) with the app's identity as parameters (§2.7) | every test under `sf-e2e/tests` | T08 |
| `amiga-record` | the generic half of `sf_core::recording`, `sf-app/src/recorder.rs`'s `Recorder`/`Replay` and zip read/write, `sf-conform/src/bug.rs`'s headless replay loop and picture check (§2.8) | `LevelSetup`, `Location`, `LevelId` naming, `MAGIC`, widescreen, the put-back by `LAYOUT`, the oracle comparison in `bug.rs` | T09 |

Each move keeps a `pub use <toolkit crate>::…` at the old path wherever callers or the frozen
chapters cite it, as amiga-chips and one-clock did, so the move does not ripple through every call
site. A toolkit crate's tests that need Superfrog's disks or data stay in Superfrog's crate.

### 2.3 Superfrog content inside moved code

- A Superfrog value the moved code bakes in becomes a parameter Superfrog passes:
  `AudioDma::booted`/`Paula::booted` take the boot DMACON (Superfrog passes `$8200`). Reason: the
  value is Superfrog's boot state, not Paula's.
- Test data copied from the game (the intro's copper list in `amiga-chips/src/copper.rs` tests,
  T89; the story set-ups in `amiga-chips/src/blit.rs` tests) is replaced in the toolkit by synthetic
  data; the original-data test moves to sf-core's tests. Reason: nothing from a disk or ROM enters
  the toolkit (draft "Copyright").
- A fitted constant whose comment records it was measured on Superfrog keeps its value and its
  comment, rewritten as a provenance line starting `learned on:` (§2.4).
- The 313-line assumptions in `Timeline::run`, `word_times`, the copper loops and
  `Paula::set_frame_lines` stay as they are (READINESS known item; parameterising is game two's
  first step, §8).

### 2.4 The name check (T01)

A toolkit test, `toolkit/tests/` or a test in the smallest toolkit crate that can see the tree,
fails when any tracked file under `toolkit/` matches, case-insensitively,
`superfrog|team ?17|sf[-_](core|assets|disk|oracle|conform|app|e2e|plan|bench|routeview|paula)`,
except a line containing `learned on:`. It covers code, comments, Cargo manifests, shaders, patches,
scripts and Markdown. Reason: draft 6 rule 3 ("no game names above tier 3"), and a check is the only
thing that keeps the rule after the first convenient exception. A line legitimately needing a name
uses `learned on:` and states where a number came from, nothing more.

### 2.5 The oracle (T05)

- Disks are a list: `amiga_oracle::OracleConfig { kickstart: PathBuf, disks: Vec<PathBuf>,
  boot: Option<usize>, … }`, with `Amiga::insert_disk(drive, index)`. `sf-oracle` keeps `locate()`
  and maps `DiskSlot` to an index. `boot: None` powers on with df0 empty (binding the shim's
  existing `sf_amiga_eject_disk`, or not inserting). Reason: game two has two disks and its own
  names; the Kickstart tests boot with none.
- The trace and sync cache keys stay byte-identical for Superfrog's existing runs: today's key text
  (`trace.rs` `run`) appends nothing when booting `DiskSlot::D1` and ` boot {:?}` (` boot D3`) for
  any other slot; the toolkit appends nothing for the caller's default boot and ` boot <label>` with
  the caller's label otherwise, and `boot: None` gets a label of its own (` boot none`). Reason: every
  cached trace would otherwise be recomputed for nothing.
- Cache roots are the caller's: the toolkit takes them as parameters; its own default is
  `~/.cache/amiga-toolkit/` with an `AMIGA_TOOLKIT_CACHE` override. Superfrog keeps its trace and
  sync caches under `~/.cache/superfrog-re/` exactly as today.
- The five patches' `superfrog-re (sf-oracle):` comments become `amiga-toolkit (amiga-oracle):`.
  This changes the patch-set hash, so `oracle.sh` builds vAmiga once more and the oracle's `BUILD`
  changes, which invalidates the trace and sync caches once (owner told at planning; §7). Do it in
  one commit; the C symbol prefix `sf_amiga_*` and `links = "sfshim"` are renamed in the same
  rebuild (`amiga_*`, `amigashim`).
- `tools/setup/oracle.sh` moves to `toolkit/tools/setup/oracle.sh`; `tools/setup/oracle.sh` stays as
  a two-line wrapper that execs it, because DESIGN setup blocks, docs and the owner run that path.
  `tools/test_oracle_sh.py` follows the script and its paths.
- A test that cannot find the Kickstart fails with where it looked, as today's oracle tests do
  (owner, 2026-10-28). Its path: `AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`; sf-oracle keeps
  reading `SUPERFROG_KICKSTART` first.

### 2.6 The comparison harness's game-free half (T06)

Moves: `sound_events.rs`; from `compare.rs` `picture`, `picture_from`, `picture_except`,
`race_rows`, `paula`, `Mode`, `Tolerance`, `PictureClass`, `classify`, `PictureJudge`,
`PassMatch`/`PassStep`/`Side`, `reg_name`, `PixelMismatch`; from `report.rs` `write_diff_png`, `png`,
`crc32`; from `channels.rs` `level`, `mixer_levels`, `line`, `parse`, `judge` and its `Tolerance`;
from `feed.rs` `swing`, `parse_audio`; from `script.rs` the token lexer with a hook for game tokens
(`parse_with` already takes the digit lookup as a closure; Superfrog's `C<entry>.<k>` password token
is its hook); `faithful.rs`'s synthetic tests (lines 50–232 today). Everything that takes a `Case`,
a `VarDef`, `FrameOut`, `LevelOutcome` or a milestone stays. `Tolerance::OWNER` keeps its values.

### 2.7 The end-to-end driver (T08)

`amiga_e2e` takes an `AppSpec`: cargo package and binary name, the flags it always passes, the
environment variables to clear and the keep-scratch variable, a function that provisions disks into
a scratch home, and the seed-cache location. Superfrog's spec lives in `sf-e2e/src/lib.rs`
(`superfrog`, `--home --capture …`, `SUPERFROG_HOME`, `SF_E2E_KEEP`, the four disks). The capture file
names (`frame-NNNNN.png`, `inputs.txt`, `audio.wav`, …) are constants in `amiga-app`'s capture
module, which the driver reads, so they cannot drift apart.

### 2.8 The recorder's generic half (T09)

`amiga_record::Recording<G: GameRecording>`: `build`, `seed` bytes, `note`, `inputs:
Vec<FrameInput>`, and a game header `G::Header` that holds the start point, the stop point and any
game toggles (Superfrog's: `LevelSetup`, `Location`, widescreen and its `W` toggles). The text form
and `MAGIC` are the game's to supply; the format of every recording and bug zip written today stays
byte-identical, and every bug zip under `bugs/` and in the tests still parses and replays. `Recorder`
and `Replay` are pure state machines; the zip layout (`ZIP_RECORDING`, `ZIP_SCREENSHOT`,
`MAX_FRAMES`) and zip read/write use `amiga-disk`; writing to disk goes through `amiga-app`'s atomic
write. The headless replay loop takes a game through a small trait (start from the header, step one
frame, give the picture); the picture check (`screenshot_differs`, `picture_png`, `decode_png`) is
generic. `bug.rs`'s oracle comparison, put-back and case writing stay in sf-conform. The two
duplicate `read_zip`s (sf-app, sf-conform) become one.

### 2.9 The display piece (T10)

`amiga_chips::display::draw(mem: &impl Ram, writes: &[copper::Write], start: copper::Fetch) -> Picture`:
lores, up to 5 bitplanes plus EHB, no sprites, no dual playfield. Its register state and fetch
timing are the generic core of sf-core's Denise (`render/mod.rs`: `Planes` with BPLxPT, BPLxMOD,
BPLCON1 delay, the per-line pointer advance, `PIPELINE`/`HIRES_PIPELINE` from docs/12, the colour
and `Split` apply), moved into `amiga_chips::display`; the window is the existing `copper::Fetch`,
not a new type. sf-core's renderer calls the moved core and keeps what is Superfrog's: `Mem` and its
write log, the level play field, widescreen, sprites and `panel::fetch`. Reason: one Denise, so a
timing fix found on game two is made once and game two starts from the measured pipeline (owner,
2026-10-28, at plan review). The move is under the faithful bar: sf-core's pictures stay
bit-identical.

### 2.10 Unhappy paths

- A red check before any change: T01 runs `sf-conform -- all` on its base before its first change
  and records every verdict; a later red is compared against that, not assumed to be this plan's.
  The known reds carried in (READINESS): `channels`' whole-game `T216` label assert.
- A red check after a move is a bug in the move. Never touch a tolerance or a fitted constant.
- The trace caches refill after T05's rebuild; a slow first comparison run after T05 is expected,
  not a defect.
- A save state written before this plan is refused by the new build tag, as by any engine change
  (DESIGN rust-engine §2.8); that is not a regression.

## 3. Architecture

### 3.1 The boundary

The pure/shell boundary is rust-engine DESIGN §3.1. In the toolkit, pure: `amiga-clock`,
`amiga-chips`, `amiga-paula`, `amiga-input`, `amiga-disk`, `amiga-record`. Shell: `amiga-oracle`,
`amiga-conform`, `amiga-e2e`, `amiga-app`. A pure crate reads no clock, file, network or random
source and depends only on pure toolkit crates and an allowlist (today's `ALLOWED` minus the sf
names).

What enforces it: a purity test in the toolkit, written from `engine/crates/sf-conform/tests/purity.rs`
with toolkit paths and lists, and the engine's own purity test kept, with the toolkit's pure crates
on its `ALLOWED` and its source scan pointed at their new paths. If a purity test fails, the fix is
to move the code, never to relax the test. Two tests rather than one parameterised file, because each
workspace's test must run from that workspace alone.

### 3.2 Dependencies between toolkit crates

```
amiga-clock ← amiga-paula ← amiga-chips
amiga-input            amiga-disk
amiga-record → amiga-input, amiga-disk, amiga-chips (Picture)
amiga-oracle → amiga-chips, amiga-paula, amiga-input, amiga-disk (build: sha1)
amiga-conform → amiga-chips, amiga-paula, amiga-input, amiga-oracle (Rgb12Frame, FrameCapture, AudioTap)
amiga-app → amiga-chips, amiga-paula, amiga-input, amiga-disk
amiga-e2e → amiga-chips, amiga-app
```

No toolkit crate depends on an engine crate. `amiga-app` and `amiga-e2e` never depend on
`amiga-oracle`, as today's sf-app never contains an emulator.

### 3.3 The build tag

`sf-app/build.rs` hashes the crates whose code is the game a save state holds (`GAME`, today
`sf-core`, `sf-assets`, `sf-paula`, `sf-disk`). Each task that moves code out of those crates adds
the toolkit crate's path to the hash (`../../../toolkit/crates/<name>`), so a toolkit change still
changes the tag. `amiga-chips` and `amiga-clock` are in no tag today; T01 adds them with the move,
because they hold the blitter, copper and clock a save state's behaviour depends on.

### 3.4 Architecture docs

`tools/architecture.py`'s `DOMAIN` and `PATH_ROOTS` gain `toolkit/`, and each chapter's `owns` follows
its files (`sound-chip` owns `toolkit/crates/amiga-paula/`, and so on; `disks-and-assets`'
`engine/crates/sf-disk/` is split). Chapter bodies stay frozen: the cited-path check gains a table
of moved prefixes (old → new), defined once in `architecture.py`, and resolves a cited old path
through it. Reason: the bodies are a dated record (engine-architecture plan), and the table keeps the
check honest without editing them.

### 3.5 measure.sh

M2 and M4/M5 scan `engine/crates` and `toolkit/crates`; M4's loop lists toolkit crates as `PASS` when
they pass. Reason: otherwise the moved items read as zero, which looks like progress and is not.

## 4. Testing

- The test block above, every task. Its third line runs the toolkit's tests; it is guarded by
  `[ -d toolkit ]` only so the block runs on a base where T01 has not landed; after T01 it always runs.
- Every task also runs, from `engine/`: `cargo test -p sf-conform --test frog_cases`,
  `--test alone`, `--test beam`, `--test story -- --ignored` (80 s–5 min each), and the tests named
  in its task doc.
- T01 opens with `cargo run -q --release -p sf-conform -- all` on its base (~2 h; run in parts if a
  time limit cuts it, as one-clock T03 did), recorded case by case in `FINDINGS.md`.
- T11 closes with `sf-conform -- all` and `cargo test -p sf-e2e --test whole_game --
  --include-ignored` (~90 min), compared with T01's verdicts.
- Each moved calculation that changes signature is proven by keeping the old function in the test
  module and comparing over a sweep, as one-clock did.

The conformance runs need the owner's disks and Kickstart and the oracle build, which are on this
machine; they are a worker's to run and record.

## 5. Environment

| | |
|---|---|
| OS | macOS (Darwin 25.5) |
| Language / runtime | Rust 1.99.0 (cargo 1.99.0), edition 2024, pinned by `engine/rust-toolchain.toml`; Python 3 for `tools/` |
| Disks, ROM | the four Superfrog zips in the main checkout's root; `~/amiga-roms/kick1.3.rom` |
| Tools | `xdftool` (amitools) at `/opt/homebrew/bin` |

**The test command.** The block at the top: the repo's stored lines from `.pir/settings.json`, plus
the toolkit line. T01 adds the same line to `.pir/settings.json`, because pir's single runs read that
file. Each line runs in its own shell from the repo root. The person's
`~/.pir/superfrog-re/settings.json` sets `testTimeoutMinutes: 120`; the cargo line took 1 h 45 min at
one-clock's planning, and the whole block 70 min in a fresh copy at plan review (2026-10-28, compile
included). Tests that move out of `engine/` leave its line and join the toolkit's, so the
total should change little, but the toolkit crates compile twice (two target directories). The
toolkit line was not measured at planning because `toolkit/` does not exist yet; T01 records its time.
Output is quiet: `-q`, `NO_COLOR=1`. `COLORTERM=truecolor` is set here but forces nothing.

**Setup.** The stored lines (a venv and `pip install -q`); unchanged.

**Dependencies.** None added. The toolkit uses what its crates use today (`serde`, `miniz_oxide`,
`png`, `cc`, `cpal`, `wgpu`, `winit`, `libc`, the objc2 crates, …).

### 5.1 What the test command cannot reach

Nothing here needs a person. `sf-conform -- all` and `whole_game.rs` are outside the block for their
length, not for a person.

### 5.3 Outside actions

Approved by the owner at plan review, 2026-10-28.

| Action | Command | Bin | What it touches | Way back | Cost | Login check |
|---|---|---|---|---|---|---|
| Fetch and build vAmiga at the pinned commit (T05's one rebuild) | `tools/setup/oracle.sh` (the wrapper; already on `.claude/settings.json` `allow`) | worker | read-only `git fetch` of public vAmiga source; builds into `~/.cache/amiga-toolkit/vamiga`, outside the repo | delete `~/.cache/amiga-toolkit/vamiga`; the old build under `~/.cache/superfrog-re/vamiga` is untouched | free | — |

Every other step is local. An action that turns up anyway is `ask` (CLAUDE.md).

## 6. Recovery

A refactor on a feature branch; the way back is not merging it. T05's vAmiga rebuild lands in a new
cache directory beside the old one, so switching back to `main` finds the old build untouched.

## 7. Decisions and rationale

- 2026-10-28, owner: phase A only; the toolkit stays in this repository (draft 6). Decisions 4, 5
  and 6 as settled before planning.
- 2026-10-28, owner: `amiga-app` takes only the clean parts (§2.2). There is no game interface;
  `main.rs` calls `sf_core::Game` directly in about 15 places, and its script directives, ⌘ commands
  and `import::Ui` mix with Superfrog's menus. Designing the run loop's interface against one game
  is a guess; it is designed when game two plays in the app. Rejected: the draft's full scope; leaving
  the app out entirely.
- 2026-10-28, owner: `amiga-conform` takes the game-free half (§2.6). `compare.rs` imports `Case`,
  `VarDef`/`VarId` and `FrameOut`; `report.rs` holds `LevelOutcome` and `MilestoneDiff`; `routine.rs`'s
  Rig is built on `State::transplant`, `LAYOUT`, `STACK = 0xC7F800` and Superfrog's sound banks.
  Rejected: a generic Rig behind a designed interface, for the same reason as the app.
- 2026-10-28, owner: the picture type moves (§2.2) and a display piece draws one from chip
  memory (§2.9); at plan review the owner chose to build it from sf-core's Denise core, moved, over a
  second simpler copy beside it. The draft's dry fit names "the display path to a Picture", which does not exist:
  sf-core's renderer draws from `State` and `DisplayData`. Rejected: a dry fit without a drawn picture.
- 2026-10-28, owner: the hunk loader and IFF readers move from sf-assets (§2.2, T04); no AmigaDOS
  (OFS) reader is written here. NORTH-STAR said game two needs one; it is written against game two's
  real disks in its own plan. Rejected: writing it now against xdftool images; leaving the readers.
- 2026-10-28, owner: a toolkit test without its Kickstart fails with where it looked (§2.5). The
  draft said "skips, as today"; today's oracle tests fail (`determinism.rs:1-4`,
  `sf-conform/src/disks.rs:5`).
- Planner, told to the owner: `keyboard.rs` is half Superfrog's (`kbd_poll` at h0+$15ff2, its
  addresses), so only `key_byte` and `CiaKeyboard` move. `adf.rs` defines `DiskSlot` ("Disk n of 4"),
  so only `ADF_SIZE` moves. The patches lose their name tag at the cost of one rebuild (§2.5). The
  architecture chapters stay frozen through a moved-path table (§3.4).
- Planner: `Picture` goes to `amiga-chips` rather than a new crate, because the display piece that
  draws it and the copper writes it reads are there.
- 2026-10-28, owner at plan review: one hunk reader in the toolkit. sf-assets' `exe.rs`, sf-oracle's
  `Cpu68k::load_exe` and `sound.rs::first_hunk` each walked the format; the oracle's loader keeps its
  game-loader rules on top of `amiga_disk::hunk`, so T05 waits for T04. Reason: game two is a hunk
  executable read by both paths, and a format fix is then made once. The two PNG writers
  (`capture::png`, `report::png`) stay apart: different encoders, so merging changes bytes the
  checks compare.
- Planner: the draft's `amiga-e2e`/`amiga-app` task is two (T07, T08), and the readers are their own
  task (T04), so each task's "Done when" fits in three lines.
- Survey (planning Stage 4), 2026-10-28: three readers checked every row of the draft against the
  code; the disagreements are the decisions above. Found beyond the draft: `sf-app/build.rs`'s tag
  omits amiga-chips and amiga-clock (§3.3); the binary is `superfrog`, not `sf-app`; the zip writer
  is `sf_disk::zip::write_stored`; the recorder is ~3.1k lines with tests, not 2 000, and its stop
  point is also Superfrog's `Location`; `tests/purity.rs` and `architecture.py` find crates by path.

## 8. Explicitly out of scope

- The route planner and route viewer (draft decisions 4, 6): their core is in Superfrog's terms.
- `sf-app`'s run loop, script directives, menus and windows, and `sf-conform`'s Rig (§7).
- Draft 5 phase B and draft 6's tasks (knowledge tiers, `games/superfrog/`, playbook, template).
- An AmigaDOS file reader (§7).
- Parameterising the 313-line frame in `amiga-clock`, the copper and Paula (READINESS): game two's
  first step, so its frame length is known when the parameter is designed.
- The `channels` `T216` label assert (READINESS): the owner's confirmation, separate work.
- Licences: the toolkit has none, like the repo; the GPL shaders keep their headers.
