# Conformance: scripts and cases

The engine is judged against the original running on the oracle (plans/forest-dumb-level-1 DESIGN
§2.1, §4). A case brings the original to a sync point, starts the engine there, feeds both the same
input and compares the variables it names (`LAYOUT` in `fd-core/src/layout.rs`) and, for a level case,
the picture. It stops at the first difference and names the pass, the frame and the two values.

Everything here is input, never disk data, so it is committed. The level sync point is cached outside
git (`fd-conform/src/sync.rs`).

## Running

Anywhere under `games/forest-dumb/engine/`:

```sh
cargo run -p fd-conform -- run cases/first-case.toml   # one case, prints the verdict; about 35 s
```

Exit 0 when the case passes, 1 for a failure or a missing input, 2 for a wrong command line. The
verdict is followed by its report: passes and pictures compared, pass ends unseen, frames tolerated,
each exclusion with the frames it hid a difference on, engine events, the frame the goal was reached
at, and the coverage report. It needs the two zips (`docs/01-disk-location.md`) and the Kickstart
(`~/amiga-roms/kick1.3.rom` or `AMIGA_KICKSTART`); absent, it fails naming where it looked.

A level case needs the level engine, fd-core's `Game`, which T07 builds. Until then `run` refuses a
level case, and the tests run level cases through `fd_conform::run_with` with a stand-in engine, a
second copy of the original (`fd-conform/tests/common/mod.rs`, used by `tests/runner.rs` and
`tests/input_timing.rs`).

## A case

A key or value the runner does not know is an error. Read by a small parser in `fd-conform/src/case.rs`,
since the plan admits no `toml` dependency. A value is one line: a whole number (decimal or `0x`), a
quoted string, `true`/`false`, a `[list]` or a `{ key = value }` table.

```toml
sync = { script = "boot-to-level1", frame = 6301 }  # scripts/<script>.txt; the engine starts here
input = "w1-stub"            # optional: scripts/<input>.txt, frames counted from the sync frame
frames = 130                 # oracle frames run after the sync point
compare = ["vars", "picture"]
vars = ["char_x", "camera_x"]                       # names in LAYOUT
goal = { all = [{ var = "char_x", value = 0x1a3 }], by_frame = 6431 }   # or { idle = true }
exclude = [{ what = "rows 194..250", task = "T10" }]                    # optional
freeze = false               # optional
swap = { frame = 3600, disk = 2 }                   # title cases only: this disk into df0 then
```

- **`sync`.** A script of `boot-to-level1` makes a level case: it starts from the cached level sync
  point (frame 6301, disk 2 already in df0, so no `swap`) and runs the script on to `frame`. Any other
  script makes a title case, booted from power-on, stepping forest-dumb-start's title scene a frame
  at a time; title cases compare `vars` only and take no `goal`, `exclude` or `freeze`.
- **`input`.** Without it the sync script goes on after the sync point.
- **`goal`.** Every level case states one; agreement that misses it fails, because two sides failing
  alike agree (porting-routines §4). `all`: every listed variable holds its value at the end of one
  compared pass ending at or before oracle frame `by_frame`, on both sides. `idle = true`, for stretches
  where only state is expected to stay in step: the run reaches its last frame with no engine event and
  every listed variable compared on every pass; a pass whose end the original hides (below) fails it.
- **`exclude`.** Picture rows not compared, each with the task that removes it. Row 0 is line 47, the
  play area's top; the status bar is rows 194 to 249. An exclusion that hid no difference over the whole
  run fails the case: it is no longer needed.
- **`freeze`.** Patches the routines `fd-conform/src/freeze.rs` lists to return at once in the
  original at the sync point. The list is empty until T21 measures the object entry points, so
  `freeze = true` is an error until then.

The oracle's frame cap is the case's last frame. A disk change is in the case and not in the script,
because a script holds only what the player holds (`amiga-conform`'s `script` module has the format).

## How a level case compares

- **At the end of each pass.** In play a pass of `game_frame` usually ends after the frame does (2 977
  of 3 000 moving frames; docs/09-play-measured.md), so the state at a frame's end is mid-pass. The
  runner takes the original's RAM at the pass end, the handler's `INTREQ=$4010` at `h0+$1f02`
  (`Amiga::watch_write`), and numbers passes by the original's own `irq_count`.
- **Pass for pass.** The engine is transplanted (`fd_core::Transplant`) from the first pass end after
  the sync point, then runs each later pass on the input of the step that pass started in. A step now
  and then ends two passes and shows only the first; the other is counted unseen and not compared.
- **A case that compares nothing fails**: no pass ended, no variable or no picture compared.
- **The picture** is `PLAY_WINDOW`, the play area and the status bar together, from the frame the pass
  ended in, judged within the owner's tolerance (`amiga_conform::Tolerance::OWNER`, `PictureJudge`). A
  run made with `Options { mode: Mode::Strict, .. }` fails on any pixel; a case cannot choose its bar.
  `Options::diff_dir` writes a diff PNG beside a picture failure; it shows the original, so it goes
  under `build/`, never into git.
- **A new variable** needs a `LAYOUT` row matching `re/symbols.toml` (a test holds them together), and
  the engine's `Transplant::probe` arm for it.

## The coverage report

Every run counts, from the first step after the sync point, how often the original executed the first
instruction of each routine in fd-core's `PORTED` (T31's `Amiga::watch_pcs`). A registered routine the
case never reaches is listed at zero, as "never reached", never left out. A ported routine with no hits
in any case is not proven by any case (DESIGN §4.1). The counts are in `Verdict::coverage` and printed
after the verdict.

## Routines: the rig, the generators and the port register

A porting task proves each routine it ports on the routine rig before any case runs it. Read
`toolkit/docs/porting-routines.md` first: its rules are what the rig and the register enforce.

- **The rig**, `fd-conform/src/routine.rs`. `Rig::new()` loads the main program from disk 1 at the
  hunk bases above and runs one routine alone on Moira in hardware mode: custom-chip writes recorded
  in order, blits landing in memory, raster waits ending. `Rig::check(name, cases, entry, &generator,
  port)` runs the original and the `port` closure from the same generated states and requires the
  same registers back (d0-d7, a0-a6), the same bytes changed off the rig's stack, and the same
  custom-chip writes in the same order. The closure takes the case's `Setup`, runs the engine
  function on it, and returns `Effects`; `Rig::memory_for(&generator)` is the memory it reads under
  the setup's pokes (`Setup::byte`). States the original hangs or faults on are counted in
  `RigReport::hangs` and `faults`, never passed; a check that compared nothing fails, and
  `RigReport::paths` counts the distinct instruction counts the original took, so a generator that
  never takes a branch shows fewer. `Rig::survey` runs the original alone.
- **The generators**, `fd-conform/src/gen/` (module `generators`; `gen` is reserved in Rust 2024).
  One `Generator` per routine, a base laid once and a function making each case, looked up by the
  routine's name in `re/symbols.toml` (or `hN+$off` until it has one) with `generator(name)`. The
  translation service's judge and an engine port's rig test use the same one. A generator task adds
  its own module with a `pub const ALL` list and one line in `MODULES`. `spike.rs` holds the eleven
  generators of the Muse spike.
- **The port register**, `fd-core/src/ported.rs`. Every routine ported into `fd-core`, by hunk and
  offset, with its Rust function. Porting tasks only append. `fd-conform/tests/ported.rs` fails on
  two entries for one address and on an entry that is not a `func` of the same name, hunk and offset
  in `re/symbols.toml`, so a routine is named there before it is registered.

The rig proves a port on generated states only. That the routine runs in play, and that the engine
matches through it, is a case's job (the coverage report above).
