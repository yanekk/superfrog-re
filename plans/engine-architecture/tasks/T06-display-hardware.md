# T06 — display-hardware

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Document the display imitation: the renderer runs the copper list cycle by cycle on the original's bus model, fetches bitplanes from the panel and the shown buffer, positions hardware sprites, and draws as Denise would; bobs are prepared and drawn as the blitter would; the front end uses its own area-mode blitter and sprite records. Separate the generic OCS imitation from what is specific to Superfrog's copper lists and buffers.

## Design sections this implements

DESIGN §2.3. Sources: `docs/12-display.md`, `docs/06-graphics-and-rendering.md`.

## Files

- `docs/architecture/display-hardware.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- `render/` (copper, playfield, panel, objects, view, mod), `level/bobs.rs`, `front/blit.rs`, `front/sprite.rs`.
- `Picture`, per-line palette, `View` and widescreen extra columns: what is Amiga output and what is the app's extension.
- Generic OCS (copper timing, bitplane fetch, sprite DMA, 12-bit colour) versus Superfrog-specific (its copper list layout, double buffer, panel split); this split drives the reuse rating.
- What the picture check in `sf-conform` compares, and so what a native renderer must reproduce (mid-line palette writes, fetch boundaries).
- The story's copper race (`front/story/cpu.rs`) is a finer instance; link `story-and-ending`.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: The generic OCS parts are likely `small-changes`; the chapter may give the rating for the whole and say which part is which.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/display-hardware.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
