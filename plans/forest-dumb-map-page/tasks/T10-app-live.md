# T10 — app-live

**Phase:** 2 · **Depends on:** T02, T05, T06 · **Weight:** heavy

## Goal

While the owner plays, the app writes `live.js` about four times a second into the page's folder, from either
engine, without changing or slowing play, and keeps playing if the write fails.

## Design sections this implements

DESIGN §2.5, §2.7, §2.9, §2.10, §3.4.

## Files

- `games/forest-dumb/engine/crates/fd-app/src/live.rs` (new: the writer thread, the folder lookup),
  `src/translated.rs` (copy `live_ranges()` bytes every 12th frame into `Frame`), `src/main.rs`, `Cargo.toml`
  (`fd-map`, `amiga-routeview`), `README.md`.
- `games/forest-dumb/engine/crates/fd-e2e/tests/live.rs` (new); `games/forest-dumb/smoke.toml`.

## Interface

```rust
pub fn live_dir(home: &Path, scripted: bool) -> PathBuf;  // scripted: <home>/map; else fd_map::default_out(), else <home>/map
pub struct LiveWriter { .. }                              // bounded channel of 1: a new frame replaces an unsent one
impl LiveWriter { pub fn start(dir: PathBuf) -> Self; pub fn send(&self, f: LiveFrame); }
pub const LIVE_EVERY: u32 = 12;                           // frames between writes
```

The `LiveFrame` is built from `fd_core::live::Live`: `level` from `world`, `actor` from `char_xy`, `actors` as
cat `enemy`, `cells` from `changed`, counter `items_left`; `t_ms` from the wall clock here in the shell. The program thread keeps the last sample's `Live::irq` and
passes it as the next `prev_irq` (T06, DESIGN §2.7).

## Tests

- [ ] `tests/deps.rs` still passes (no oracle or test tooling linked).
- [ ] A scripted run (`--script`, `--engine hand`, T84's route's first 600 steps): `<home>/map/live.js` exists,
  its last frame's cell and items left equal a headless `Game`'s at that step.
- [ ] A scripted translated run to W1-1 (full): the same against `location_in_ram`.
- [ ] State hashes with live writing equal a run's without it (`--capture-hashes`).
- [ ] The map folder made read-only: the run finishes, its hashes unchanged, one stderr line names the failure.
- [ ] On the title (`--max-frames` before play): `playing: false`.

## End to end (the worker drives this)

- suite: `fd-e2e/tests/live.rs` · the app under `--script`, `--home` temp, `--timeout`
- [ ] the cases above, through the real app binary and its real frame loop

## Done when

- [ ] The tests pass and are labelled per DESIGN §5 (one run of 5 s or less in smoke with an `[[area]]`, the rest full).
- [ ] fd-app's README says where `live.js` goes and that scripted runs never write the owner's folder.
