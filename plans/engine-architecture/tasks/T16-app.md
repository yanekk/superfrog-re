# T16 — app

**Phase:** 2 · **Depends on:** T01 · **Weight:** light

## Goal

Write the light chapter for `sf-app`: window, GPU scaling and CRT filters, audio output, input and gamepads, menus and dialogs, settings, save states, the bug recorder UI, level files, scripted runs. Mostly `shell`; the chapter calls out the Amiga imitation that lives here (PAL pacing at 49.92 Hz, CRT monitor shaders, the 4:3 port).

## Design sections this implements

DESIGN §2.3 (light), §2.1 (`shell`). Sources: `plans/rust-engine/DESIGN.md` §2.5–§2.9, §3.2.

## Files

- `docs/architecture/app.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Kind mix with the `amiga` parts named; reuse rating with what another game changes (title, menus, level-file format).
- How the app drives `Game` (pacing, `step`, `render`, Paula hand-off) and so what a native core must still offer it.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Likely `small-changes`.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/app.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
