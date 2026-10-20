# crt-filters — Brief

**Brief reviewed:** 2026-10-20 — 4 fixed, 1 decided with the person

## The change

View → CRT Filter (`sf-app/src/crt.wgsl`, T27) scales every effect with surface pixels per game
pixel, so at the owner's real setup (non-Retina 2560×1440, Scale Auto, 2 px a game pixel) the
scanline depth is 0.15 spread evenly over both rows, the mask is off below 3×, and only the gain and
vignette remain: it reads as a palette change. Measured from the shader's formulas, 2026-10-20.
The owner wants a real CRT look and wants to compare RetroArch filters in the app before choosing
one. So the single toggle becomes a View → Filter submenu: None, Basic (today's filter, unchanged),
Lottes, Geom, Easymode, the last three ported from libretro `slang-shaders`.

## Decisions

- 2026-10-20: today's filter stays as is and is renamed "Basic". Reason: the owner sees it work at 3× and fullscreen.
- 2026-10-20: ship several filters and pick in the app rather than mock first. Reason: compare at the real size while playing; the losers are removed by a later single run.
- 2026-10-20: ports are `crt/shaders/crt-lottes.slang` (public domain, Timothy Lottes), `crt/shaders/crt-geom.slang` (GPL v2+, cgwg/Themaister/DOLLS) and `crt/shaders/crt-easymode.slang` (GPL, EasyMode), from github.com/libretro/slang-shaders master. Reason: the single-pass RetroArch staples; royale and guest-advanced are many-pass and too big. Checked 2026-10-20: all three exist there; each `.slangp` is one pass with `filter_linear0 = false` and `scale_type0 = viewport`, which matches the app's nearest sampler and its viewport-sized pass. Geom's `interlace_detect` and Easymode's `SCANLINE_CUTOFF` (400) do not trigger at Superfrog's height.
- 2026-10-20: GPL code accepted into the repo. Each ported file keeps its original licence header and credits verbatim at its top. Reason: owner's choice, given that the repo has no licence of its own.
- 2026-10-20: each port uses RetroArch's default parameter values, fixed as constants. No tuning UI. Lottes and Geom therefore curve the picture (warp, rounded corners); Easymode is flat. Reason: owner wants the RetroArch look as shipped.
- 2026-10-20 (plan review): at exactly 2 surface pixels a game pixel each port samples a quarter game pixel lower (one row on the line centre, the other in the gap), the only departure from the shipped defaults. Reason: at 2× the three ports, as shipped, light both rows of a game line equally (fragment centres at 0.25 and 0.75 of a source line, symmetric for Lottes' `Dist`, Geom's line weights and Easymode's `cos` scan weight), so there are no scanlines at the owner's size; measured from the upstream sources 2026-10-20. Other scales are untouched.
- 2026-10-20: menu only, no keyboard shortcut. Reason: owner's choice.
- 2026-10-20: menu is View → Filter ▸ None, Basic, Lottes, Geom, Easymode, exactly one checked, built like the Scale submenu.
- 2026-10-20: `settings.toml` gets `filter = "none" | "basic" | "lottes" | "geom" | "easymode"` (default `none`). An older file's `crt = true` reads as `basic`, `crt = false` as `none`; `filter` wins if both are present; only `filter` is written back. An unknown name is an `Err` naming the allowed values, as a bad `scale` is.

## Not changing

Scale, fullscreen, widescreen, the integer scaling and the game frame itself: every filter is presentation only, after scaling, as Basic is (DESIGN §2.7). Basic's shader code is not altered beyond what renaming needs. Curvature draws inside the picture's viewport only; the black border outside stays black.

## Done when

- View → Filter offers the five filters, the checked one is remembered across launches, and old `crt = …` files migrate as above.
- Lottes, Geom and Easymode each visibly differ from None at 1×, 2× and 3×, show scanlines at 2× (through the quarter-pixel nudge) and 3×, and none changes a game frame.
- README's View line names the Filter submenu; `view.rs`'s CRT tests cover every filter.

## Tests

- `settings.rs` unit tests: round trip of each filter name; `crt = true` → basic, `crt = false` → none; `filter` beats `crt`; unknown name is `Err`; missing both → none; a migrated `crt = …` file is written back with a `filter` line and no `crt` line.
- `menu.rs`: each filter's command id round-trips (script names `menu filter-none`, `filter-basic`, `filter-lottes`, `filter-geom`, `filter-easymode`); the old `menu crt` id is removed.
- `sf-e2e/tests/view.rs`: extend `crt_changes_only_the_scaled_picture…` to every filter at 1×, 2×, 3×: game frames identical to None's; the shot differs from None's; black outside the viewport. Basic keeps its existing 1× legibility and 2× scanline asserts. For Lottes, Geom and Easymode at 2× and 3×, assert scanlines in the central half of the picture (the warp moves rows near the edges): sum each row phase within a game line (row `0..s`) over that region; the brightest phase exceeds the darkest by at least 10%. At 2× this fails without the nudge, which is what it is for. Extend the remembered-by-next-launch test to a filter chosen from the menu, and add an old-file migration case.
- `pir`'s test command here runs only the Python suite, so the builder runs these itself before reporting: `cargo test -p sf-app`, and `cargo test --release -p sf-e2e --test view` (finds the disks in the main checkout; today's CRT test takes about 2 minutes).
- Keep `crt_drops_no_frame_over_3000_frames` (ignored) running with the heaviest port (Geom) on.

## End to end

Drive the real app through its script layer (`N: menu filter-…`, `shot`) on the existing `sf-e2e` rig, no new tooling: each filter at 1×, 2×, 3× and widescreen at 2×, title screen and a mid-level frame (W1-1). Save shots to a scratch folder and judge them against the RetroArch look (scanlines, mask, curvature where the port has it): nothing clipped outside the viewport, picture legible at 1×.

## Changed during the build

- 2026-10-20: pir's tests failed on the starting point too (system `python3` lacks `amitools`, `machine68k`). `.pir/settings.json` setup now makes `venv` and installs `requirements.txt` into it, and the test runs `venv/bin/python`; the person's call.
