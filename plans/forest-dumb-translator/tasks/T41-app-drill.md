# T41 — app-drill

**Phase:** 7 · **Depends on:** T40 · **Weight:** medium

## Goal

The app's drill (`pir-e2e` § 3): a worker uses the translated app the way a person would, across the whole flow and at every size, judges it against DESIGN, fixes what has one right answer with a test each, and brings the owner only choices with two defensible answers.

## Design sections this implements

DESIGN §2.10, §2.4.

## End to end (the worker drives this)

- suite: `fd-e2e` (scenarios added for each fix) · sizes: every size `drill_sizes.rs` uses
- [ ] launch, title, every menu row (START, KODY, KONIEC) and Esc on the title
- [ ] Start, level 1 play, death, game over, restart; level end into world 2
- [ ] keys held through a load; resize during play and during a load; pause or focus loss if the app has it
- [ ] the stop path (`--stop-after`) at each size; the bug zip replays
- [ ] sound across title, play, death and level end

## Environment (the worker owns this)

```
cd games/forest-dumb/engine && cargo build --release -p fd-app   # with the generated tree in build/translator/tree/
# runs use --timeout and --max-frames; screens saved to build/drill/, never the repo
# teardown: no app process left (pgrep fd-app prints nothing)
```

## Done when

- [ ] Every interaction driven at every size, screens saved, a dated worker-driven 🐞 or 📌 row in FINDINGS.
- [ ] Each fix has an end-to-end test that fails without it; two-answer choices were asked and answered.
