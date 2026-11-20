# T09 — secrets-layer

**Phase:** 2 · **Depends on:** T07, T08 · **Weight:** light

## Goal

Put T07's confirmed secrets on the page as their own layer, off at load, so the owner can reveal them.

## Design sections this implements

DESIGN §2.3, §2.4.

## Files

- `games/forest-dumb/engine/crates/fd-map/src/lib.rs`, `src/level.rs` (secrets into the objects, cat
  `secret`, named by kind), `README.md`.
- `games/forest-dumb/engine/crates/fd-routeview/tests/levels_chrome.rs` (a secrets case).

## Interface

`Category::new("secrets", "Secrets", <colour>, false, &["secret"])` in `fd_map::spec()`; each `Secret` as a
`MapObject { cat: "secret", name: "hidden passage" | "invisible floor" | .. }`.

## Tests

- [ ] `map_level` on a made-up level with a known secret lists it with cat `secret`.
- [ ] Each world's secret count equals `fd_assets::secrets`'s.

## End to end (the worker drives this)

- suite: `levels_chrome.rs` · sizes: 1280×800
- [ ] at load no secret is drawn; switch Secrets on → each is drawn at its cell with its name on hover
- [ ] a world with none shows the layer with a count of 0

## Done when

- [ ] Tests pass; if T07 confirmed no kind, the layer is absent and the README says why.
