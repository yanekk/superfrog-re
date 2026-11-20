# T212 — conform-known-reds

**Phase:** 3 · **Depends on:** T29; blocks T30 · **Weight:** medium

Added by T29 with the owner's yes (2026-10-18).

## Goal

`cd engine && cargo run -q --release -p sf-conform -- all` green, so T30's acceptance starts from
a green conformance suite. T29's run (2026-10-18) was red on the cases below, each red since T197
(FINDINGS 2026-10-16) and owned by no task until now:

- `complete-w4-2`: the original ends 101 frames in a row mid-pass from frame 7867, past
  `LAG_LIMIT` (100), so nothing is compared (FINDINGS T184 row).
- `enemies-w6-1-complete`: checks 1–3 at 2136 after a lag, `clock_shown` (original $30109, engine
  $30209), the case setting `resync = false` (FINDINGS T184 review row).
- `w2-2-complete` (1229) and `w2-3-complete` (6729 on T29's run; T197's row had 2166).

T29's run: 208 of 212 green, 6 034 s, `game-whole` among the green.

For each: find why it parts from the original and fix the engine, or, where the case's own bound
is what is wrong (a measured longer run of mid-pass frames, say), give it a `lag-limit` with its
note as `game-whole` does. Never loosen a check to make a case green without the measurement
that justifies it (T120: no silent pass).

## Design sections this implements

DESIGN §2.1.

## Files

- `engine/conform/cases/{complete-w4-2,enemies-w6-1-complete,w2-2-complete,w2-3-complete}.toml`;
  the engine module each divergence leads to (`engine/crates/sf-core/src/`); a shorter case beside
  them for each fix.

## Interface

None new: a case may gain a `lag-limit` with its note, as `game-whole` has.

## Done when

- [ ] `sf-conform all` green, its run time in PROGRESS notes.
- [ ] Each fix guarded by its case (or a shorter case cut from it), and a FINDINGS row each.
