# T142 — story-draw-end-order

> **Dropped** by the owner's amendment of 2026-10-15 (DESIGN §2.1 "What faithful means"): it chased
> sub-frame timing no player perceives. Not to be built. Its cases are re-judged by T181.

**Split from:** T56. Carries "that case is green" for engine frames 4026 and 4490–4491.

**Phase:** 2 · **Depends on:** T56 · **Weight:** light

## Goal

A draw's or the masks' DMACON writes land in the original's frame and in its order beside the
vertical blank's handler.

## Found in T56

- Engine frame 4026 (oracle 7041): the original's first write is the handler's `AUD0PER` (line 15),
  the engine's a DMACON `$0400` before it. Look at whose write it is (a draw's end or the masks')
  and where the handler was taken.
- Engine frame 4490 (oracle 7505): the original writes a draw's DMACON `$0400` on line 312, clock
  215; the engine's comes in the next frame (4491 differs as a result).
- Writes are now ordered by (line, cycle) within a frame (T56): a set-up job's carry their cycle,
  the handler's its entry; a draw's DMACON writes still carry cycle 0.

## Interface

None new.

## Files

`engine/crates/sf-core/src/front/story/intro.rs`.

## Tests

- [ ] `sf-conform scan` of `story-intro-full`: frames 4026, 4490 and 4491 match; the story tests
  stay green.

## Done when

- [ ] As the tests.
