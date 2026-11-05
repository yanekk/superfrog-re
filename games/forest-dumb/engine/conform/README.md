# Conformance: scripts and cases

The engine is judged against the original running on the oracle (plans/forest-dumb-start DESIGN §2.8,
§3.4). A case boots the original from power-on to a sync point, starts the engine there, feeds both the
same input frame by frame, and compares the modelled variables (`LAYOUT` in
`fd-core/src/title.rs`) each pass. It stops at the first difference and names the pass, the frame and
the two values. One pass of the idle title takes one frame.

Everything here is input, never disk data, so it is committed. Nothing is written or cached.

## Running

Anywhere under `games/forest-dumb/engine/`:

```sh
cargo run -p fd-conform -- run cases/first-case.toml   # one case, prints the verdict; about 35 s
```

Exit 0 for the same on every pass, 1 for a difference or a missing input, 2 for a wrong command line.
The same comparison runs in `cargo test` (`crates/fd-conform/tests/first_case.rs`). It needs the two
zips (`docs/01-disk-location.md`) and the Kickstart (`~/amiga-roms/kick1.3.rom` or `AMIGA_KICKSTART`);
absent, it fails naming where it looked.

## A case

Superfrog's case format, narrowed to what this runner does. A key or value it does not know is an
error. It is read by a small parser in `fd-conform`, since the plan admits no `toml` dependency.

```toml
sync = { script = "boot-to-title", frame = 4922 }   # scripts/<script>.txt; compared after 4922 frames
swap = { frame = 3600, disk = 2 }                   # optional: this disk into df0 before that frame
frames = 15078                                      # frames run and compared after the sync point
compare = ["vars"]                                  # state only, so far
vars = ["mt_counter"]                               # names in LAYOUT
```

The oracle's frame cap is `sync frame + frames`. A disk change is here and not in the script, because
a script holds only what the player holds (`amiga-conform`'s `script` module has the format).

## This game's runner shape

The first game keeps pictures, sound, re-syncs and a run alone; none of them exists here yet. The hunk
bases in `fd-conform/src/lib.rs` hold for the boot of `boot-to-title`, the only boot so far.
A new variable needs a `LAYOUT` row and an arm in `engine_value`.

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
matches through it, is a case's job (T06's coverage report).
