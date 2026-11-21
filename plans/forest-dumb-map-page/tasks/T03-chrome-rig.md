# T03 — chrome-rig

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

Nothing in the repo drives a browser. Every page task proves its page in a real Chrome, so the rig comes
first: a toolkit test crate that opens a `file://` page in headless Chrome, evaluates JavaScript, sets the
viewport, takes screenshots, blocks any non-`file://` request, and always tears Chrome down.

## Design sections this implements

DESIGN §5 End to end, §5.2.

## Files

- `toolkit/crates/amiga-browser/` (new): `Cargo.toml` (`serde_json`, `libc`), `README.md`, `src/lib.rs`,
  `tests/drive.rs` with a made-up page under `tests/fixtures/`.
- `toolkit/Cargo.toml` (member), `toolkit/smoke.toml`, `toolkit/README.md` (crate list).

## Interface

```rust
pub fn chrome() -> Result<PathBuf, String>;    // /Applications/Google Chrome.app/..., or CHROME env; Err names where it looked
pub struct Browser { .. }                       // Drop kills the process group, removes the temp profile
impl Browser {
    pub fn launch(opts: Opts) -> Result<Browser, String>;   // --headless=new, temp --user-data-dir, --remote-debugging-pipe
    pub fn open(&mut self, page: &Path) -> Result<(), String>; // waits for load
    pub fn viewport(&mut self, w: u32, h: u32) -> Result<(), String>;
    pub fn eval(&mut self, js: &str) -> Result<serde_json::Value, String>;
    pub fn wait_for(&mut self, js: &str, within: Duration) -> Result<serde_json::Value, String>; // js truthy
    pub fn screenshot(&mut self, png: &Path) -> Result<(), String>;
    pub fn click(&mut self, x: f64, y: f64) -> Result<(), String>;      // real input events
    pub fn scroll(&mut self, x: f64, y: f64, dx: f64, dy: f64) -> Result<(), String>;
    pub fn blocked_requests(&self) -> Vec<String>;   // every non-file:// URL asked for
}
pub struct Opts { pub limit: Duration /* 30 s */, pub width: u32, pub height: u32 }
```

The pipe (fds 3 and 4, NUL-separated JSON) avoids a websocket library. Chrome missing panics with a
message, never skips, like the disks.

## Tests

- [ ] The fixture page opens; `eval` reads its text; a click on a button changes it.
- [ ] A fixture that asks for an `https://` font: the request is blocked and listed.
- [ ] A rewritten script file is re-read by the page (the DESIGN §2.5 mechanism).
- [ ] The time limit: a page that never finishes is cut off, Chrome's group is gone afterwards.
- [ ] Drop: no Chrome process from this test remains (checked by pid).

## Done when

- [ ] `cargo test -p amiga-browser` passes, labelled per DESIGN §5 (one test of 5 s or less in smoke, with an `[[area]]`; the rest full).
- [ ] The README says how to call it and what it never does (no profile, no network, no visible window).
