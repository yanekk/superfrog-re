# T180 — faithful-compare

**Phase:** 2 · **Depends on:** T120; blocks T181, T182 and every open conformance task · **Weight:** medium

Added by the owner's amendment of 2026-10-15 (DESIGN §2.1 "What faithful means").

## Goal

Make `sf-conform`'s default judgement DESIGN §2.1's checks 1–3 (state pass by pass, picture within
tolerance, sound as events), keeping today's frame-exact comparison behind `--strict` for diagnosis.
After this task, "green" in any task doc means faithful.

## Design sections this implements

DESIGN §2.1 checks 1–3, §4.

## Files

- `engine/crates/sf-conform/src/{compare.rs,report.rs,main.rs,case.rs}`; a new `sound_events.rs`.
- `engine/crates/sf-core/src/layout.rs`: a `presentation` tag on variables that only say how a frame
  was produced (copper lists, bitmaps, blitter/beam bookkeeping, sound-driver register shadows).
- `engine/conform/README.md`: the faithful checks, `--strict`, and the printed tolerance counts.
- `engine/crates/sf-conform/tests/faithful.rs`.

## Interface

```rust
pub enum Mode { Faithful, Strict }                       // `--strict` selects Strict; default Faithful
pub struct Tolerance { pub colour_step: u8 /* 1 */, pub few_pixels: u32 /* 64 */,
                       pub glitch_frames: u32 /* 2 */, pub glitch_budget_pct: u32 /* 1 */,
                       pub sound_frames: u32 /* 2 */ }  // DESIGN §2.1; defaults are the owner's bar
pub enum SoundEvent { Start { ch: u8, addr: u32, len: u16, period: u16, vol: u8 },
                      Period { ch: u8, period: u16 }, Volume { ch: u8, vol: u8 }, Stop { ch: u8 } }
pub fn sound_events(writes: &[PaulaWrite]) -> Vec<SoundEvent>
```

The tolerance values are not per-case settings. A case cannot loosen them.

A sound event's volume is the one Paula plays (`AUDxVOL`'s low 7 bits, 64 and above as 64), so a
write that sounds the same compares equal (the `clr.w AUD3VOL` bus words of T131/T146: `$f4fd`
against `$33fc` is 64 both times; `$0605` against `$33fc` is 5 against 64 and still differs).

No variable a gameplay rule reads (`obj_slots`, `hitboxes`, `level_map`, the frog's, the counters)
is tagged `presentation`; each tag carries a one-line reason in `layout.rs`, and the README lists
the tagged variables.

Matching by pass count needs a pass count on both sides: the original's from its main loop parking
in `level_frame_loop`'s wait (`sync::mid_pass`), the engine's exposed by `Game` (outside a level,
where the main loop waits every frame, a pass is a frame). The frog, camera and clock run at 50 Hz in
the vertical blank (docs/08), so two pass ends a frame apart differ by a tick in those variables.
With the original's pacing fed, pass ends should fall on the same frame on both sides; the worker
measures whether they ever do not, and if they do, raises with the owner or coordinator how the
50 Hz variables are compared there rather than choosing.

## Tests

- [ ] Unit: a one-step colour difference on many pixels is tolerated; 64+ pixels of a wrong sprite
      for 2 frames is tolerated and counted, for 3 frames fails; the 1 % budget fails when exceeded.
- [ ] Unit: `sound_events` from synthetic write lists; the same events with writes reordered inside a
      frame, or shifted by 2 frames, match; 3 frames or a different sample fails.
- [ ] Unit: state compared by pass count: a pass ending one frame later on one side is equal (its
      50 Hz variables as the note above settles); a `presentation` variable differing is not a state
      difference.
- [ ] T120's guards hold in faithful mode: a case comparing nothing, or past `LAG_LIMIT`, fails.
- [ ] A deliberately broken mechanic (e.g. the frog's jump height off by one) still fails a frog case
      in faithful mode: the loosening did not blind the checks.

## Done when

- [ ] `run`, `all` and `scan` judge faithfully by default and print, per case, tolerated picture
      frames, tolerated sound shifts and re-syncs; `--strict` reproduces today's results.
- [ ] Every case in the test block is still green, or its red is a FINDINGS row naming T181.
- [ ] README updated.
