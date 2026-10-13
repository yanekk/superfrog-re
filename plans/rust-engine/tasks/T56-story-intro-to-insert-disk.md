# T56 — story-intro-to-insert-disk

**Phase:** 2 · **Depends on:** T49, T50, T54, T55; blocks T21, T47, T48 · **Weight:** heavy

**Split from:** T46. T46's goal and done-when, after its parts.

## Goal

Every frame of the disk-3 intro, untouched, from its start to the "insert disk 1" page (about
frame 9 268), matches the original (picture, Paula writes, the story's variables), loads re-synced
as T32 does. Scenes 4–15's picture misses T46's scan left (2257–2269, 2865–2879, 3071–3097,
3295–3337, 4551–4870, 7227–7713, ...) are this task's, once T49–T55 have run.

## Interface

None new: `Start::Disk3` and the sync point `power-on-disk3`.

## Files

`engine/crates/sf-core/src/front/story/`; `engine/conform/cases/story-*`;
`docs/13-front-end.md`.

## Tests

- [ ] A case from the intro's start to the insert-disk-1 page (outside the test block if slow);
  the test block keeps one short intro case (`story-intro`).

## Done when

- [ ] That case is green; docs/13's intro timings cover every scene, and the fed values (handler
  entry and end) are written down there.
