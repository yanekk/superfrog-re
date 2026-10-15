# T141 — story-fade-colour-race

**Split from:** T56. Carries "that case is green" for the fades' colour races.

**Phase:** 2 · **Depends on:** T56 · **Weight:** medium

## Goal

The copper's read of a colour word that the handler's fade step rewrites sees what the original's
did: the step's writes (`vbl::colour_writes`, a component cleared then set six cycles apart)
timed to the cycle against the copper's read of that word (`render::story_screen`'s `word`).

## Found in T56

- Every remaining case is a 1–3 cycle race; the engine's writes are already within a few cycles.
  Note that `fade_probe` prints the *wait position* at which the copper first sees a value: colour
  `k` is read about `4k` cycles later, so add that before comparing with the engine's write times.
- Fade across (kind 1), scene 5's start: engine frames 3444, 3446, 3452 (oracle 6459, 6461,
  6467), colour 29's blue (original `$230`, engine `$236`): the original's copper read falls
  between the clear and the set; the engine's after the set. Oracle 6459 probe: colour 29 seen
  `230` from wait 31:$e2, `236` from 32:$04; engine writes 7377 (clear), 7383 (set).
- Fade in (kind 2), scene 9's start, colour 28, engine frames 7232–7241 (oracle 10 637 on):
  the original reads the old value, the engine the cleared one (probe: `000` from wait 32:$08).
- A text page's fade: engine frames 4071–4079 (colour 31, original `$0dd`, engine `$cdd`).
- Engine frames 4981–4983: colour 25 (`$222`/`$202`), likely the same kind.
- Both directions show the engine's write landing before the copper's read where the original's
  lands after: look for one bias (the read's cycle or the writes') before tuning per case.
- The handler's bus now holds the audio slots too (T56); it moved these writes by about 4
  cycles.

## Interface

None new.

## Files

`engine/crates/sf-core/src/front/story/` (`vbl.rs`, `mod.rs`); `render/` if the read time moves.

## Tests

- [ ] A unit test of the race at one of these frames (write and read cycles), failing before.
- [ ] `sf-conform scan` of `story-intro-full`: none of these frames differ; `story-intro` (the
  test block) and the `story-*` ignored cases stay green.

## Done when

- [ ] The listed frames match.
