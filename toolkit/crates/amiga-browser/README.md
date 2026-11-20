# amiga-browser

A test rig: a page a crate writes, opened from disk in headless Google Chrome and driven by a test.
It speaks Chrome's DevTools protocol over `--remote-debugging-pipe` (fds 3 and 4, NUL-separated JSON),
so it needs no websocket library and no port. Only tests depend on it (`[dev-dependencies]`).

## How to call it

```rust
use amiga_browser::{Browser, Opts};
use std::time::Duration;

let mut b = Browser::launch(Opts { width: 1920, height: 1080, ..Opts::default() })
    .unwrap_or_else(|e| panic!("{e}"));          // Chrome missing: fail, never skip
b.open(&page_dir.join("index.html"))?;           // waits for the load event
let title = b.eval("document.title")?;           // JSON value; a thrown error is Err
b.wait_for("window.ready === true", Duration::from_secs(2))?;  // until truthy
b.click(200.0, 150.0)?;                          // real mouse events at a viewport point
b.scroll(400.0, 300.0, 0.0, 500.0)?;             // a wheel turn there
b.viewport(800, 600)?;
b.screenshot(&out.join("shot.png"))?;
assert!(b.blocked_requests().is_empty());        // every non-file:// URL the page asked for
// drop(b): Chrome's process group is killed and its profile removed
```

- `chrome()` finds Chrome: `$CHROME`, else `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.
  Its error names where it looked.
- `Opts::default()` is a 30 s limit at 1280×800. The limit covers the whole session from launch: a
  call that passes it kills Chrome and returns an error saying so, and every later call returns the
  same. A backstop thread kills the group a second after the limit, even if the test is stuck elsewhere.
- Events (paused requests, load events) are handled while any call waits; `blocked_requests` reports
  what was seen up to the last call, so wait on the page (`wait_for`) before reading it.
- A page that re-reads a data file does it by inserting a `<script src="data.js?t=…">` tag; a file
  replaced by rename is read again (`tests/drive.rs`). A page opened from disk cannot `fetch`.
- `pid()` is Chrome's process and process group id; `profile()` the temp profile, a handy place for
  a test's screenshots (removed on drop).

## What it never does

- Never the owner's Chrome or profile: a fresh `--user-data-dir` under the temp dir, every time.
- Never the network: every request that is not `file://` is failed (`Fetch` domain) and listed, and
  `--host-resolver-rules` resolves no host name behind that. Background networking, sync, component
  updates and extensions are switched off.
- Never a visible window: always `--headless=new`.
- Never outlives its `Browser`: Chrome runs in its own process group, killed with `SIGKILL` on drop or
  at the limit, then reaped.

## Tests

`cargo test -p amiga-browser`, about 3 s with Chrome installed. Smoke runs the click test alone (about
1 s); the blocked font, the script re-read, the time limit and the drop check are full.
