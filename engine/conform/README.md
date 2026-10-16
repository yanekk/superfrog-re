# Conformance: scripts and cases

The engine is judged against the original running on the oracle (DESIGN §2.1, §4). A **case**
brings both to a **sync point**, feeds both the same **input script** frame by frame, and judges
them by DESIGN §2.1's faithful checks (below): the modelled variables (`LAYOUT` in
`sf-core/src/layout.rs`) pass by pass, the picture in the display window within a tolerance, the
sound as events, and the engine run alone meeting the original's milestones. The runner stops at
the first check that fails and says which variable, which pixel (with a diff PNG), which sound
event or which milestone. `--strict` judges frame-exact instead, for diagnosis.

Everything here is input, never disk data, so it is committed. Diff images and the cached sync
points are derived from the disks and live outside the repo (temp dir, `~/.cache/superfrog-re/sync/`).

## Running

From `engine/`:

```sh
cargo run -q --release -p sf-conform -- run conform/cases/title-rng.toml   # one case (or several)
cargo run -q --release -p sf-conform -- all                                # every case here
cargo run -q --release -p sf-conform -- --strict run conform/cases/title-rng.toml  # frame-exact
cargo run -q --release -p sf-conform -- run --alone conform/cases/complete-w1-2.toml  # check 4 only
cargo run -q --release -p sf-conform -- record conform/cases/title-rng.toml --watch blink=h0+1368:W
```

`run`, `all` and `scan` judge faithfully; with `--strict` frame-exact (and without check 4).
`run --alone` judges by check 4 only: checks 1–3 end a run at their first failure, so a case red
there would never reach it. `run` and `all` exit 1 on a divergence. `record` runs only the original and prints, for each frame
on which something changed, the input held, its pacing (`OnTime`/`Late`, below), every `LAYOUT`
variable and each `--watch` address (`h0+<hex>` is the game's hunk 0, `0x<hex>` a raw address;
width `B`, `W` or `L`; `--every` prints every frame). It is how a script is steered: watch the
variables the routine you are porting uses, and find the frames where they do what you need.

`record` leaves out `LAYOUT` entries over 1 KB (the playfield buffers, the bob sheet, the level
map, the object slots: its header line names them); `--all-vars` puts them back, `--watch` reads
one word of them. It stops with an error once it has written 200 MB of lines and PNGs; `--max-mb N`
moves the cap. Both exist because a whole-memory recording of a long case reached 38 GB, filled the
disk and crashed the parallel run (2026-10-15). Prefer `--only` with `--watch` and a short case.

Needs what the oracle needs: `tools/setup/oracle.sh` run, the four disks and Kickstart 1.3
(DESIGN §5). The first run to a sync point takes ~10 s per 4 000 frames; later runs restore the
cached snapshot.

## The faithful checks (DESIGN §2.1 checks 1–3, T180)

1. **State, pass by pass.** Every `LAYOUT` variable but the `presentation` ones is compared at the
   end of each main-loop pass, the original's `n`th pass with the engine's `n`th: in a level the
   25 Hz pass, elsewhere a frame (`sf_core::PassClock`, the same rule on both sides). A pass whose
   end falls a frame later on one side is compared with each side's state at its own pass's end,
   the side ending first held (`compare::PassMatch`); a pass whose end is not seen (the next one
   began and was still running at the frame's end) is not compared, and sides more than a pass
   apart fail. Frames ending mid-pass are left out, and a lag re-synced, as under Pacing below; a
   case with `resync = false` fails there on a gameplay variable only.
2. **Picture, within tolerance.** A frame whose differing pixels are all at most one step off in
   each of R, G and B, or fewer than 64 pixels, is tolerated however long it lasts. Any other
   difference (a differing display window among them) is tolerated for at most 2 frames in a row,
   and on at most 1% of the case's frames (a case under 100 frames has none). Beam races are
   excused first, as under Beam races. A `render` comparison stays exact.
3. **Sound, as events.** Each frame's Paula writes become, per channel, a sample started (its
   address and length as the DMA latched them; the period and volume the frame left), a period or
   volume change, a channel stopped (`sound_events`). The volume is the one Paula plays (low 7
   bits, 64 and above as 64). Each of the original's events must be met by the same event on the
   same channel within ±2 frames, in the same order per channel, and the engine may make none the
   original does not. A period or volume change followed by its channel's stop within 2 frames
   need not be met (the stop that much sooner would have left it unheard). All frames' writes
   count, a lag's among them; what the original plays while it loads is not compared, and the
   engine's Paula is left as the original's when a load ends or a lag re-syncs.

The thresholds are `Tolerance::OWNER`, the owner's bar: not a case setting, and never changed to
make a case green. Nothing is tolerated silently: `run`, `all` and `scan` print per case the
re-syncs (with the lag, as before), then `tolerated:` the picture frames by kind (one-step,
few-pixel, short-glitch), the sound events matched a frame or more apart, those cut off by the
case's end and the changes before a stop left unmet, and the frames on which the passes were a
frame apart with how many states were compared so.

`presentation` variables (`sf_core::layout::presentation`, each with its reason) only say how a
frame was produced; checks 2 and 3 judge them: the copper lists (`end_copper`, `award_copper`,
`tally_copper`, `slot_copper`, `exit_sprite_copper`, `level_copper`, `front_copper`, and the
intro's `StoryBootCopper` and `StoryCopper`), the bitmaps (`bob_sheet`, `playfield_buffer_0`, `_1`,
`playfield_clean`, `hunk3_gap`, `panel_bitmap`, `hero_frames`, the intro's text page, buffers and
clean background), `shown_buffer` and `scroll_column` (the shown column, T181), the blit queue
(`blits_pending`, `blit_next`, `blit_queue`) and the sound driver's fade and register shadows
(`music_fade`, the intro's `StoryShadow`). No variable a gameplay rule reads is tagged.

## Plays alone (DESIGN §2.1 check 4, T182)

Every case that steps the engine (all but `render`-only ones) also runs a second engine from the
same start, given only what check 4 allows: each frame's input as the original consumed it (its
load and disk-wait frames skipped), the original's `Pacing`, and the RNG seeds the original read
from its clock (at power-on in the start; `rng_seed_from_cia` at the end sequence's end, handed
over when the engine reaches it). It gets none of the rest the comparison's engine is fed: no
re-sync, no transplant after a load, no beam anchors, handler timings, load lines or the memory
that runs on during a load. Where a load leaves something, it has what its own instant load gives,
as in normal play. A `Pacing::Late` whose cut it never reaches (it made fewer writes than the
original) ends the frame on its own clock, as normal play does; a panic is reported, not raised.

Both sides' milestones are read frame by frame of play (`sf_conform::milestones`) from the same
`LAYOUT` variables by one function, so a derivation fault cannot make them differ:

- **scene**, by the copper list the frame starts with (COP1LC: the original's as its CPU last wrote
  it, the engine's as `Game::cop1lc`): title, award pages, hall of talent, attract demo, a level by
  world and level number, the bonus level, game over (a level's list with the lives below none),
  tally, slot machine, end sequence, and any other list by its address. The blank list a screen
  fades through changes no scene. The story intro is one scene (its memory is not the game's);
- **coin taken** (`coins_collected` up, each coin with the count it left for the exit);
- **item taken** (a marker cleared from `level_map` that no coin or kill explains: fruit, tile
  coins, checkpoints), with the marker and its cell;
- **enemy killed** (a slot's hit points from 0 or more, not immune, to below none, the slot still
  holding the same object), by its init routine;
- **life lost** (`frog_lives` down); **level exit** (`level_state` from negative to 0 or more with
  the frog not dying); **tally** (with the score as it begins); **end** (score, lives, coins at
  the case's last frame).

The engine's milestones must equal the original's, in order (milestones within 10 frames of each
other in either order), each within ±10 frames of play (`MILESTONE_FRAMES`, the owner's bar).
A failure names the first milestone that parts, and the first frame at which the engine alone
ended a pass with a gameplay variable unlike the original's: where to look for the cause.
`SF_CONFORM_MILESTONES=1` prints a green run's milestones.

Cases red alone are listed in FINDINGS with their owner; a test that runs one keeps checks 1–3
with `case.alone = Alone::Off` and names the owner (T184: `enemies-w1-1`, `enemies-w3-1`,
`enemies-w3-activation`, `enemies-w5-2-complete`, `front-attract`).

## Which cases the test block runs (T181)

Every case green under the faithful checks has a test in `crates/sf-conform/tests/`: those with
assertions of their own beside their subject (`frog_cases.rs`, `enemies_*.rs`, `front.rs`,
`story.rs`, ...), the rest one line each in `cases.rs`. The slow ones are ignored and run with
`cargo test -p sf-conform -- --ignored`: `attract-demo-3` (`cases.rs`, about 4 min) and the
intro's `story-scene1-out`, `story-scene4-init`, `story-scene4-sound` and `story-intro-full`
(`story.rs`, 80 s to 5 min), and the final level's `final-win`, `final-lose` and `final-idle` (`bosses.rs`,
which runs the win to the level's end in the block). A case red today has no test that runs it whole (a test may keep
checks 1–3 with `Alone::Off`); its FINDINGS row names the first failing check, frame and cause and
the task that owns it: `complete-w4-2`, `enemies-w1-1`, `enemies-w3-1`, `enemies-w3-activation`,
`enemies-w5-2-complete`, `enemies-w6-1-complete`, `front-attract` (T184), `enemies-w6-4` (T185).

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
compare = ["vars"]                                  # any of "vars", "picture", "paula" (sound), "render"; default all but render
resync = false                                      # a re-sync after a lag fails the case (default true; see Pacing)
lag-limit = { frames = 133, note = "why; the task" } # optional: this case's limit on frames in a row mid-pass (see Pacing)
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
- `sync = { game-over = "<script>" }`: game over after the last life (T34): W1-1 reached as
  `{ level = "W1-1" }` reaches it, `frog_lives` set to 0 on its first frame, the script played
  until the frog's death ends the level's loop into `game_over_seq`. The level is transplanted at
  the end of the frame before, so the case's first frame is the one game over begins in. With
  `score = N` beside it, `score` is set to N with the lives (T200: a new high score).
- `sync = { end-sequence = true, score = N }`: `"end-sequence"` with `score` then set to N (T200:
  the game's end with a new high score).
- `sync = { level = "W1-1" }`: the first in-level frame (in the final level, `WW-1`, every disk the
  original asks for goes in, so it plays on into the end sequence), reached the way a player would: W1-1 by
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
- `picture` with `frames = [from, to]`: those frames' pictures are not compared. Only for a
  difference the owner accepted as not to be modelled (`task = "none (accepted)"`, the date in
  its note): `story-intro-full` 7237–7241 is the one (T181).

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
memory and reports both (`lag … re-synced at …`). Such frames are rare; `frog-run-w1-1` has two. A
case with `resync = false` (the `items-*` cases) fails there instead: a re-sync loads the original's
whole memory and so erases any divergence that lands on a lag frame (T14 review, T36 review).

An original that never finishes its pass would leave every frame out and the case green comparing
nothing (T120). So a case fails once more than `LAG_LIMIT` (100) frames in a row end mid-pass;
the frame after each lag ends the run, as its variables are compared. 100 is fitted to the cases
(2026-10-15): all but two have runs of 97 or under. Those two, `complete-w4-3` (261) and
`enemies-w6-1` (133), are pinned at their run with `lag-limit`, each with its reason (the
original's slowdown, which the run alone still covers). `run`, `all` and `scan` print each case's
frames compared, its longest mid-pass run, its longest run left out (the frames after each lag
among them, not limited) and the frames the original was late (`Outcome::late`: the slowdown
normal play does not reproduce, DESIGN §2.1; per level in docs/08 "Lag").

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
