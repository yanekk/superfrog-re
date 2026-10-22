# T07 — wide-recording

**Phase:** 1 · **Depends on:** T01 · **Weight:** medium

## Goal

A recording says whether it was played in widescreen and when it was switched, so a replay plays the
same game; comparing a widescreen recording with the original is refused with a plain reason; the
bug-fix guide says a widescreen-only bug is ours.

## Design sections this implements

DESIGN §2.8, D9; BR §2.6.

## Files

- `engine/crates/sf-core/src/recording.rs` (format v2)
- `engine/crates/sf-conform/src/` (`bug replay` applies the mode; `bug compare` refuses)
- `engine/crates/sf-conform/tests/widescreen_recording.rs` (new)
- `docs/BUG-FIX.md` (one paragraph)

## Interface

```text
superfrog recording 2
…header as v1…
widescreen off            # new field, after `spud`/`wings`; the mode at frame 0
inputs
0: -
900: R W                  # `W` toggles widescreen once, at frame 900
901: R                    # the writer starts a line at each toggle frame; W is never held
```

`Recording { …, widescreen: bool, toggles: Vec<u32>, … }`. `W` acts once at its line's first frame,
unlike the held tokens; the parser records a toggle there and holds only the line's other tokens. Version 1
parses as `widescreen off` with no toggles; the writer always writes 2. `bug compare` on a recording
whose header or any input has widescreen exits 1 with
`widescreen recording: there is no original to compare a widescreen game with (docs/BUG-FIX.md)`;
`bug case` refuses the same way (a case is an oracle comparison). `bug replay` sets the mode at frame 0
and at each toggle and renders the wide view while it is on, so its frames and its screenshot comparison
are the widescreen picture (DESIGN §2.8).

## Tests

- [ ] round trip of v2 with and without toggles; a v1 file parses as widescreen off.
- [ ] a `W` token in a frame that also holds R/F is parsed and written in a fixed order.
- [ ] a replay of a v2 recording with two toggles matches a direct engine run with the same switches,
  state for state (engine only).
- [ ] `bug compare` and `bug case` on a widescreen recording print the message and exit 1 without
  starting the oracle or writing a case.
- [ ] `bug replay` of a recording toggled on mid-level writes wide frames from the toggle on.
- [ ] a line holding `W` for several frames toggles once, not every frame.
- [ ] header field missing or misspelt in a v2 file: `RecordingError` naming the line.

## Done when

- [ ] Tests green; BUG-FIX.md has the widescreen paragraph.
- [ ] The app does not yet write v2 toggles (T10 wires it); the format is in place.
