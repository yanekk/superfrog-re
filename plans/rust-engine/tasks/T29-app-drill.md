# T29 — app-drill

**Phase:** 3 · **Depends on:** T21, T25, T26, T27, T28 · **Weight:** medium

## Goal

A worker uses the finished app the way a player would, end to end, at every size, judging it against
DESIGN and the original, fixing what has one right answer with an end-to-end test each, and bringing
the owner only the choices with two defensible answers. Then the full conformance suite runs green.

## Design sections this implements

All of DESIGN §2; `pir-e2e` § 3.

## Environment (the worker owns this)

```
temp home: H=$(mktemp -d); cd engine && cargo build --release -p sf-app
run: target/release/superfrog --home "$H" --max-frames <N> --timeout <S> [--scale 1|2|3] [--allow-fullscreen]
teardown: confirm no superfrog process remains (pgrep superfrog prints nothing); rm -r "$H"
```

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 1×, 2×, 3×, 2× widescreen, fullscreen, fullscreen widescreen
- [ ] first run from an empty home: import, intro, title, password, game start.
- [ ] a full scripted game through the app (script layer) from power-on to the end sequence: frames equal the headless whole-game case.
- [ ] every menu item, every shortcut, Controls window, save/load, export/play level file, at each size.
- [ ] unlisted things a player would try: resize mid-scroll, fullscreen during the intro, pause during a slot spin, load a state during a boss, Cmd+Q during import, unplug audio output, very long play (50 000 frames).
- [ ] screens saved to the scratch folder, never the repo.

## Done when

- [ ] `cd engine && cargo run -q --release -p sf-conform -- all` green, run time in PROGRESS notes.
- [ ] Every defect found is fixed with a failing-then-passing e2e test, or is a FINDINGS row; every two-answer choice was put to the owner.
- [ ] A dated, worker-driven 🐞/📌 FINDINGS row saying what was driven and what was seen; teardown confirmed.
