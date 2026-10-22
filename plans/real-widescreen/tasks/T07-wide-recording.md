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
900: R W                  # `W` toggles widescreen from this frame on (a token like R or F)
```

`Recording { …, widescreen: bool, … }`; the input token `W` is parsed by the same token rule. Version 1
parses as `widescreen off` with no toggles; the writer always writes 2. `bug compare` on a recording
whose header or any input has widescreen exits 1 with
`widescreen recording: there is no original to compare a widescreen game with (docs/BUG-FIX.md)`.

## Tests

- [ ] round trip of v2 with and without toggles; a v1 file parses as widescreen off.
- [ ] a `W` token in a frame that also holds R/F is parsed and written in a fixed order.
- [ ] a replay of a v2 recording with two toggles matches a direct engine run with the same switches,
  state for state (engine only).
- [ ] `bug compare` on a widescreen recording prints the message and exits 1 without starting the
  oracle.
- [ ] header field missing or misspelt in a v2 file: `RecordingError` naming the line.

## Done when

- [ ] Tests green; BUG-FIX.md has the widescreen paragraph.
- [ ] The app does not yet write v2 toggles (T10 wires it); the format is in place.
