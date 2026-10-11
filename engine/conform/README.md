# Conformance: scripts and cases

The engine is judged against the original running on the oracle (DESIGN §2.1, §4). A **case**
brings both to a **sync point**, feeds both the same **input script** frame by frame, and compares
what DESIGN §2.1 lists: the modelled variables (`LAYOUT` in `sf-core/src/layout.rs`), the picture
in the display window, and the ordered Paula writes. The runner stops at the first frame that
differs and says which variable, which pixel (with a diff PNG) and which Paula write.

Everything here is input, never disk data, so it is committed. Diff images and the cached sync
points are derived from the disks and live outside the repo (temp dir, `~/.cache/superfrog-re/sync/`).

## Running

From `engine/`:

```sh
cargo run -q --release -p sf-conform -- run conform/cases/title-rng.toml   # one case
cargo run -q --release -p sf-conform -- all                                # every case here
cargo run -q --release -p sf-conform -- record conform/cases/title-rng.toml --watch blink=h0+1368:W
```

`run` and `all` exit 1 on a divergence. `record` runs only the original and prints, for each frame
on which something changed, the input held, its pacing (`OnTime`/`Late`, below), every `LAYOUT`
variable and each `--watch` address (`h0+<hex>` is the game's hunk 0, `0x<hex>` a raw address;
width `B`, `W` or `L`; `--every` prints every frame). It is how a script is steered: watch the
variables the routine you are porting uses, and find the frames where they do what you need.

Needs what the oracle needs: `tools/setup/oracle.sh` run, the four disks and Kickstart 1.3
(DESIGN §5). The first run to a sync point takes ~10 s per 4 000 frames; later runs restore the
cached snapshot.

## Writing a script — `scripts/<name>.txt`

```text
# Power-on to the title screen.
1000: M        # port-1 mouse button: skips the crack intro
1005: -        # nothing held
4600: R F      # right and fire
4610: K19      # raw key $19 (P) held
4612: -
```

- A line `<frame>: <inputs>` sets everything held **from that frame until the next line**. Before
  the first line nothing is held. Frames count from the case's sync point (from power-on for a
  script used as a sync point) and must strictly increase.
- Inputs: `U D L R F` port-2 joystick (up is jump), `M` port-1 mouse button, `K<hex>` a raw key
  code `00`..`7F` (P is `19`, ESC `45`, space `40`), `-` or nothing for nothing held.
- `#` starts a comment.

## Writing a case — `cases/<name>.toml`

```toml
# What this case proves, and why its window is where it is.
sync = { script = "boot-to-title", frame = 4410 }   # run that script from power-on, 4 410 frames
script = "walk-right"                               # optional; default: nothing held
frames = 50                                         # frames compared, from the sync point
compare = ["vars"]                                  # any of "vars", "picture", "paula", "render"; default all but render
```

`render` compares, each frame, the picture the renderer draws from the original's memory transplanted
into a fresh engine (no stepping) with the original's picture: the renderer's own proof while the
mechanics are not ported (`cases/render-*.toml`, one per level). It leaves out, and lists with the
reason, frames that are not a level screen and frames with a late redraw: the original rewrote the
copper list, the panel or the shown buffer after the beam passed it, which an end-of-frame transplant
cannot see (docs/12 "Late redraws"). Stepping cases must still match those frames.

Sync points:

- `sync = "power-on"`: both switch on; the engine gets the RNG seed the original chose on this run.
- `sync = { script = "<name>", frame = N }`: the original runs the script from power-on for N frames;
  the engine starts from its memory there (every `LAYOUT` variable transplanted).
- `sync = { level = "W1-1" }`: the first in-level frame, reached the way a player would: W1-1 by
  START GAME (`scripts/start-w1-1.txt`), any other level of W1–W6 and the bonus level (`"WB-1"`) by
  typing its level code on the options screen, the disks swapped as the game asks (docs/12
  "Reaching every level"). About 30 s the first time; cached after.

Compare only what the engine models. A case comparing the picture before the renderer exists is red
by construction. A variable is in `LAYOUT` or it is not modelled; to see one that is not, `--watch` it.

## Pacing

In a level the original's main loop runs a 25 Hz pass every second frame and sometimes overruns it
(docs/08 "Lag"). The runner reads, each frame, whether the original's pass was still running at the
vertical blank and hands that to the engine (`Pacing::Late`), so a late pass is not a divergence by
itself (owner, 2026-10-11). Title-side screens are never late.
