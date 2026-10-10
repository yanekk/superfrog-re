# T27 — crt-and-widescreen

**Phase:** 3 · **Depends on:** T24 · **Weight:** light

## Goal

The two visual extras: a CRT shader applied after integer scaling, and widescreen, which widens the
window and asks the renderer for extra columns while the simulation stays untouched (DESIGN §2.7).

## Design sections this implements

DESIGN §2.7.

## Files

- `engine/crates/sf-app/src/{crt.wgsl,video.rs}`; `menu.rs` — View → CRT Filter, Widescreen.
- `engine/crates/sf-e2e/tests/view.rs`

## Interface

`Settings.crt`, `Settings.widescreen`; widescreen picks `View { extra_columns_each_side }` to fill
16:9 at the current integer scale (53 columns each side at 320 wide).

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 1×, 2×, 3×, 2× widescreen, fullscreen widescreen (≤ 300 frames)
- [ ] widescreen on mid-level: the centre 320 columns of every captured frame equal the 4:3 capture of the same script; `Game::probe` identical.
- [ ] widescreen at the left edge of a level and on the title, intro and slot screens (which are not level views): shows what DESIGN §2.7 says and nothing wraps or tears.
- [ ] CRT on: the capture differs from CRT off only after scaling (game frame input to the shader identical); CRT on at 1× is legible.
- [ ] both settings persist.

## Done when

- [ ] e2e green.
- [ ] What widescreen does on non-level screens is decided: if DESIGN does not fix it and there are two reasonable answers, the worker asked the owner.
- [ ] Frame pacing unaffected (no dropped frame over 3 000 frames with CRT on, measured).
