# T10 — level-drawing

**Phase:** 2 · **Depends on:** T08, T09 · **Weight:** heavy

## Goal
Draw the level, character and status bar as the original does: the picture half of the character stage.
## Design sections this implements
§2.6; `docs/06`.
## Reuses
`amiga-chips` (`Planes`, `Colours`, `copper::run*`, `Blit`), `sf-core/src/render/` as pattern, T04's `picture_in`, `Picture::rewindow`. Sprite composition moves to the toolkit only if T00 found hardware sprites.
## Files
`fd-core/src/render/{mod.rs,playfield.rs,status.rs,objects.rs}`, `engine/conform/cases/picture-*.toml`, `fd-conform/tests/purity.rs`, `smoke.toml`.
## Interface
```
impl Game { pub fn picture(&self) -> Picture; pub fn picture_rgba(&self, out: &mut [u8]) }
```
Rendering reads state only.
## Tests
- [ ] First frame's picture matches the oracle's within tolerance.
- [ ] Across a scroll in all directions.
- [ ] Status bar with T00's values; digits for each value range it shows.
- [ ] Character at every animation frame in a run.
- [ ] Palette as read from the executable.
## Done when
- [ ] Picture cases pass on clear stretches, with the differing pixels' report empty apart from named exclusions.
- [ ] `amiga-chips` admitted to purity's `ALLOWED` with why-pure lines.
- [ ] A worker opened frames from both sides; recorded as worker-driven.
