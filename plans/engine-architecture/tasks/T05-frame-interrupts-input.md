# T05 — frame-interrupts-input

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Document the order of one frame as the original runs it and the engine copies it: the vertical-blank interrupt, the copper interrupt the scroller asks for, the main loop's pass every second frame, and what each runs (sound, keyboard, clock, fades); plus input as the original sees it (joystick bits, the CIA keyboard and its handshake timing). A native game loop would naturally collapse this order, so the chapter says what the order decides.

## Design sections this implements

DESIGN §2.3. Sources: `docs/08-executable-map.md` "Main loop and interrupt", "Input"; `docs/13-front-end.md` "The keyboard".

## Files

- `docs/architecture/frame-interrupts-input.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- `level/frame.rs`, `level/clock.rs`, `front/vbl.rs`, `input.rs` in full; the story's `front/story/vbl.rs` as a second instance (owned by `story-and-ending`).
- Which state each interrupt touches, and where the 25 Hz pass and the 50 Hz interrupt disagree in a way the game shows (a value read half-updated, an input sampled in the interrupt and acted on in the pass).
- Input: `FrameInput`, how keys reach the game, and which of it is Amiga behaviour (CIA, raw key codes) versus game logic (what a key does).
- The must-preserve list with the check behind each.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Rate for another Amiga game: the interrupt/pass skeleton is common; the handler contents are per game.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/frame-interrupts-input.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
