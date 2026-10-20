# crt-filters — Brief

**Brief reviewed:** not yet

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
- 2026-10-20: ports are `crt/shaders/crt-lottes.slang` (public domain, Timothy Lottes), `crt/shaders/crt-geom.slang` (GPL v2+, cgwg/Themaister/DOLLS) and `crt/shaders/crt-easymode.slang` (GPL, EasyMode), from github.com/libretro/slang-shaders master. Reason: the single-pass RetroArch staples; royale and guest-advanced are many-pass and too big.
- 2026-10-20: GPL code accepted into the repo. Each ported file keeps its original licence header and credits verbatim at its top. Reason: owner's choice, given that the repo has no licence of its own.
- 2026-10-20: each port uses RetroArch's default parameter values, fixed as constants. No tuning UI. Lottes and Geom therefore curve the picture (warp, rounded corners); Easymode is flat. Reason: owner wants the RetroArch look as shipped.
- 2026-10-20: menu only, no keyboard shortcut. Reason: owner's choice.
- 2026-10-20: menu is View → Filter ▸ None, Basic, Lottes, Geom, Easymode, exactly one checked, built like the Scale submenu.
- 2026-10-20: `settings.toml` gets `filter = "none" | "basic" | "lottes" | "geom" | "easymode"` (default `none`). An older file's `crt = true` reads as `basic`, `crt = false` as `none`; `filter` wins if both are present; only `filter` is written back. An unknown name is an `Err` naming the allowed values, as a bad `scale` is.

## Not changing

Scale, fullscreen, widescreen, the integer scaling and the game frame itself: every filter is presentation only, after scaling, as Basic is (DESIGN §2.7). Basic's shader code is not altered beyond what renaming needs. Curvature draws inside the picture's viewport only; the black border outside stays black.

## Done when

- View → Filter offers the five filters, the checked one is remembered across launches, and old `crt = …` files migrate as above.
- Lottes, Geom and Easymode each visibly differ from None at 1×, 2× and 3× (scanlines present at 2×), and none changes a game frame.
- README's View line names the Filter submenu; `view.rs`'s CRT tests cover every filter.

## Tests

- `settings.rs` unit tests: round trip of each filter name; `crt = true` → basic, `crt = false` → none; `filter` beats `crt`; unknown name is `Err`; missing both → none.
- `menu.rs`: each filter's command id round-trips (script names `menu filter-none`, `filter-basic`, `filter-lottes`, `filter-geom`, `filter-easymode`); the old `menu crt` id is removed.
- `sf-e2e/tests/view.rs`: extend `crt_changes_only_the_scaled_picture…` to every filter at 1×, 2×, 3×: game frames identical to None's; the shot differs from None's; black outside the viewport. Basic keeps its existing 1× legibility and 2× scanline asserts. For Lottes, Geom and Easymode at 2× and 3×, assert scanlines: a game line's middle row brighter than its edge rows, summed over the picture. Extend the remembered-by-next-launch test to a filter chosen from the menu, and add an old-file migration case.
- Keep `crt_drops_no_frame_over_3000_frames` (ignored) running with the heaviest port (Geom) on.

## End to end

Drive the real app through its script layer (`N: menu filter-…`, `shot`) on the existing `sf-e2e` rig, no new tooling: each filter at 1×, 2×, 3× and widescreen at 2×, title screen and a mid-level frame (W1-1). Save shots to a scratch folder and judge them against the RetroArch look (scanlines, mask, curvature where the port has it): nothing clipped outside the viewport, picture legible at 1×.

## Changed during the build
