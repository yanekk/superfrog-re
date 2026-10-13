# T49 — story-fade-setup-job

**Phase:** 2 · **Depends on:** T46; blocks T56 · **Weight:** light

Split from T46 (owner, 2026-10-13: stop and cut the rest into tasks).

## Goal

Frame 1242 of `story-intro` stops diverging: the original's fade set-up (`fade_in` h0+$b6d8,
`fade_out` $b906, `fade_across` $b4b4) runs with interrupts off across the frame's end, so at the
frame's end its record is half written (`fade_records` +4 and the accumulators differ).

## How (found in T46, discarded uncommitted)

In `Intro::fade_setup` push a `Job` whose `inside` is every even address of the routine
(`(from..to).step_by(2).map(h0)`: across $b4b4..$b5c0, in $b6d8..$b7bc, out $b906..$ba42: the PC is
first in the chain) and whose writes are record 0 (`FADES[0].at`, $1f2) and the copper lists
(h2+$da5a, $240). With it the runner was green to frame 3184.

## Files

`engine/crates/sf-core/src/front/story/intro.rs`.

## Tests

- [ ] A case to frame 3185 (`frames = 3185`, sync `power-on-disk3`) green; keep it out of the test
  block if it is slow (about 45 s); say so in docs/13.

## Done when

- [ ] `sf-conform run` on that case is ok; `sf-conform scan` lists nothing before 3185.
