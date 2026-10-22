# T09 — front-end

**Phase:** 2 · **Depends on:** T01 · **Weight:** heavy

## Goal

Document the front end outside the story: title, menu, password and level codes, high scores, game over, the attract demo, fades, front-end animation and the shared front-end code and data loading. Much of it ports the original's routines over its own memory and copper lists; the chapter separates what the player experiences (game logic) from how the original drew it (copper rewrites, sprite records, blits).

## Design sections this implements

DESIGN §2.3. Sources: `docs/13-front-end.md`.

## Files

- `docs/architecture/front-end.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- Everything under `front/` this chapter owns (not `story/`, `ending.rs`, `chain.rs`, `vbl.rs`, `blit.rs`, `sprite.rs`): `mod.rs`, `title.rs`, `menu.rs`, `password.rs`, `hiscore.rs`, `gameover.rs`, `attract.rs`, `fade.rs`, `anim.rs`, `common.rs`, `data.rs`, `hero.rs`, and `sf-core/tests/front.rs`.
- Per screen: kind mix, mechanisms used, must-preserve list.
- The attract demo replays recorded input against the game: say what it depends on (RNG, timing) to stay in step.
- Password and level codes: game logic, but encoded as the original encoded them; what must stay exact.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Mostly `superfrog-only`; name any generic piece (fade engine, palette handling).
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/front-end.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
