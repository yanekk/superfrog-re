# T05 — page-live

**Phase:** 2 · **Depends on:** T04 · **Weight:** heavy

## Goal

The toolkit page gains a live mode: it re-reads a `live.js` file the app rewrites and draws where play is.
Game-free, off unless a game's `PageSpec` asks for it, so Superfrog's page is unchanged. The writer half,
`write_live`, is here too so the toolkit owns the file's shape on both sides.

## Design sections this implements

DESIGN §2.5, §2.6, §2.7, §2.10, §3.3.

## Files

- `toolkit/crates/amiga-routeview/src/live.rs` (new), `src/site.rs`, `src/page.html`, `src/lib.rs`, `README.md`.
- `toolkit/crates/amiga-routeview/tests/live_chrome.rs` (new), `tests/live.rs` (new).
- `toolkit/smoke.toml`.

## Interface

```rust
pub struct LiveSpec { pub file: String /* "live.js" */, pub poll_ms: u32 /* 250 */, pub stale_ms: u32 /* 3000 */,
                      pub counter_labels: Vec<(String, String)> /* ("items_left", "Items left") */ }
pub struct PageSpec { .., pub live: Option<LiveSpec> }
pub struct LiveFrame { pub t_ms: u64, pub playing: bool, pub level: Option<String>,
    pub actor: Option<(i32, i32)>, pub cell: Option<(u16, u16)>,
    pub actors: Vec<(i32, i32, String)>, pub cells: Vec<(u16, u16, u8)>, pub counters: Vec<(String, i64)> }
impl LiveFrame { pub fn to_js(&self) -> String }            // `window.LIVE = {...};` (DESIGN §3.3)
pub fn write_live(dir: &Path, f: &LiveFrame) -> io::Result<()>; // live.js.tmp then rename
```

The page, when `live` is set: every `poll_ms` inserts `<script src="live.js?t=…">`, removes it after load or
error; draws the actor dot and its cell, each actor in `actors` by its cat's shape, each changed cell with its
value's layer (0: the object hidden); shows the counters; switches level when `LIVE.level` changes (a hand
pick holds until it changes again); "Follow me" toggle, on at load, off on a hand scroll or drag, on by click,
scrolling to keep the dot in view; the status line "Not playing", "App not running" (no file, or
`Date.now() - t > stale_ms`), "Playing a level this map does not have".

## Tests

- [ ] `live.rs`: `to_js` shape; `write_live` leaves no `.tmp` and a reader never sees a partial file
  (a reader loop beside 1 000 writes).
- [ ] Superfrog's site writes with `live: None` and its page holds no live code path that runs.

## End to end (the worker drives this)

- suite: `amiga-routeview/tests/live_chrome.rs` on `amiga-browser`, made-up level, the test writing frames
  · sizes: 1280×800, 1920×1080, 800×600
- [ ] a frame with `playing: true` → dot at its pixel, readout names the cell, counter shown
- [ ] a new frame moving the actor 200 cells → the view scrolls to it (Follow me on)
- [ ] a hand scroll → Follow me off, the next frame does not scroll; click Follow me → it scrolls back
- [ ] a frame with `cells: [[c, r, 0]]` on a coin → that coin is gone; a value elsewhere → drawn there
- [ ] `actors` → drawn at their positions, moving with the next frame
- [ ] `level` changes → the page switches; pick another by hand → it stays until `level` changes again
- [ ] `playing: false` → no dot, "Not playing"; stop writing for 4 s → "App not running"; delete the file → the same
- [ ] an unknown level id → "Playing a level this map does not have"
- [ ] no request other than `file://`

## Done when

- [ ] `tools/check_tier_names.py` passes: no game is named in the toolkit files touched.

- [ ] Both new test files pass and are labelled, the live area in smoke with an `[[area]]`.
- [ ] Superfrog's page tests are green; the README documents `LiveSpec`, `LiveFrame` and `write_live`.
