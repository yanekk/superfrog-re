# T132 — story-intro-full-green

**Split from:** T56. Carries T56's done-when: "that case is green; docs/13's intro timings cover
every scene, and the fed values (handler entry and end) are written down there".

**Phase:** 2 · **Depends on:** T56, T128, T129, T130, T131; blocks T21, T47, T48 · **Weight:** light

## Goal

Every frame of the disk-3 intro, untouched, from its start to the "insert disk 1" page matches the
original: the case `story-intro-full` (T56, an ignored test in `tests/story.rs`) is green.

## State at T56's split

- `sf-conform scan conform/cases/story-intro-full.toml` (about 10 min on a busy machine): 36
  differing frames, all in T128–T131's lists, against 867 (picture and writes alone) before T56.
- docs/13 has the set-up job, the unpacks' end lines and the fed values (entry, end, merged
  blanks, a held handler's acknowledge in the next step). Still to write: the frame table of every
  scene (text page, set-up, play, out) past scene 1, from a run of the case.

## Interface

None new.

## Files

`docs/13-front-end.md`; whatever the last divergences need.

## Tests

- [ ] `cargo test -p sf-conform --test story -- --ignored`: every story case green, the full one
  included.

## Done when

- [ ] That case is green; docs/13's intro timings cover every scene, with their frames.
