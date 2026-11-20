# T04 — page-map-only

**Phase:** 2 · **Depends on:** T03 · **Weight:** light

## Goal

The toolkit page assumes every level has a replayed route (`END = track.length - 1`). A map-only level has
no track and no verdict. Make the page draw such a level cleanly, game-free, with Superfrog's page unchanged.

## Design sections this implements

DESIGN §2.3.

## Files

- `toolkit/crates/amiga-routeview/src/page.html`, `src/site.rs`, `README.md`, `tests/made_up_level.rs`.
- `toolkit/crates/amiga-routeview/tests/page_chrome.rs` (new, dev-dependency `amiga-browser`), `Cargo.toml`.
- Every builder of an `Entry`, for the `Option`: `games/superfrog/engine/crates/sf-routeview/src/site.rs`,
  `src/main.rs`; `games/forest-dumb/engine/crates/fd-routeview/src/lib.rs`.

## Interface

```rust
pub struct Entry { .., pub verdict: Option<Verdict>, .. }  // None: "Map only", no fault count
```

`track: []` in a level's JSON means map only: no actor, no route or heat layer, no scrubber or play button,
the readout showing the cell under the pointer.

## Tests

- [ ] `made_up_level.rs`: a map-only entry writes; its JSON has an empty track.
- [ ] Chrome (1280×800, 800×600): a site with one routed and one map-only level; the map-only one shows its
  objects and "Map only", no error in the console, switching between both works.
- [ ] Superfrog's `sf-routeview` tests pass unchanged.

## End to end (the worker drives this)

- suite: `amiga-routeview/tests/page_chrome.rs` on `amiga-browser` · sizes: 1280×800, 800×600
- [ ] open → map drawn, "Map only" shown, no console error
- [ ] toggle each layer → its objects hide and show
- [ ] switch to the routed level and back → scrubber appears and disappears

## Done when

- [ ] The Chrome test passes and is labelled per DESIGN §5; Superfrog's and the toolkit's suites are green.
- [ ] The README documents map-only entries.
