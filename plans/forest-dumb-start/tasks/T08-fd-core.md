# T08 — fd-core

**Phase:** 5 · **Depends on:** T00, T06, T07 · **Weight:** medium

## Goal

The engine's first crate, and the smallest real port: the game's measured timing, a placeholder
scene for the idle title screen, and the one modelled variable T06 chose, produced purely from
per-frame inputs (DESIGN §2.8, §3.3). It is the frame parameterisation's consumer, the seed of
the engine, and the engine side of the first case. It plays nothing: no graphics, no sound, no
input handling beyond what the scene's contract needs.

## Design sections this implements

DESIGN §2.8, §2.9, §3.1 (the pure side), §3.3.

## Files

- `games/forest-dumb/engine/crates/fd-core/` — `Cargo.toml` (depends on `amiga-clock`,
  `amiga-input` by path), `src/lib.rs`, `src/timing.rs`, `src/title.rs` (the scene and the
  variable).

## Interface

```
/// The game's display timing, carrying the frame length T00 measured (DESIGN §2.9).
pub struct FdTiming;
impl amiga_clock::Timing for FdTiming {
    const TIMING: amiga_clock::DisplayTiming = /* frame_lines: the measured value, fetch_lines and
        friends as the map's evidence describes; the values the first case needs are frame_lines
        and honest defaults elsewhere */;
}

/// The placeholder scene: the idle title screen's state, advanced once a frame.
pub struct TitleScene { /* per DESIGN §2.8 */ }
impl TitleScene {
    pub fn new() -> Self;
    /// Advance one frame. `input` is the frame's input (amiga-input's FrameInput), which the
    /// idle title ignores except where the map's evidence says it matters.
    pub fn frame(&mut self, input: FrameInput);
    /// The modelled variable, exactly as the map's evidence describes its behaviour.
    pub fn variable(&self) -> /* T06's type */;
}

/// Where the harness reads the same variable from the original's memory (T06's address,
/// hunk-relative, resolved by fd-conform as Superfrog's LAYOUT is).
pub const LAYOUT: &[(&str, u32)] = &[("<variable name>", /* T06's hunk-relative address */)];
```

If the variable's model needs a table over the purity limit, the table is passed into
`TitleScene::new` (or a `with_table` constructor) by `fd-conform`, which loads it from the disks;
`fd-core` never reads a file.

## Tests

- [ ] `FdTiming::TIMING.frame_lines` equals T00's measured value (asserted, with a comment
      pointing at the FINDINGS row).
- [ ] The scene is deterministic: two runs fed the same input produce the same variable every
      frame.
- [ ] The variable behaves exactly as the map's evidence describes: the right start value, the
      right changes at the right frames, the right wrap or range.

## Done when

- [ ] `fd-core` builds in the engine workspace against the parameterised toolkit, its tests are
      green, and the variable's behaviour matches the map's evidence to the letter.
- [ ] The crate reads no clock, file or network (its dependency list is `amiga-clock` and
      `amiga-input` only); the workspace purity test arrives in T09 and must pass it.

## Outside actions

None.
