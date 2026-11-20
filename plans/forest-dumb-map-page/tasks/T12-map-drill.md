# T12 — map-drill

**Phase:** 2 · **Depends on:** T09, T11 · **Weight:** medium

## Goal

Use the whole thing the way the owner will: the app playing while the page follows it, at a laptop size and
the second-screen size, judged against DESIGN §2. Fix what has one right answer, each with a test; bring the
owner only the choices with two.

## Design sections this implements

DESIGN §2 all.

## Environment (the worker owns this)

```
bring-up: fd-app --script <T84 route through to world 2, as translated_whole.rs plays it> --home $TMP/h
          --timeout 300; amiga-browser on $TMP/h/map/index.html (written by a scripted `menu show-map`)
teardown: both stopped by their seatbelts; confirm no fd-app and no headless Chrome process remains
```

## End to end (the worker drives this)

- sizes: 1280×800, 1920×1080, 800×600 · screens saved to the scratch folder, never the repo
- [ ] title: "Not playing"; W1-1's play: dot on the character's cell each poll, items left falling, coins
  vanishing behind the character, creatures in view moving
- [ ] Follow me across W1-1; a hand scroll stops it; the click resumes it
- [ ] the level ends and world 2 loads: "Not playing" during the load, then the page on W2-1 by itself
- [ ] a level picked by hand during play holds until the next change
- [ ] the app quits: "App not running" within 4 s
- [ ] Secrets on and off in each world; every layer; W6-1 at 800×600
- [ ] the unasked: a resize mid-play, two pages open at once, the page reloaded mid-play

## Done when

- [ ] Every defect with one right answer is fixed with a test that fails without it.
- [ ] A dated, worker-driven FINDINGS row says what was driven and what was seen.
- [ ] Any two-answer choice was asked and its answer recorded.
