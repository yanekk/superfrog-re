# T16 — app-drill

**Phase:** 3 · **Depends on:** T13, T14 · **Weight:** light

## Goal
A worker uses the whole app on the free backend, fixes what has one right answer, and brings the owner only two-answer choices.
## Design sections this implements
§2.7, §2.8.
## Reuses
`amiga-e2e` OS layer, `sf-e2e/tests/drill.rs`, `recorder_drill.rs`.
## Files
`fd-e2e/tests/drill.rs`, fixes where they fall, `FINDINGS.md`.
## End to end (the worker drives this)
- suite: `fd-e2e`, OS layer with `FD_E2E_OS=1` · sizes: default, scale 1, resized
- [ ] Each menu item, Controls, Start Level, recording, Quit, Tab, Esc, resize, lost focus, very long note, each saved to scratch.
- [ ] Judged against DESIGN §2.7, §2.8; each decidable defect gets a failing-then-passing test.
## Done when
- [ ] A dated worker-driven row in FINDINGS says what was driven and seen.
- [ ] Two-answer questions were asked, not decided.
