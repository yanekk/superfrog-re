# T31 — render-remaining-levels

**Phase:** 2 · **Depends on:** T13 · **Weight:** light

Added by the T13 review (owner, 2026-10-12): T13 renders 25 of the 29 levels against the original;
this task brings the other four under the same test.

## Goal

Reach, on the reference emulator, the four levels T13's render cases cannot: W6's second map set
(maps 2–4, `d4_c14`–`c17`, loaded by `level_load` h0+$a144 only when DF1–DF3 are connected and DF0
holds disk 1; docs/10) and the final level, which has no level code (password_table entries 24–27 all
lead to the bonus level). Then render each one by transplant, as T13's `render-*` cases do.

## Design sections this implements

DESIGN §2.1 (picture), §3.3 (`render`); T13's tests line "200 sync points spread over all 29 levels".

## Files

- `engine/crates/sf-oracle/src/{amiga.rs,config.rs}` and the shim, if a second drive is needed: an
  oracle with DF1 connected (today `insert_disk` refuses any drive but 0).
- `engine/crates/sf-conform/src/sync.rs` — `SyncPoint::AtLevel` for `W6-2`..`W6-4` with `alt`, and for
  the final level (a script that plays through W6-4 from its level code, or another route the worker
  finds in the original's code).
- `engine/conform/cases/render-w6-{2,3,4}-alt.toml`, `render-final.toml`; scripts under
  `engine/conform/scripts/`.
- `docs/12-display.md` "Reaching every level" — the new routes.

## Interface

No new engine interface. `SyncPoint::AtLevel(LevelId { world: W6, level: 2..=4, alt: true })` and the
final level's id (named as `sf_assets::LevelId` names it) become reachable; the existing
`compare = ["render"]` case form is used unchanged.

## Tests

- [ ] each of the four levels: a `render` case of 400 frames, every compared frame equal, skipped
      frames listed with T13's reasons, at least 300 frames compared.
- [ ] the oracle with a second drive still passes the existing one-drive sync points (W1-1 and one
      password level) unchanged, cached snapshots keyed so the two set-ups never share a cache entry.

## Done when

- [ ] the four render cases are green in `sf-conform -- all`.
- [ ] docs/12 "Reaching every level" describes how each is reached.
- [ ] any oracle change has its own unit test, and the one-drive set-up is still the default.
