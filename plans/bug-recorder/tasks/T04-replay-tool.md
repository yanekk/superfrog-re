# T04 — replay-tool

**Phase:** 1 · **Depends on:** T01, T02 · **Weight:** light

## Goal

Give an agent a headless replay of a bug zip: the engine started as the recording says, stepped
through its inputs unpaced as the app played them, pictures saved outside the repo, and a check that
the replay ends on the owner's screenshot and location, which is the proof the reproduction is
faithful to what the owner saw.

## Design sections this implements

DESIGN §2.7 (second bullet), §2.5.

## Files

- `engine/crates/sf-conform/src/bug.rs` (new): zip reading, replay
- `engine/crates/sf-conform/src/main.rs`: `bug replay`
- `engine/crates/sf-conform/tests/recorder_replay.rs` (new)

## Interface

```
sf-conform bug replay <zip> [--out DIR] [--frames A..B] [--every N]
```

`--out` defaults to `~/.cache/superfrog-re/bugs/<zip stem>/`. It writes `frame-NNNNNN.png` for the
frames asked (default: the last 250) and prints, one line each: `frames N`, `stopped-at <recorded>`,
`replay-at <location>`, `screenshot equal|differs (P pixels)`. Exit 0 when parsed and replayed,
whatever the comparison says.

```rust
pub fn read_zip(bytes: &[u8]) -> Result<(Recording, Vec<u8> /* png */), String>;
pub struct Replayed { pub game: Game, pub location: Location, pub last: Picture }
pub fn replay(rec: &Recording, mut each: impl FnMut(u32, &Game)) -> Replayed;
```

## Tests

- [ ] a recording built in the test (an existing script's inputs from W1-1, `start_level`, PNG from the engine's own picture) zipped with `write_stored`: replay says equal and the location matches
- [ ] the same zip with one input changed early: replay says differs
- [ ] a zip missing `recording.txt`, a non-zip, a bad header: refused with the reason, exit non-zero
- [ ] `--frames` and `--every` write exactly the frames asked; nothing is written under the repo

## Done when

- [ ] `sf-conform bug replay` works as above with the tests listed
- [ ] `engine/conform/README.md` documents it in a short section
- [ ] the test command is green
