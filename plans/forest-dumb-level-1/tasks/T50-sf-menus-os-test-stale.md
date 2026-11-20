# T50 — sf-menus-os-test-stale

**Phase:** 3 · **Depends on:** — · **Weight:** light

## Goal
Bring Superfrog's OS-layer menu test back to green: it is red on the base branch (DESIGN §4.2), found by T12 on 2026-11-05. It expects File to hold only "Import Disks…"; the app's File menu also holds "Export Level" and "Play Level File…" (`sf-app/src/menu.rs`).
## Design sections this implements
§4.2.
## Reuses
`sf-e2e/tests/menus.rs`, `sf-app/src/menu.rs` (read only), `sf-e2e/README.md` § The OS layer.
## Files
`games/superfrog/engine/crates/sf-e2e/tests/menus.rs`.
## Interface
None. The test follows the app; the app's menus do not change.
## Tests
- [ ] `os_layer_the_menu_bar_holds_every_item_and_continue_follows_the_progress` asserts the File menu's items as the app builds them, enabled state included, and passes.
- [ ] Every other assertion in that test is checked against `menu.rs` on the way; any other stale item is fixed the same way.
## Done when
- [ ] `cd games/superfrog/engine && SF_E2E_OS=1 cargo test -q -p sf-e2e --test menus -- --ignored --test-threads 1` passes on an unlocked screen (Accessibility and Screen Recording granted). If the screen is locked, ask the owner to unlock it.
- [ ] `menu.rs` is unchanged.
