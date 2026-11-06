# Fixing a bug the owner recorded

The procedure a session follows when it is handed a bug zip ("fix the bug in
`games/forest-dumb/bugs/<zip>`"). The owner played the app from Game ▸ Start Level… with the
recorder on, pressed Stop where something looked wrong, typed a note and saved one zip. The zip is
the whole report: the owner does not describe the bug further and does not re-test the fix. The
session's evidence is the headless replay and the original running on the oracle. Design:
`plans/forest-dumb-level-1/DESIGN.md` §2.8, §2.12; what "faithful" means: §2.1 there and
`knowledge/self-testing-faithful-bar.md`.

## Before you start

- **Where the zip is.** The app saves into the main checkout's `games/forest-dumb/bugs/`
  (git-ignored, so not in a worktree), or `~/Documents/Forest Dumb Bugs` when it ran with no
  checkout. From a worktree: `"$(git rev-parse --path-format=absolute --git-common-dir)/../games/forest-dumb/bugs/<zip>"`.
  Below, `$ZIP` is that absolute path. Never commit the zip or anything rendered from it: only a
  case and its script, which are inputs.
- **What is in it.** `recording.txt` (build, level, frames, `stopped-at`, the owner's `note`, the
  inputs one a step) and `screenshot.png` (the frame Stop was pressed on). Read the header first:
  `unzip -p "$ZIP" recording.txt | sed -n '1,/^inputs$/p'`.
- **The zip is tied to its build.** `bug replay` and `bug compare` refuse a zip whose `build` line
  is not this engine's (DESIGN §2.12) and name both builds. Work from the commit the build line
  names (its hash in brackets) until the case of step 5 exists: after the fix, the case is the
  check, not the zip.
- **Needs** the disks in the main checkout, `toolkit/tools/setup/oracle.sh` run and Kickstart 1.3,
  as every `fd-conform` run does (`engine/conform/README.md`). Every command below runs from
  `games/forest-dumb/engine/`.
- Read `plans/forest-dumb-level-1/FINDINGS.md` before touching a mechanic.

## 1. Replay it

```sh
cargo run -q --release -p fd-conform -- bug replay "$ZIP" --out /tmp/bug
```

It prints `frames N`, `stopped-at`, `replay-at` and `screenshot equal|differs (P pixels)`, and
writes the last frame as `last.png`. On the build the recording was made with, expect `screenshot
equal` and `replay-at` equal to `stopped-at`. A difference here, before any fix, means the replay
is not the owner's game: stop and report that to the owner, with both lines.

## 2. Compare it with the original

```sh
cargo run -q --release -p fd-conform -- bug compare "$ZIP"
```

The original is started at W1-1's level sync point and fed the recording **pass by pass**: the
app runs one engine step a frame and each step is one pass of `game_frame`, which reads the
joystick once, so each of the original's frames gets the joystick of the pass that reads next and
the keys of the pass that starts next. The engine side is the owner's game: `Game` stepped on the
recording's own inputs. Both are judged at every pass end (every play variable) and every frame
(the picture, within the owner's tolerance), as a level case is. The first line is one of:

- `same`: nothing differed, to the recording's end.
- `differs at frame F (engine step S): <what>`: the original's frame F, the recording's step S
  (the app's frame, counted from 0), and the first variable or picture that differs. A picture
  difference also writes its diff PNG to the output folder.
- `same up to frame F (engine step S), not judged after it`: the original left play there (game
  over, the level's end, a load) and the engine did not, or nothing could be judged. Nothing after
  it was compared.

It writes to `build/bugs/<zip stem>/` (or `--out DIR`): `original-stop.png` and `engine-stop.png`
(the pictures at Stop, or where judging ended), `original-inputs.txt` (what the original was fed,
in its frames from the sync point) and `bug-case.toml` (the case that runs it, step 5). Exit 0
only on `same`. About 20 s for 4 000 steps, release build, warm cache.

Read the rest of the report before trusting the first line:

- **`lag: on N of M passes the original read other input than the recording's`**: where two of
  the original's passes read the joystick in one frame, the second reads the first's input. The
  engine still plays what the owner pressed, so a difference at or after such a step may be the
  lag's, not the engine's. Measured: 2 of 4 000 passes in a varied recording; none on short ones.
- **`the engine's own start differs from the original's at the first pass end`**: the engine
  runs from `Game::new`, as the app starts it, and since T69 the report says the start equals the
  original's. If it ever says it differs, a difference in the first few frames may be the start's,
  not the recording's: say so in the report.
- **`not judged yet: ... (T23)`, `(T24)`**: what the engine does not model yet (the countdown and
  its digits, Esc and P, `hits_taken`). A bug in one of these is a missing feature, not a quirk:
  name its task.
- **The original is not frozen.** Its creatures and collectibles are there; the engine has none
  until T22 and T24. A difference where one comes into view is the missing feature.

## 3. Same on both: a quirk of the original, kept

If the original does the same thing at the same point, the engine is faithful and the behaviour
stays. Commit nothing. Report to the owner in plain words:

```
The original does this too, so I have left it as it is.

What happens: <one or two sentences>.
When: <W1-1, the spot, and the frame, e.g. "frame 1 200 of 4 000 (24 s in)">.
The pictures at the moment you pressed Stop, the original's and ours:
  <path>/original-stop.png
  <path>/engine-stop.png
<Any caveat from step 2: lag, a gap not judged yet.>
```

Only say "the original does this too" when the first line is `same` (not `same up to …`) and both
pictures show the same place.

## 4. Different: find the first difference, then fix the engine

The `differs at` line names where the two first part. Fix the engine where its routine differs
from the original's code (`docs/07-executable-map.md`, `re/symbols.toml`, Ghidra), following
`toolkit/docs/porting-routines.md`, not by fitting the numbers. A routine in the port register
(`fd-core/src/ported.rs`) is proved on the routine rig too: a fix adds the state that shows the
bug to its generator.

## 5. Keep it as a case that fails before the fix and passes after

Before the fix, on the recording's build, copy what step 2 wrote:

```sh
cp build/bugs/<stem>/original-inputs.txt conform/scripts/bug-<short-name>.txt
sed 's/bug-NAME/bug-<short-name>/' build/bugs/<stem>/bug-case.toml > conform/cases/bug-<short-name>.toml
cargo run -q --release -p fd-conform -- run cases/bug-<short-name>.toml
```

The case is unfrozen, compares what `bug compare` did and excludes only the gaps that differed
here; its goal is where the original's character was at the last pass end judged. It must fail at
the difference step 4 found. Run it again after the fix: it must pass. Keep both outputs for the
commit message. The case's engine is transplanted at the first pass end and stepped on the inputs
the original read, so it is the recording's game except at lagged passes (step 2).

Add it to `engine/crates/fd-conform/tests/bug_cases.rs` (create it with the first bug: one test a
case, `fd_conform::run(&Case::load(...))`, asserting `passed()`), and label the file in
`smoke.toml` (smoke if it runs alone in 10 s, else full). Run the tests of the parts the fix
touched.

## 6. Review, and the `Escape:` trailer

The fix gets a fresh review, by a session that did not write it. The fix commit ends with a trailer
naming what the bug escaped from: the task that built the mechanic, found from `git log` and blame
on the lines fixed:

```
<subject, in the run's own form>

<what was wrong, the fails-before and passes-after runs>

Escape: forest-dumb-level-1 T{nn}
```

`Escape: unknown` when the origin cannot be traced. Never `Escape:` without a name.

## 7. A difference the case cannot show

If the replay shows the problem but the comparison cannot (`same up to` a frame before it, a lagged
pass just before it, or a gap not judged yet), do not force it into a case or
call it a quirk. Report to the owner what the replay shows (frame, picture path, one sentence),
what the comparison could and could not see, and why. Commit nothing unless the owner says how to
go on.
