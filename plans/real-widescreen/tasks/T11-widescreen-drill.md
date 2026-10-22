# T11 — widescreen-drill

**Phase:** 2 · **Depends on:** T10 · **Weight:** medium

## Goal

A worker plays the widescreen game through the real app the way a person would, saves the screens,
judges them against DESIGN §2, fixes what has one right answer with a test each, and brings the owner
only choices with two defensible answers.

## Design sections this implements

DESIGN §2 whole; pir-e2e drill.

## Files

- `engine/crates/sf-e2e/tests/widescreen_drill.rs` (one test per defect found and fixed)
- fixes wherever a defect lies, each with its test
- screens to a scratch folder outside the repo

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 2× widescreen and fullscreen widescreen
- [ ] at least one level of each world, the bonus level and the final level, each ≥ 3 000 frames, with
  scripted play (finishing scripts from `engine/conform/scripts/` where they still fit): strips show
  moving things; nothing pops in visibly inside a strip, vanishes, tears, or shows black.
- [ ] each level's left and right end reached in widescreen: camera stops, frog walks into the strip.
- [ ] switching widescreen on and off mid-level several times, during a fade, while paused.
- [ ] Spud thrown into the strip; bonus shots crossing the strip; the witch fight in widescreen.
- [ ] a recording made in widescreen with switches, replayed.

## Environment (the worker owns this)

```
cd engine && cargo build --release -p sf-app -p sf-e2e   # app runs are bounded by script length
# teardown: every app process the drill started has exited (pgrep -f sf-app empty)
```

## Done when

- [ ] Every item above driven, screens judged; a dated 📌 FINDINGS row (worker-driven) says what was
  driven and seen.
- [ ] Every defect with one right answer fixed with a failing-then-passing test; two-answer ones asked.
- [ ] `cd engine && cargo test -q --workspace` run once after the fixes, recorded.
