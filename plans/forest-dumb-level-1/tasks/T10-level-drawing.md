# T10 — level-drawing

**Phase:** 2 · **Depends on:** T08, T09, T40 · **Weight:** medium

## Goal
Draw the level, character and status bar as the original does: the `drawing` routines `re/translate.toml` gives T10. Most arrive as passing literal ports from T40; this task turns them into engine drawing and proves each on the rig. Picture agreement over runs is T44's (DESIGN §4.3).
## Design sections this implements
§2.6, §4.3; `docs/06`.
## Reuses
T40's ports and table, T33's service for a missing or failed port, T32's rig and T37's generators, `amiga-chips` (`Planes`, `Colours`, `copper::run*`, `Blit`), `sf-core/src/render/` as pattern, T04's `picture_in`, `Picture::rewindow`. Sprite composition moves to the toolkit only if T00 found hardware sprites.
## Files
`fd-core/src/render/{mod.rs,playfield.rs,status.rs,objects.rs}`, `fd-core/src/ported.rs` (appends), `fd-conform/tests/{drawing_routines.rs,purity.rs}`, `smoke.toml`.
## Interface
```
impl Game { pub fn picture(&self) -> Picture; pub fn picture_rgba(&self, out: &mut [u8]) }
```
Rendering reads state only.
## Tests
- [ ] Routine conformance on 10 000 states, custom-chip writes included, with T37's generators.
- [ ] Palette as read from the executable.
- [ ] The first frame's picture is drawn without panic and has the status bar's rows (agreement is T44's).
## Done when
- [ ] Every T10 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] `amiga-chips` admitted to purity's `ALLOWED` with why-pure lines.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it. Non-zero hits on the original are shown by T44.
- [ ] The display buffers are engine state or drawn from the map as T00's answer says, and the choice is written in the render note.

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
