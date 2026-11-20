# T57 — sf-scale-fullscreen-os-test

**Phase:** 3 · **Depends on:** — · **Weight:** light

Added by T50's review with the owner's yes (2026-11-05): T50's Done when runs every OS test in `menus.rs`, and the scale and fullscreen one fails on code identical to the base branch. T50 closed on its own test; this task takes the other.

## Goal
Make Superfrog's OS-layer test `os_layer_scale_and_fullscreen_through_the_menu_bar_and_its_shortcut` (`sf-e2e/tests/menus.rs`) pass reliably on the owner's 1920×1080 display. On 2026-11-05 it failed every run, each at another wait: "the window at 3×: not within 20 s" (T50 review, first run); "fullscreen: not within 20 s" after Scale 3× (1002×817), 1× and 2× had all been reached (T50 review, second run); and at ⌃⌘F (T50's builder). The app and the test were unchanged from the base branch, so the cause is the test's timing or the display, not a stale assertion. Find which, by measurement, and fix the test; change the app only if the app is what is wrong, and say so.

## Design sections this implements
§4.2.

## Reuses
`sf-e2e/tests/menus.rs` (`until`, `window_is`), `sf-e2e/README.md` § The OS layer, `sf-app/src/menu.rs` and the app's window code (read first).

## Files
`games/superfrog/engine/crates/sf-e2e/tests/menus.rs`; the app's window code only if the measurement shows the app at fault.

## Interface
None.

## Tests
- [ ] `os_layer_scale_and_fullscreen_through_the_menu_bar_and_its_shortcut` passes five runs of five on the 1920×1080 display, each wait's bound justified by a measured time, not raised until green.
- [ ] A failure prints the window bounds seen, so the next red says where it stopped.

## Done when
- [ ] `cd games/superfrog/engine && SF_E2E_OS=1 cargo test -q -p sf-e2e --test menus -- --ignored --test-threads 1` passes five times running on an unlocked screen (Accessibility and Screen Recording granted). If the screen is locked, ask the owner to unlock it.
- [ ] The cause found is one row in `FINDINGS.md`.
