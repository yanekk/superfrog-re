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
  The engine starts where the game program does and loads at once (owner, 2026-10-12): its first
  frame is compared with the original's first frame after the title's loads, and every later load
  skips the original's frames until its load is over (docs/13 "Where the engine starts").
- A front-end routine's memory that the original's CPU is still writing at a frame's end (an
  unpacking, a long blit) is compared once the original is out of it (`FrameOut::jobs`).
- `sync = { script = "<name>", frame = N }`: the original runs the script from power-on for N frames;
  the engine starts from its memory there (every `LAYOUT` variable transplanted) and its CPU, so a
  front-end routine the engine knows goes on from the same place (`Start::Resume`).
- `sync = { level = "W1-1" }`: the first in-level frame, reached the way a player would: W1-1 by
  START GAME (`scripts/start-w1-1.txt`), any other level of W1–W6 and the bonus level (`"WB-1"`) by
  typing its level code on the options screen, the disks swapped as the game asks (docs/12
  "Reaching every level"). About 30 s the first time; cached after.
- `sync = { slot = "<name>", start = "<name>" }`: the slot machine's first frame. W1-1 is reached
  by the `start` script (`start-w1-1` when left out; `start-w1-1-blink1` and `start-w1-1-late`
  START GAME later, so the RNG has moved 1 and 3 steps: it never moves in W1-1), the `slot`
  script is played in W1-1 to its last line's frame, the level's exit is forced (`level_state` =
  1), up picks the slot machine in the tally and fire goes on every 4th frame. The engine starts
  from the original's memory and CPU on the first frame `slot_machine` runs (`Start::Resume`).
  `scripts/slot-none.txt` plays nothing in W1-1: two coins from its clock go in.

Compare only what the engine models. A case comparing the picture before the renderer exists is red
by construction. A variable is in `LAYOUT` or it is not modelled; to see one that is not, `--watch` it.
`vars = ["rng_state"]` compares only the variables named, for a scene that models only some of
what the original changes there.

## What a case leaves out — `[[exclude]]`

What the original does in a case that the engine does not model yet is listed in the case, with
the task that will model it (DESIGN §2.1):

```toml
[[exclude]]
what = "objects"     # "objects", "enemies", "panel" or "unreached"
note = "the original's object spawns, bump blocks and tile pickups return at once from power-on"
task = "T36"
```

- `objects`: the original's object spawners, bump blocks and tile pickups are made to return at
  once from power-on (`sf_conform::freeze`), and the engine runs with no object kinds
  (`Game::frog_only`): the frog plays the level alone on both sides (a separate cached sync point).
  The frog cases (`cases/frog-*.toml`) run so.
- `enemies`: only each world's marker spawners the engine's dispatch has none for (enemies, the
  world's scenery) and the frog-side spawns no task has ported (`freeze::UNPORTED_SPAWNS`) are
  frozen; the items play on both sides (`cases/items-*.toml`). The sync point is cached per set.
- `panel`: the panel's bitmap and the picture's panel lines are not compared.
- `unreached`: listed only: what the case's input never makes the original do (the restart after
  a death), so there is nothing to freeze.

A `render`-only case does not step the engine at all.

## Pacing

In a level the original's main loop runs a 25 Hz pass every second frame and sometimes overruns it
(docs/08 "Lag"). The runner reads, each frame, whether the original's pass was still running at the
vertical blank and hands that to the engine (`Pacing::Late`), so a late pass is not a divergence by
itself (owner, 2026-10-11). Title-side screens are never late.

A frame the original ends in the middle of a pass (its main loop not parked in
`level_frame_loop`'s wait) holds memory part way through what the engine does in one go. The runner
does not compare it, nor the next frame's picture and Paula writes, which carry the rest of that
pass; on the frame after, if a variable still differs, it re-syncs the engine from the original's
memory and reports both (`lag … re-synced at …`). Such frames are rare; `frog-run-w1-1` has two.

`Pacing::Late { writes, line, hpos, phase, step }` says where the original's frame ended: after its
`writes`th Paula write (the last at beam `line`, slot `hpos`), in the pass's `phase`th call, `step`
slots into its loop; the engine ends its frame at the same point. Each frame the runner also anchors
the engine's beam clock at the original's own positions (owner, 2026-10-13):
`Game::anchor_writes` with where each of the frame's Paula writes was made, `Game::anchor_blits`
with where each blit started (`Amiga::blit_starts`). The engine's fitted costs (`level::beam`)
place its work between anchors; normal play has only the costs. The costs are refitted with
`cargo run --release --example fit -- <case.toml>...`, which prints `Cost::cycles`'s arms; the
`profile` example shows where the original's CPU is, line by line.

## Beam races

The engine draws a step at once; the original's CPU or blitter writes the picture while the beam is
drawing it, so a write landing near the beam shows on one frame or the next depending on a few
cycles. A drawing step that wrote raster lines while the beam was within 4 lines of them
(`sf_core::game::RACE_LINES`, owner 2026-10-13) reports them in `FrameOut::beam_races`. When a
frame's picture differs from the original's, but matches with those lines left out
(`compare::picture_except`), the frame is excused, never silently: it counts in `Outcome::raced`, and
a green run prints `beam races excused at N frame(s)`. A pixel differing on any other line still
diverges.
