# T05 — compare-tool

**Phase:** 1 · **Depends on:** T03, T04 · **Weight:** heavy

## Goal

Give an agent the comparison with the original: the bug zip's start and inputs played on the oracle,
retimed to the original's passes, judged by the faithful checks over the whole recording, with the
first difference named when there is one and the original's picture at the STOP moment saved for a
quirk report; and turn a recording into a permanent case. It is proven on existing routes recorded
as the app would record them, through a level end, the tally, the slot machine and the next level.

## Design sections this implements

DESIGN §2.8, §2.9 (as T00 settled it), §2.10 steps 2, 4, 5.

## Files

- `engine/crates/sf-conform/src/bug.rs`: retiming, compare, case writing
- `engine/crates/sf-conform/src/main.rs`: `bug compare`, `bug case`
- `engine/crates/sf-conform/tests/recorder_compare.rs` (new)
- `engine/conform/README.md`

## Interface

```
sf-conform [--strict] bug compare <zip> [--out DIR]
sf-conform bug case <zip> <name>
```

`compare` prints `same` or `differs at frame F (engine step S): <first divergence>`, and writes
`original-stop.png` and `engine-stop.png` to `--out` (default beside the zip). `case` writes
`engine/conform/cases/<name>.toml` (`sync` as T03's `Setup` form, `script = "<name>"`, `frames`, a
first-line comment with the note and `stopped-at`) and `engine/conform/scripts/<name>.txt` in the
original's frames; it refuses an existing name.

```rust
/// The recording's inputs placed at the frames the original's passes read them (DESIGN §2.9).
pub fn retime(rec: &Recording, original: &mut Synced) -> InputScript;
pub fn compare(rec: &Recording, mode: Mode) -> Result<Outcome, Divergence>;
```

## Tests

- [ ] `chain-w1-1-to-w1-2`'s inputs played unpaced from `start_level` as a recording, retimed: `compare` says same
- [ ] W1-1 to its exit, the tally choosing the slot machine, `slot-gamble`'s play, on into W1-2's intro and 500 frames of W1-2: same (the stretch the acceptance bug is in)
- [ ] the long-lag level T00 used: same
- [ ] an engine with a variable deliberately perturbed mid-run (a test hook, as existing divergence tests do): differs, naming that frame and variable
- [ ] `case` output parses as a case and `run` passes it; a second `case` with the same name is refused
- [ ] slow ones ignored with times; one short one in the block

## Done when

- [ ] `bug compare` and `bug case` work as above with the tests listed
- [ ] the acceptance stretch (W1-1 exit → slot → W1-2) compares same on today's engine, or the difference found is a FINDINGS row and the owner is told
- [ ] the test command is green
