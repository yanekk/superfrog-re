# T13 — forest-dumb-app

**Phase:** 3 · **Depends on:** T01, T10, T12 · **Weight:** medium

## Goal
The app the owner opens: window, keys, pad, menu, quit, disks message, straight into level 1, driven by a real end-to-end test.
## Design sections this implements
§2.3, §2.7, §2.9, §2.12.
## Reuses
`amiga-app` (video, pacing `Pacer::new(rate, 313)`, `Clock::Wall`, pad, lock, atomic write, moved windows), `amiga-e2e`'s `AppSpec`, `sf-app/src/{main,window,menu}.rs` as pattern, `fd-tools` lookup.
## Files
`games/forest-dumb/engine/crates/fd-app/`, `games/forest-dumb/engine/crates/fd-e2e/{src/lib.rs,tests/{smoke.rs,controls.rs,disks.rs,window.rs}}`, `Cargo.toml` profile overrides, `games/forest-dumb/smoke.toml`, `games/forest-dumb/CLAUDE.md`.
## Interface
```
fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR] [--max-frames N] [--timeout S] [--scale N] [--bugs-dir DIR]
FOREST_DUMB_HOME, FOREST_DUMB_DISKS
pub fn spec() -> AppSpec   // fd-e2e
```
## Tests
- [ ] Listed under End to end, plus: settings round trip, corrupt settings use defaults and are not overwritten, second copy refused.
- [ ] `fd-app` does not link `amiga-oracle` (dependency test).
## End to end (the worker drives this)
- suite: `fd-e2e`, on `amiga-e2e` · sizes: default, scale 1
- [ ] Launch → first frame hash equals headless `Game`'s.
- [ ] Scripted Up, Right keys → state hashes equal the headless replay of the same input, every frame.
- [ ] Controls window rebinding Up changes what moves the character, and persists.
- [ ] Missing disks → plain message naming where it looked, non-zero exit.
- [ ] Esc and menu Quit exit cleanly; watchdog ends a hung run.
## Done when
- [ ] End-to-end tests pass; smoke holds one under 45 s with an `[[area]]`, "the app plays level 1".
- [ ] `games/forest-dumb/CLAUDE.md` says how to run it.
