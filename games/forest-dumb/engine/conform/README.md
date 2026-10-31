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
