# T08 — game-flow

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Document the top of the game: `Game` as the decision function (`new`, `step`, `step_paced`, `render`, `probe`, `save`, `load`), the scenes and how a frame's scene is told from its copper list, the fixed sequence of calls per world and level (`front/chain.rs`), the original's RNG, save states and bug recordings. Separate the game's own flow from the Amiga machinery it is expressed in (disk prompts and swaps, overlay loads).

## Design sections this implements

DESIGN §2.3. Sources: `plans/rust-engine/DESIGN.md` §3.3, §2.8; `docs/08-executable-map.md` "Scene flow", "RNG", "Disk swaps"; `plans/bug-recorder/DESIGN.md`.

## Files

- `docs/architecture/game-flow.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- `game.rs`, `scene.rs`, `lib.rs`, `rng.rs`, `recording.rs`, `savestate.rs`, `front/chain.rs`.
- `SceneKind` told by copper list: an Amiga-shaped classifier used by conformance milestones and the bug recorder; say what a native rewrite must give them instead.
- `DiskPrompt` and disk swaps: the original's floppy behaviour kept as game flow; classify it.
- RNG: game logic, but proven against the original's routine; what must stay bit-exact.
- What in `game.rs` belongs to `cpu-time-and-beam` (`Cpu`, `Job`, `PassClock`, `CODE_TIMES`): describe briefly and link, do not duplicate.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Rate the flow (Superfrog-specific) and note the generic parts (`Game` shape, save-state header, recording format).
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/game-flow.md` follows the template, front matter complete, and the
      test command passes; no other file under `docs/architecture/` changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
