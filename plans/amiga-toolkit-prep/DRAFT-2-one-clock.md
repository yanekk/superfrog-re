# Draft 2 — one clock

**Status:** draft, 2026-10-23; owner decisions recorded 2026-10-24. Not reviewed. Input for `pir plan` (3 tasks).
Source: `docs/architecture/cpu-time-and-beam.md` §3–§7; native order step 2; rust-engine FINDINGS
T213, T216, T217 rows.

## The change

Five timing models, in three units, not four:

| Model | Where | Unit |
|---|---|---|
| Level `Clock` | `level/beam.rs:436` (`LINE_CYCLES` 454, fetch window `FETCH_*` const, `blitter_slot`, `blit_cycles` = vAmiga SlowBlitter) | 454/line, 313 lines |
| Front end `Cx` | `front/mod.rs:300` (own `FRAME_LINES`, `BEAM_CYCLES_PER_LINE`; `busy` rounds to whole lines; `blit_wait` 1.23/slot) | lines |
| Slot draw | `slot/draw.rs:444` `fn spend` (fitted "lost" per fetched word) | cycles, **312** lines |
| Story bus simulator | `front/story/sim.rs`, `cpu.rs` (cycle-exact DMA arbitration) | 227/line |
| Story `Intro::t` / story `Cx` | `front/story/intro.rs`, `front/story/mod.rs:96` | raster lines |

Plus `render::copper::Beam` (227/line). `level/timeline.rs` already bridges level `Clock` to `Cx`.

Change (minimal scope): one generic beam piece — a `BeamPos` type with unit conversions (454, 227,
Paula hpos), one set of frame and line constants, and a `Clock` parameterised by frame length,
fetch window and per-word steal, with `blitter_slot`, `blit_cycles`, `slots_per_word`, and
`Timeline` beside it — in its own crate. The level uses it with today's parameters. `Cx` and the
slot draw re-express their positions on `BeamPos` and keep their own rounding rules as named
policies. The story simulator stays its own cycle-exact engine and only adopts `BeamPos` and the
constants.

Tasks: T1 `BeamPos` and constants unified; T2 generic `Clock` crate, level pointed at it, `Timeline`
moved; T3 `Cx` and slot draw onto `BeamPos`/`Clock` policies, then a full conformance run.

## Decisions

- 2026-10-24: minimal scope, the shared clock only. Costs stay inline (`Cost`, its tables, every `spend`);
  turning the 89 level and 26 front-end charge points into timed events with a refit is a
  native-rewrite step, not toolkit preparation.
- 2026-10-24: `Cx` keeps its whole-line rounding as a named policy on the shared clock. Reason: its
  `AudioDma` positions are the coarse ones (rust-engine FINDINGS); a finer clock could move note
  restarts, which is audible and could disturb the T215/T216 channel judging. Any refit is a
  separate decision.
- 2026-10-24: this draft owns the blitter timing merge that draft 4 leaves out.

## Not changing

Every `Cost` kind, its tables (`COSTS`, `BLANK_COSTS`, `RESTART_COSTS`), every `spend` call site,
every fitted constant (`VBL_END_LINE`, `FADE_*`, `BLITTER_LOST`, the 1.23/slot), the slot's
312-line frame (a setting, never "fixed" to 313), the story's cycle-exact arbitration, the
cut-point types (`Cut`, `Pending`, `Frames`: draft 3 part C). Paula writes keep their beam
positions exactly.

## Done when

- One definition per meaning of frame length and line length (north-star M2 constants row: today 12
  engine-wide, 8 in sf-core; target ≤ 3, the probes and tests import them); one `BeamPos`; `Clock`
  lives in a crate with no sf-core dependency, listed in `PURE`.
- Every check below green, no tolerance changed.
- `docs/architecture/README.md` step 2 gets a "done (minimal)" line; FINDINGS notes the policies.

## Tests

- pir's test line runs `cd engine && cargo test -q --workspace` (owner, 2026-10-24); the builder also runs `cargo test -p sf-conform --test beam` (`the_cost_model_alone_puts_paula_writes_on_the_originals_lines`,
  70 %/98 % bands), `--test frog_cases`, `--test alone`, `cargo test -p sf-conform --test story --
  --ignored` (slot and story cases, 80 s–5 min each), `--test channels -- --ignored`
  (`the_whole_games_channels_sound_as_the_original_does`, ~7 min cached).
- T3 ends with `cargo run -q --release -p sf-conform -- all` (212 cases, ~2 h, T217's figure) and
  sf-e2e `whole_game.rs`.
- New: `BeamPos` conversion round-trip tests; `Clock` with Superfrog's parameters reproduces
  today's `level::beam` unit tests unchanged.

## Risks

- `Cx`'s fitted constants were tuned to its whole-line rounding; anything that changes the rounding
  shows as red front-end cases. The named-policy approach is there to avoid that.
- How many picture frames depend on display writes carrying a beam position is unmeasured
  (cpu-time-and-beam §5). Conformance pins the clock to the original's writes, so drift between
  pins is hidden; out of scope to measure here (~2 h `sf-conform all` with zeroed costs).

## Toolkit payoff

Generic: `BeamPos`, `Clock`, `blitter_slot`, `blit_cycles`, `slots_per_word`, `Timeline`;
`feed.rs`'s stepping order and hashes. Superfrog-only, stays in sf-core: `Cost` and its tables,
`routine_cost`, `Cut`/`Pending`/`Frames`, the fitted constants. Owns the blitter *timing* merge that
draft 4 leaves out. Moves north-star M2, M5.
