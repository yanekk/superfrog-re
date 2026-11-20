# fd-view-menu — Brief

**Brief reviewed:** 2026-11-20 — 8 fixed

## The change

fd-app (`games/forest-dumb/engine/crates/fd-app`) gains a View menu matching sf-app's, less
Widescreen: View ▸ Scale (1×, 2×, 3×, Auto; radio checks), Fullscreen (⌃⌘F, checked when on),
Filter (None, Basic, Lottes, Geom, Easymode in `amiga_app::video::Filter::ALL` order; radio checks).
The menu bar reads Forest Dumb, Game, View. Each choice applies at once and is saved in the shared
`amiga_app::settings::Settings` (`scale`, `fullscreen`, `filter`), so the next launch starts with
it. The pieces both apps need move from sf-app into `toolkit/crates/amiga-app`, and sf-app uses
them with its behaviour unchanged.

## Decisions

All 2026-11-20, the owner.

- Each item behaves as sf-app's (`sf-app/src/main.rs` `set_scale`, `set_filter`,
  `toggle_fullscreen`): a scale resizes the window to fd-app's `initial_size` for the picture unless
  fullscreen; a filter shows from the next present; fullscreen toggles `Video::fullscreen` and the
  window refits to the recording bar on leaving. Each saves the settings at once.
  fd-app has no `fit_window_to_bar` as sf-app does: its `show_menu` grows or shrinks the window by
  `BAR_HEIGHT` when the bar comes or goes, and its `initial_size` leaves the bar out. So a scale
  chosen while the bar shows sizes the window to the picture plus `BAR_HEIGHT`, and a bar that came
  or went while fullscreen leaves the window the right height once fullscreen ends.
- At launch fd-app applies the saved `filter` and `fullscreen` as sf-app does (`sf-app/src/main.rs`
  after `Video::new`: `video.filter = settings.filter`; fullscreen only if allowed, else the
  staying-windowed line). Today fd-app reads neither. It calls `before_launch()` before the event
  loop runs, as `sf-app/src/main.rs` does (`menu::before_launch()`).
- `--scale N` still wins at launch; a scale chosen from the menu in such a run is saved (as sf-app).
- fd-app gains `--allow-fullscreen`, refused unless `--max-frames` is 300 or fewer, as sf-app's
  `FULLSCREEN_MAX_FRAMES`. Without it a scripted run never goes fullscreen: View ▸ Fullscreen logs
  `fd-app: fullscreen refused: a scripted run needs --allow-fullscreen` and leaves the setting
  unchanged; a saved `fullscreen = true` opens windowed with a line saying so.
- Script `menu` ids follow sf-app's: `scale-1`..`scale-3`, `scale-auto`, `fullscreen`,
  `filter-<name>` (sf-app's script also accepts `menu scale 2`; fd-app takes what its own script
  parser takes for ids, extended only as far as these ids need). The shared parse keeps sf-app's
  range, `scale-1` to `scale-16` (sf-app's `from_id` today), so sf-app is unchanged; the menu shows
  only 1×, 2×, 3× and Auto.
- No Widescreen item; `settings.widescreen` is read and written back as it was, never changed.
- Moved to amiga-app (new module, e.g. `view_menu.rs`, game-free): `set_fullscreen(&Window, bool)`
  from `sf-app/src/window.rs`; `before_launch()` from `sf-app/src/menu.rs` (registers
  `NSFullScreenMenuItemEverywhere = NO`); the Scale/Fullscreen/Filter item builder holding its
  `CheckMenuItem`s and a method setting their checks from (scale, fullscreen, filter), plus the item
  ids and their parse. sf-app's `MenuBar` uses it and still appends its own Widescreen item after
  Filter, so its View menu reads exactly as today. How the ids join each app's `Command` (a
  toolkit enum wrapped by a variant, or ids parsed by each app) is the builder's choice.
- amiga-app gains a `muda = "0.21"` dependency, the version both apps already use (0.21.2 in both
  games' `Cargo.lock`; `toolkit/Cargo.lock` gains it); on macOS its `objc2-foundation` already has
  `NSUserDefaults`, `NSDictionary`, `NSString`, `NSValue`. Its `Cargo.toml` header ("menus stay the
  game's") and `toolkit/crates/amiga-app/README.md` "API by job" name the new module.
- The e2e test is two parts: a script-layer test that runs in smoke, and an OS-layer test
  (`#[ignore]`, `FD_E2E_OS=1`, screen unlocked), since only the Accessibility API reads menu titles
  and checks.

## Not changing

- sf-app's behaviour, its menu order and items, and its tests `sf-e2e/tests/menus.rs` and the
  widescreen tests: they stay unedited and must pass.
- Widescreen in fd-app: it runs the translated original, which has no wider view.
- fd-app's Forest Dumb and Game menus.

## Done when

- `fd-e2e/tests/view_menu.rs` (or similar) holds both parts and is labelled in
  `games/forest-dumb/smoke.toml`; the script-layer part is smoke if 10 s or less alone.
- sf-app's unit tests, `sf-e2e/tests/menus.rs` and the widescreen tests pass unchanged;
  `tools/check_tier_names.py` passes. Smoke runs only `menus.rs`'s script tests; the builder runs
  the rest itself and names each result in its commit: `menus.rs`'s three `os_layer_*` tests
  (`SF_E2E_OS=1 … -- --ignored --test-threads 1`), the full `sf-e2e` files `widescreen_app.rs`,
  `widescreen_drill.rs`, `widescreen_frog.rs`, `widescreen_objects.rs`, and the new fd OS-layer
  test. A locked screen or a missing Accessibility permission is asked of the owner, not skipped.
- `fd-app/README.md` (flags, script format) and `games/forest-dumb/CLAUDE.md` "Playing it" name the
  View menu and `--allow-fullscreen`.

## Tests

- fd-app unit: every View `menu` id parses back to its command; bad ids (`scale-0`, `scale-17`,
  `filter-crt`, `filter-`) do not. `--allow-fullscreen` with `--max-frames 300` parses, with 301 or
  without `--max-frames` is refused.
- fd-app settings unit: a file with `widescreen = true` saved after a View change keeps it.
- Script layer (smoke): `menu scale-2` then a `shot` shows the surface at 2× the picture and integer
  scaled; `menu filter-geom`, then a second `shot` is no longer the plain integer scaling (a shot is
  the present's own pass, `Video::shot`, so it shows the filter); settings.toml then holds
  `scale = 2` and `filter = "geom"`; a relaunch with `scale: None` on that home opens at 2× and its
  `shot` shows Geom too (the launch applied the saved filter). `menu fullscreen` without
  `--allow-fullscreen` logs `fullscreen refused` and leaves `fullscreen = false`. A saved
  `fullscreen = true` in a scripted run opens windowed and logs that it stays windowed.
- OS layer (`#[ignore]`, as `drill_os.rs` and sf-e2e's `os_layer_*` tests, release, screen-lock check
  first): `os_menu_bar()[1..]` is `["fd-app", "Game", "View"]` (the app menu, built as "Forest Dumb",
  shows the process name of an unbundled binary, as `drill_os.rs`'s `QUIT`); `os_menu_items(&["View"])` lists exactly Scale, Fullscreen, Filter (no AppKit "Enter Full Screen",
  so `before_launch` took);
  defaults checked Auto, Fullscreen off, None; choose 2× and Geom through the menu, they are the
  only checks in their submenus; quit, relaunch on the same home, still checked. View ▸ Fullscreen
  goes fullscreen with its check on, ⌃⌘F back with it off (bounds as sf-e2e's `menus.rs`).
- Smoke budget: the last good smoke run took 842 s of 900 (`~/.cache/superfrog-re/smoke-times.tsv`,
  2026-11-20, `a4eebadc`, measured again at plan review). fd-e2e's `window.rs` takes 5 s alone for
  similar launches, so two or three launches should fit. If the new smoke test does not fit, tell the owner before moving anything.

## End to end

The script-layer part drives the real fd-app through `--script` `menu` lines and `shot`, at the
default 1× launch and at 2×, on `App::launch` (hand engine, the owner's disks). The OS-layer part
clicks the real menu bar through the Accessibility API and presses ⌃⌘F through CGEvent, windowed at
Auto (2×) and fullscreen. Both use `fd-e2e`'s existing launcher and `amiga_e2e`'s OS calls; no new
rig.

## Changed during the build
