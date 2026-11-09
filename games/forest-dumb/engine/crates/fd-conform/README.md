# fd-conform: the library a conformance test calls

fd-conform holds `fd-core` to the original. It has two levels. A case runs the original on the vAmiga
oracle from a sync point and the engine beside it, comparing variables at each pass end and the
picture. The routine rig runs one routine of the original on Moira over generated states and compares
it with a port. It also replays bug zips. It is test tooling only: the engine never depends on it, and
it holds no game logic of its own. The case file format, the CLI and how a level case compares are in
[`engine/conform/README.md`](../../conform/README.md); this page covers the Rust API.

Everything that touches the original needs the owner's disks and Kickstart. When one is missing, the
call returns `Err` naming the path. Tests run from the crate directory, so `cases/x.toml` resolves
under `engine/conform/` (`Case::load` falls back to `conform_dir()`).

## Start here

1. **Load and run a case on the engine** (`tests/level_entry.rs`):
   `let v = run(&Case::load(Path::new("cases/w1-idle.toml"))?)?; assert_eq!(v.outcome, Outcome::Same, "{v}");`
   `run` picks the title or level runner and builds `Game::new(game_data()?, Start::Level1)` itself.
2. **Run a case on a `Game` you keep** (`tests/camera_cases.rs`):
   `run_with(&case, &mut game, &Options::default())`. Use `engine_for(&case, data)` when the case may
   freeze; it freezes the `Game` only if `case.freeze`.
3. **Read the original at a case's sync point** (`tests/camera_cases.rs`): `reach_case(&c, 0)` returns
   the `Amiga` there, with `Inputs::load(&c)?.at(frame)` for each later step's input.
4. **Judge a routine port** (`tests/camera_routines.rs`):
   `let mut r = Rig::new()?; let g = generators::generator("h0_51fc").unwrap();`
   `judge::check_regs(&mut r, name, 10_000, generators::h(0, off), &g, regs, port)`. Here `port` is a
   `fn(&mut judge::Cpu)`. An engine port loads typed state from `cpu.mem`, runs, and stores it back.
5. **Survey a generator before any port exists**: `r.survey(name, cases, entry, &g)` and
   `r.coverage(...)` for branches and bytes read (`tests/gen_camera.rs`, `src/bin/fd-judge.rs`).
6. **A test engine that is the original**: `tests/common/mod.rs` `StandIn` implements `Engine` with
   a second oracle, and can be made to differ in one way (`bump`, `paint`, `event`).

## The public API by job

**Cases** (`src/case.rs`, re-exported at the root)
- `Case::load(&Path) -> Result<Case, String>`, `Case::parse(name, text)`, `case.is_level()`.
- `Case` fields: `sync_frame`, `frames`, `input`, `vars`, `goal`, `exclude`, `freeze`, `thaw`, `set`, `swap`.
- `Goal::{All { all, by_frame }, Idle, GameOver { by_frame }, LevelEnd { by_frame }}`; `Compare { vars, picture }`; `Exclusion { rows, cols, var, what, task }`.

**Running and verdicts** (`src/lib.rs`)
- `run(&Case) -> Result<Verdict, String>`: title or level; the default options.
- `run_with(&Case, &mut dyn Engine, &Options) -> Result<Verdict, String>`: level cases only.
- `run_inputs(&Case, &Inputs, &mut dyn Engine, &Options)`: the same on inputs held in memory (bug zips).
- `engine_for(&Case, Arc<GameData>) -> Game`.
- `Options { mode, register, diff_dir }`: `Default` is `Mode::Faithful`, `fd_core::PORTED`, no diff PNGs.
- `Verdict { outcome, passes, pictures, unseen, tolerated, cut_redraws, edge_cuts, excluded, events, goal_met, game_over, level_end, coverage }`, `.passed()`; `Display` prints the report.
- `Outcome::{Same, Differs, PictureDiffers, ComparedNothing, GoalMissed, NotIdle, Unported, ExclusionUnused, SceneDiffers}`. An unused exclusion fails the case.
- `trait Engine: Transplant` (`pass`, `picture`, `events`, `freeze`, `thaw`, `off_play`, `game_over`, `level_end`, `cut_redraw`, `foreground_cut`); `Game` implements it.

**Reaching the original** (`src/lib.rs`, `src/sync.rs`)
- `reach_case(&Case, slack) -> Result<Amiga, String>`: at the case's sync frame, frozen and `set` applied.
- `reach_case_counting(&Case, slack, extra: &[u32]) -> Result<(Amiga, PassInputs), String>`: also pairs passes with their inputs.
- `Inputs::load(&Case)`, `Inputs::with_after(&Case, InputScript)`, `inputs.at(frame) -> FrameInput`.
- `PassInputs::{anchor, pcs, step, drop_through, next_read, next_start, take}`: a pass's input is not its step's (T67).
- `sync::reach_level_1(frames)`, `sync::reach_level_1_in(kickstart, disk_dir, cache, frames)`, `sync::LEVEL_1_SYNC` (6301), `LEVEL_1_SCRIPT`, `cache_dir()`, `level_1_key`, `level_1_script()`.
- `OracleMem(&AmigaMemory)` and `Live(&Amiga)` implement `fd_core::Mem`, so `.value(var)` reads a `LAYOUT` variable.
- `address(name) -> Result<u32, String>`, `conform_dir()`, `interrupted_pc`, `redraw_cut`, `edges_cut`, `edges_cut_at` (T74: where a pass cut `main_loop`'s edge redraws), `records_edge_cuts`, `Sampler` (which cases and steps the runner samples for it).
- Constants: `HUNK_BASES`, `PLAY_WINDOW`, `STATUS_ROWS`, `PASS_END` (for `am.watch_write`), `GAME_OVER_SCREEN`, `HIGH_SCORES`, `NEXT_WORLD`, `READ_JOYSTICK`, `PAUSED_BRANCH`, `AFTER_NEXT_WORLD`.

**Data** (`src/lib.rs`, `src/disks.rs`)
- `game_data() -> Result<Arc<GameData>, String>`, `game_data_in(&Path)`; `disks::load()`, `disks::read(dir)` return the two ADFs.

**Routine rig** (`src/routine.rs`)
- `Rig::new()`, `Rig::from_program(exe)`, `Rig::from_memory(mem)` for hand-assembled tests.
- `rig.check(name, cases, entry, &Generator, port: impl FnMut(&Setup) -> Effects) -> Result<RigReport, RigFailure>`; `check_regs` adds `RegsCompared`.
- `rig.survey(...) -> RigReport` (original alone); `rig.coverage(...) -> routine::Coverage` (`branches`, `reads`, `.uncovered()`).
- `rig.memory_for(&g)`, `set_max_instr`, `last_regs`, `name(addr)`; `seed(name)`.
- `RegsCompared::{ALL, NONE, d(&[usize])}`: an engine port names only the registers a caller reads.
- `RigReport { passed, paths, hangs, faults, unmodelled_blits }`. A hang or fault never passes, and a check that compared nothing fails.
- `Setup { d, a, pokes, hw_reads, beam }` with `put`, `b`, `w`, `l`, `byte`, `Setup::random(&mut Gen)`; `Generator { base, case }`; `Effects { regs, writes, hw_writes }`.
- `h(hunk, off)`: the address the oracle loads `hN+$off` at. Also `STACK`, `STACK_AREA`, `MAX_INSTR`, `PROGRAM`.

**Judge** (`src/judge/`)
- `judge::check` and `judge::check_regs` take a `port: fn(&mut Cpu)` and wrap `Rig::check_regs`.
- `Cpu { d, a, mem }`; `Mem` has `rb/rw/rl/wb/ww/wl`, `ram`, `changed_since`, `hw_writes`. Hardware follows the oracle's hardware mode.
- `judge::port::port` is the routine built from `FD_PORT` by `build.rs`. Without that variable it is a stub that changes nothing.

**Generators** (`src/gen/`, the module `generators`)
- `generators::generator(name) -> Option<Generator>` and `generators::all()` (name is the `re/symbols.toml` name or `hN+$off`).
- One module per subsystem (`camera`, `character`, `objects`, ...), each with `pub const ALL`. A new module is appended to `MODULES`. `candidate` comes from `FD_GEN` and is listed first.
- `character::COVER` / `character_anim::COVER` and their `cover(name)` record what each routine reads and its branches.

**Coverage and the ported register** (`src/coverage.rs`)
- `Coverage { hits }`, `Coverage::new(register)`, `.add`, `.unreached()`; `coverage::address(&Ported)`. The register itself is `fd_core::PORTED`. This `Coverage` is not `routine::Coverage`.

**Freeze** (`src/freeze.rs`)
- `FROZEN`, `named(name)`, `for_case(thaw)`, `apply(&mut Amiga, &[Frozen])`.

**Bug zips** (`src/bug.rs`)
- `open_zip(bytes, build)`, `replay(data, &rec, each) -> Replayed`, `retime(&rec) -> Retimed`.
- `compare(data, &rec, diff_dir) -> Result<Compared, String>` and `compare_with(..., before_step)`.
- `Compared::{first, left, same, report, case_text}`, `default_out(zip)`, and `screenshot_differs`, `picture_png`, `frame_png` re-exported.

## Commands

| Command | Does |
|---|---|
| `cargo run -p fd-conform -- run <case.toml>` | One case; exits 0 only when it passes |
| `cargo run -p fd-conform -- bug replay <zip> [--out DIR]` | Replay a bug zip on the engine, check the screenshot |
| `cargo run -p fd-conform -- bug compare <zip> [--out DIR]` | Retime the zip onto the original and compare it; writes `bug-case.toml` |
| `cargo run -p fd-conform --bin fd-judge -- 'hN+$off' --gen <name> [--cases N] [--survey \| --cover]` | One routine against the built-in port; prints one JSON line |

Output formats: the `//!` headers of `src/main.rs` and `src/bin/fd-judge.rs`.

## Where the details are

- Case format, CLI, how a level case compares, coverage report: [`engine/conform/README.md`](../../conform/README.md).
- The rig, generators and port register as a method: `toolkit/docs/porting-routines.md`.
- The translation service and the generator records: `games/forest-dumb/docs/11-translation.md`.
- Pass timing, the display window, the camera: `games/forest-dumb/docs/09-play-measured.md`.
- Bug zip procedure: `games/forest-dumb/docs/BUG-FIX.md`.
