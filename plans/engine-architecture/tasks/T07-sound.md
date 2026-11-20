# T07 — sound

**Phase:** 2 · **Depends on:** T01 · **Weight:** medium

## Goal

Write two chapters. `sound-chip`: `sf-paula`, the Paula channel model at the Paula clock and the resampler to the host rate, the `PaulaWrite` interface the game emits, and the beam-position gap behind T213. `sound-driver`: the game's music replayer (Player 4.1A), effect envelopes and the channel-3 streamer in `sf-core/src/audio/`, whose state is the original's own hunk-0 bytes. The first is the most reusable piece of imitation in the project; the second is the most likely to be shared with other Team 17 games.

## Design sections this implements

DESIGN §2.3. Sources: `docs/11-sound.md`; rust-engine DESIGN §3.1 (the `resample.rs` floating-point exception); `plans/rust-engine/tasks/T213-paula-write-timing.md`.

## Files

- `docs/architecture/sound-chip.md` (replacing the T01 stub)
- `docs/architecture/sound-driver.md` (replacing the T01 stub)
- `plans/engine-architecture/FINDINGS.md` (rows only)

Edit no other chapter, and do not change any chapter's `owns`: an ownership that looks wrong goes
in `FINDINGS.md` for T18 (DESIGN §2.7). No code changes.

## What the chapter must cover

- `sound-chip`: channel behaviour (DMA, period, volume, interrupts if modelled), what is exact and what is approximate (the resampler), what the per-channel conformance taps compare.
- `sound-driver`: how much is Player 4.1A (generic, used by other games) versus Superfrog's effect tables; its byte-exact state and why (`audio/mod.rs` header).
- The interface between them: `PaulaWrite` and the missing beam position (T213): what a native rewrite of either side must keep.
- Player 4.1A reuse for other Team 17 games: flag, marked unchecked.
- Front matter: `kind`, `mix` (with one sentence in the body on how it was judged), `reuse`.
- Reuse: Two ratings, one per chapter.
- Cite code as repo paths and item names, and link the RE notes rather than restating them.

## Tests

- [ ] the test command passes with this chapter's front matter complete (no `tbd`)
- [ ] every path the chapter cites exists (the drift check does this)
- [ ] the reviewer samples at least five claims, including every "must preserve" row, against
      the code and finds them true

## Done when

- [ ] `docs/architecture/sound-chip.md` and `docs/architecture/sound-driver.md` follow the template,
      front matter complete, and the test command passes; no other file under `docs/architecture/`
      changed.
- [ ] The chapter covers every point under "What the chapter must cover".
- [ ] Every Amiga-shaped part has a "must preserve" entry naming an existing check that proves it,
      or says plainly that no check covers it.
