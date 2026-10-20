# Fixing a bug the owner recorded

The procedure a session follows when it is handed a bug zip ("fix the bug in `bugs/<zip>`"). The
owner played the app from Game ▸ Start Level… with the recorder on, pressed STOP where something
looked wrong, typed a note and saved one zip. The zip is the whole report: the owner does not
describe the bug further and does not re-test the fix. The session's evidence is the headless
replay and the original running on the oracle. Design: `plans/bug-recorder/DESIGN.md` §2.10;
what "faithful" means: `plans/rust-engine/DESIGN.md` §2.1.

## Before you start

- **Where the zip is.** The app saves into the main checkout's `bugs/` (git-ignored, so it is not
  in a worktree): `"$(git rev-parse --path-format=absolute --git-common-dir)/../bugs/<zip>"`. Below,
  `$ZIP` is that absolute path. Never commit the zip or anything rendered from it (RE §2.3): only a
  case and its script, which are inputs.
- **What is in it.** `recording.txt` (the start, the inputs, `stopped-at` and the owner's `note`)
  and `screenshot.png` (the frame STOP was pressed on). Read the header first:
  `unzip -p "$ZIP" recording.txt | sed -n '1,/^inputs$/p'`.
- **Needs** the disks in the main checkout, `tools/setup/oracle.sh` run and Kickstart 1.3, as every
  `sf-conform` run does (`engine/conform/README.md`). Every command below runs from `engine/`.
- Read `plans/rust-engine/FINDINGS.md` before touching a mechanic.

## 1. Replay it

```sh
cargo run -q --release -p sf-conform -- bug replay "$ZIP"
cargo run -q --release -p sf-conform -- bug replay "$ZIP" --out /tmp/bug --frames 1200..1500 --every 5
```

It prints `frames N`, `stopped-at`, `replay-at` and `screenshot equal|differs (P pixels)`, and
writes the last 250 frames (or the ones asked) as PNGs to `~/.cache/superfrog-re/bugs/<zip stem>/`
(never the repo). On the engine the recording was made with, expect `screenshot equal` and
`replay-at` equal to `stopped-at`; a difference here, before any fix, means the replay is not the
owner's game: stop and report that to the owner, with both lines. Look at the frames before STOP
until you can say in one sentence what goes wrong and on which frame; the note says what the owner
saw, the frames say when. A 20-minute recording replays in about 40 s.

## 2. Compare it with the original

```sh
cargo run -q --release -p sf-conform -- bug compare "$ZIP"
```

The original is started at the same level with the same values and fed the same inputs, retimed
onto its own frames, and judged by the faithful checks in legs of 8 passes. The first line is
`same` or `differs at frame F (engine step S): <what differs>`. It writes `original-stop.png` and
`engine-stop.png` beside the zip. Exit 0 same, 1 differs. About 2½ minutes for a 7 500-frame
recording.

Read the rest of the report before trusting the first line:

- **`the original left the recording at frame L`**: the original lagged and stopped playing what the
  owner played (its frog, score, lives, hearts or level no longer the recording's). After L the
  legs judge the engine along the original's own path, not the owner's. If L comes before the
  frames where the problem shows, the comparison has not looked at the problem: look at
  `original-stop.png` beside `engine-stop.png`. If they do not show the same place, `same` says
  nothing about this bug; go to step 7. (On a played W1-1 the original leaves at about frame 1815
  today; plan `bug-recorder` T14 is the work that keeps it on the recording.)
- **`re-synced from it after a lag N times`**: after each lag the engine is put back on the
  original's state, so a difference made inside or just before a lag is not seen. If the problem
  falls in such a stretch, a `same` there is weak; say so in the report.
- **Easy difficulty** (`difficulty easy` in the header) has no conformance cases yet: a difference
  there may be the first time anyone compared it. Say so in the report.

## 3. Same on both: a quirk of the original, kept

If the original does the same thing at the same point, the engine is faithful and the behaviour
stays (RE §2.1). Commit nothing. Report to the owner in plain words, in this shape:

```
The original does this too, so I have left it as it is.

What happens: <one or two sentences, e.g. "after the slot machine the screen stays black for
about 3 seconds before the world map">.
When: <where and the frame, e.g. "W1-1's slot machine, frame 4 600 of 7 487 (1 min 32 s in)">.
The pictures at the moment you pressed STOP, the original's and ours:
  <path>/original-stop.png
  <path>/engine-stop.png
<Any caveat from step 2: lag re-syncs over the stretch, easy difficulty.>
```

Only say "the original does this too" when step 2's checks hold: the original was on the
recording over the problem, and both pictures show the same place.

## 4. Different: find the first difference, then fix the engine

The `differs at frame F (engine step S): variable <name> …` line names the original's frame F, the
recording's step S fed there, and what differs first (a variable, a pixel, a sound event or a
milestone); the full divergence follows. For the frame-exact view:

```sh
cargo run -q --release -p sf-conform -- --strict bug compare "$ZIP"
```

`--strict` differs at frame 0 on `playfield_clean` on every Start Level start (a known residue of
the start, plan `bug-recorder` FINDINGS, T03/T05): ignore that line and read the next one. To see
what the original's routine does around F, make the case (step 5) and use `sf-conform record` on it
with `--watch` on the variables involved (`engine/conform/README.md`).

Fix the engine where the routine differs, against the original's code (`docs/08-executable-map.md`,
Ghidra), not by fitting the numbers.

## 5. Keep it as a case that fails before the fix and passes after

```sh
cargo run -q --release -p sf-conform -- bug case "$ZIP" bug-<short-name>
cargo run -q --release -p sf-conform -- run conform/cases/bug-<short-name>.toml
```

`bug case` writes `conform/cases/bug-<short-name>.toml` and `conform/scripts/bug-<short-name>.txt`
(the inputs in the original's frames; the owner's note on the case's first line). Make it before
the fix and run it: it must fail at the difference step 4 found. Run it again after the fix: it
must pass. Keep both outputs for the commit message. The case is run whole, not in legs: on a
lagging level it may need a `lag-limit`, or the script cut before the original leaves the
recording (`engine/conform/README.md` § Comparing a bug zip).

Add it to `engine/crates/sf-conform/tests/cases.rs`:

```rust
case!(bug_short_name, "bug-short-name");
case!(#[ignore = "slow: about 4 min"] bug_short_name, "bug-short-name");   // if it is long
```

Run the tests of the parts the fix touched, and `cargo test -q -p sf-conform --test cases`.

## 6. Review, and the `Escape:` trailer

The fix gets a fresh review, by a session that did not write it. The fix commit ends with a trailer
naming what the bug escaped from: the task of the finished plan that built the mechanic, found from
`git log` and blame on the lines fixed:

```
<subject, in the run's own form>

<what was wrong, the fails-before and passes-after runs>

Escape: rust-engine T{nn}
```

`Escape: unknown` when the origin cannot be traced. Never `Escape:` without a name.

## 7. A difference the case cannot show

If the replay shows the problem but the comparison cannot (the original left the recording before
it, it falls inside a re-synced lag, or the case runs green while the replay still shows it), do not
force it into a case or call it a quirk. Report to the owner what the replay shows (frame, picture
path, one sentence), what the comparison could and could not see, and why. Commit nothing unless
the owner says how to go on.
