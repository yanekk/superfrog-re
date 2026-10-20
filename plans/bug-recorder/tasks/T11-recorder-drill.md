# T11 — recorder-drill

**Phase:** 2 · **Depends on:** T07, T08, T09, T10 · **Weight:** medium

## Goal

A worker uses the whole recorder as the owner will, at every size, judges it against DESIGN and the
prototype, fixes what has one right answer with a test each, and brings the owner only the choices
with two defensible answers, before the owner records the acceptance bug on it.

## Design sections this implements

DESIGN §2.1–§2.7, D1.

## Environment (the worker owns this)

```
cd engine && cargo build --release -p sf-app
target/release/superfrog --home <tmp> --bugs-dir <tmp>/bugs   # each run under --timeout 600
rm -rf <tmp>                                                   # teardown; confirm no superfrog process remains
```

## End to end (the worker drives this)

- suite: `sf-e2e` OS layer, screens saved to the task's scratch folder, never the repo · sizes: 1×, 2×, 3×, widescreen, fullscreen
- [ ] Start Level → play W1-1 to its exit → tally → slot machine → W1-2 → STOP → save with a note → `--replay` the zip → `sf-conform bug replay` says equal
- [ ] every ending action from the menu and its shortcut, with each answer
- [ ] the unlisted things a person would try: Esc and Return in each window, a very long note, a pasted note, resizing while recording, switching to fullscreen while the bar shows, ⌘. while paused, ⌘. in the save window, starting a level while the dialog of a previous one is half filled
- [ ] a recording of at least 20 minutes saves and replays (size and time noted)

## Done when

- [ ] every item above driven, each defect with one right answer fixed with an end-to-end test that fails without it
- [ ] a dated worker-driven 🐞/📌 row in FINDINGS saying what was driven and seen; two-answer choices asked of the owner
- [ ] the test command is green
