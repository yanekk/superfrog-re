# T07 — frame-parameterise

**Phase:** 5 · **Depends on:** T00 · **Weight:** medium

## Goal

The toolkit's frame length becomes a value a game supplies, not a constant the toolkit assumes
(DESIGN §2.9). The toolkit's README names this as the next game's first step; the beam `Clock`
and the policy functions already take the frame length, and this task threads it through the
remaining consumers. Superfrog's engine must keep behaving exactly as it does: it passes the PAL
constant it always used, and its tests are the proof. This is tier-2 work; no game name enters
the tree except on `learned on:` lines.

## Design sections this implements

DESIGN §2.9, §3.2.

## Files

- `toolkit/crates/amiga-clock/src/blit.rs`, `timeline.rs` — the frame length as a parameter:
  `Timeline` gains a `frame_lines` field set in `new`, the blit helpers take it where they read
  the constant.
- `toolkit/crates/amiga-chips/src/copper.rs` — the copper run takes the frame length instead of
  reading `FRAME_LINES`.
- `toolkit/crates/amiga-paula/src/channel.rs` — the per-frame resample takes the frame length
  from its configuration instead of the constant.
- `toolkit/crates/dry-fit/src/lib.rs` — passes its frame length explicitly.
- `games/superfrog/engine/crates/sf-core/` — the call sites pass `FRAME_LINES` (or the game's
  `Timing` value, which is the same 313) explicitly. Nothing else in Superfrog's engine changes.
- `games/forest-dumb/docs/` — a dated line recording the measured frame length from T00's
  FINDINGS, ready for T08's timing. (The number itself is in T00's rows; this line makes the
  game's notes carry it.)

## Interface

`FRAME_LINES` (313) and `SHORT_FRAME_LINES` stay as the PAL constants. Production code in the
toolkit never reads them from the crate root; every consumer takes `frame_lines: u16` (or a
`DisplayTiming`, where one already flows). The rule a reviewer checks: no `use
amiga_clock::FRAME_LINES` in toolkit production code, only in tests and in Superfrog's engine.

## Tests

- [ ] The toolkit's suites green, with a new test per touched module: a 312-line frame wraps the
      copper run, the blit timing and the timeline's yield at line 311; the Paula resample takes
      the frame length it is given.
- [ ] Superfrog's engine suite green (this task touches its call sites, so the selector runs the
      toolkit and every game; the end gate re-runs it).
- [ ] The purity test still green: nothing about the boundary moved.

## Done when

- [ ] No toolkit production code reads `FRAME_LINES` from the crate root; the consumers take it
      as a parameter, and the suites above are green.
- [ ] The game's measured frame length (T00's answer) is recorded in the game's notes, dated,
      ready for T08.

## Outside actions

None.
