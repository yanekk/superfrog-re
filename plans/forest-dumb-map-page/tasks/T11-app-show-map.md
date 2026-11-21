# T11 — app-show-map

**Phase:** 2 · **Depends on:** T08, T10 · **Weight:** medium

## Goal

Game ▸ Show Map writes the page fresh into the folder the app's live file goes to and opens it in Chrome, so
the owner gets the map from the app in one click.

## Design sections this implements

DESIGN §2.8, §2.9, §2.10.

## Files

- `games/forest-dumb/engine/crates/fd-app/src/menu.rs` (the item, id `show-map`), `src/main.rs`,
  `src/live.rs` (the page write on a thread), `src/script.rs` (`N: menu show-map`), `README.md`.
- `games/forest-dumb/engine/crates/fd-e2e/tests/show_map.rs` (new); `games/forest-dumb/smoke.toml`.

## Interface

```rust
pub enum Opened { Opened(PathBuf), WouldOpen(PathBuf), Failed(String) }
pub fn show_map(dir: &Path, files: &Files, program: &[u8], scripted: bool) -> Opened;
```

Unscripted: `open -a "Google Chrome" <index.html>`; Chrome absent (no `/Applications/Google Chrome.app`) or a
write error → the native message (as the disk message, `rfd`) naming it. Scripted:
prints `show-map: would open <path>` and opens nothing.

## Tests

- [ ] Script `100: menu show-map`: the page is in `<home>/map`, six levels, stdout has the `would open` line,
  the game's hashes are unchanged.
- [ ] The folder unwritable: stderr names it, the run goes on to its frame cap.
- [ ] The menu lists Show Map under Game (OS layer, `FD_E2E_OS=1`, full).

## End to end (the worker drives this)

- suite: `fd-e2e/tests/show_map.rs` · the app under `--script`
- [ ] the cases above through the real menu command path

## Needs a person

```
cd games/forest-dumb/engine && cargo run --release -p fd-app
```
Then Game ▸ Show Map. Expect: Chrome opens the map beside the app on the level being played, the dot on the
character. Tell me: that Chrome opened on your screen and the page was the map. (It takes the owner's screen,
DESIGN §5.1.)

## Done when

- [ ] The scripted tests pass and are labelled per DESIGN §5; the owner's answer is in FINDINGS with the date.
