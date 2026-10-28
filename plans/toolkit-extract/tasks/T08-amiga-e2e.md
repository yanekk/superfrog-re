# T08 — amiga-e2e

**Phase:** 3 · **Depends on:** T07 · **Weight:** medium

## Goal

Move the end-to-end driver (launch the real app in its own process group with a scratch home, feed
it a script, read its captures, capture its windows) into `amiga-e2e`, with everything that names
the app given by the game in an `AppSpec`. Game two's app is driven by the same harness. Every test
under `sf-e2e/tests` stays Superfrog's.

## Design sections this implements

DESIGN §2.2 row `amiga-e2e`, §2.7.

## Files

- New: `toolkit/crates/amiga-e2e/src/{lib,app,os}.rs`.
- `engine/crates/sf-e2e/src/lib.rs` (Superfrog's `AppSpec`, re-exports), `sf-e2e/Cargo.toml` (drop
  the unused `sf-paula` and `sf-disk` dependencies).
- `docs/architecture/verification.md` or `app.md` `owns`, moved-path table.

## Interface

```rust
pub struct AppSpec {
    pub package: &'static str,          // "sf-app"
    pub binary: &'static str,           // "superfrog"
    pub workspace: PathBuf,             // engine/, today derived from CARGO_MANIFEST_DIR/../..
    pub flags: Vec<String>,             // --home --capture --max-frames --timeout --scale --script
    pub clear_env: Vec<&'static str>,   // SUPERFROG_HOME
    pub keep_env: &'static str,         // SF_E2E_KEEP
    pub provision: fn(home: &Path) -> io::Result<()>,  // disks and seed cache into the scratch home
    pub already_running: &'static str,  // the lock message to retry on
}
pub fn run(spec: &AppSpec, script: &str, ..) -> Result<Run, ..>;  // as today's app::run
pub struct Image; impl Image { pub fn of_picture(p: &Picture) -> Image; }
```

## Tests

- [ ] `os.rs`'s and `lib.rs`'s unit tests pass in the toolkit.
- [ ] Every sf-e2e test that runs in the test block passes unchanged.
- [ ] `cargo test -p sf-e2e --test whole_game` (non-ignored part) passes.

## Done when

- [ ] `amiga-e2e` holds the driver with no game name or `sf-*` dependency; Superfrog's spec is in sf-e2e.
- [ ] Test block green; DESIGN §4's per-task runs green.
