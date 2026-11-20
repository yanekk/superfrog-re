# Brief for `pir plan`: level 2 of Forest Dumb Forever

Written 2026-11-20 for the planner. Slug: `forest-dumb-level-2`. The sequence around this plan is in
[ROADMAP.md](ROADMAP.md). Read that, `games/forest-dumb/CLAUDE.md`, [14-translator.md](14-translator.md)
(its "Open after the plan" above all), [11-translation.md](11-translation.md) "The procedure",
`plans/forest-dumb-translator/REPORT.md` and `plans/forest-dumb-level-1/FINDINGS.md` first.

## The goal

The owner opens the app, plays through to world 2 or starts it directly, and plays `W2-1` from start to
end: its collectibles, enemies, special mechanics, deaths and the level's end into world 3's load, all as
the original does. A test plays it from start to end, and the level's cases pass against the original.

This is the first level plan of the new kind (ROADMAP, "What a level plan is now"): **the translated
program is the engine.** The plan does not port W2-1 routine by routine. It writes W2-1's cases, runs
them on the translated program against the original, and fixes what fails in the translator or the
machine by pattern, never by a hint, an address special case or an edit to generated Rust (translator
DESIGN §2.2). Hand-written Rust replaces a translated routine only where the owner chooses (Open, 1).

## Where it starts

- `world-2-start` passes: world 2's load, its SCENA screen and 300 passes of play equal to the original
  (translator T33, T53). `translated_whole.rs` plays the app from launch through level 1 to 300 steps of
  world 2. Beyond that stretch W2-1 has never been compared.
- Level 1's hand engine is mounted only in W1-1 (`--mount level-1`). Every other world runs translated.
- 49 of 50 cases pass. `power-on-level-1` fails from frame 6301 at `h0+$1fe6` (engine $80, original
  $f1), unexplained, on the translated program alone.

## Stages, in order of work

| Stage | Delivers | Proven by |
|---|---|---|
| Carry-over | The translator plan's open defects a level plan meets: `task_pack.py` takes the plan from its argument, not `PLAN = 'forest-dumb-level-1'`; `power-on-level-1`'s 6301 difference found and fixed by pattern; the two power-on cases' stale `Fails today` notes; `whole::heard` counting a held sample as sound | The power-on case passes; a W2-1 task gets a pack; a silent run fails the sound check |
| The level read | W2-1's map, objects, enemies and mechanics read from its files and the code, as level 1's were (`docs/10-object-system.md`); what differs from W1-1 named | A rendered W2-1 with its objects, compared with the oracle's screen |
| Cases | W2-1's cases on the original, the kinds level 1 has: stretches, camera, each death, collectibles, each enemy kind, each mechanic, the level's end | Each case runs on the oracle and records a goal |
| The route | A route through W2-1 by `fd-plan`, shown by `fd-routeview`; the owner plays a stretch where it stalls (Superfrog's rule, `games/superfrog/docs/planner-stall-ask-owner.md`) | The route reaches the level's end on the original |
| Fixing | Every case passing on the translated program, each fix general | `fd-conform run-all` on W2-1's cases; the whole-tree judge and level 1's 50 cases unchanged |
| The app | W2-1 played end to end in the app, its sound included | An `fd-e2e` test plays the route from launch or from world 2's start, equal to a headless run every step |

The planner orders the tasks and may run them side by side. Carry-over goes first: the pack fix gates
every later task, and the 6301 difference may be the same fault W2-1's cases would meet.

## Settled by the owner

- Faithful to the original, judged by machine on the oracle, never by the owner playing or looking
  (`knowledge/self-testing-faithful-bar.md`). The cracked `[cr WT]` set is the measuring stick.
- The frame bar is the translator plan's DESIGN §2.7 as it stands at its close, with its four owner
  allowances (pass-end judging with one frame of slack, other custom writes a pass early or late, the
  OS's stretch after a load, a byte one frame late). No new allowance without the owner.
- Translate first; Haiku writes generators and drafts chosen rewrites (owner 2026-11-12, translator T44).
- One plan a level. Do not generalise for W3-1 to W6-1 beyond what W2-1 needs; what W2-1 proves shared
  is written at the lowest tier where it is true.
- The app has sound now (translator DESIGN §2.10); W2-1 plays its music and effects as the original.

## Reuse

Root CLAUDE.md, "Before building something", binds this plan. Every task doc names what it reuses.

| Need | Start from |
|---|---|
| Cases and the runner | `games/forest-dumb/engine/conform/cases/`, its README; `world-2-start.toml` for starting in world 2 (`set` at `load_world`) |
| Translating and judging | `tools/translate.py`, `fd-judge`, [11-translation.md](11-translation.md) |
| Route and map page | `fd-plan`, `fd-routeview`, level 1's plan (`plans/forest-dumb-level-1/`) |
| A hand rewrite, if chosen | the seam (`fd-translated/src/seam/`, `fd-judge --seam`), level 1's mount and its timing walk (DESIGN §2.9) |
| App and its tests | `fd-app`, `fd-e2e` (`translated_whole.rs`, `translated_short.rs`) |
| Level files | `tools/render_levels.py`, [LEVEL-INDEX.md](LEVEL-INDEX.md), `docs/05`, `docs/06` |

Level 1's hand engine and Superfrog's game logic are patterns for how to work, never sources for how W2-1
behaves.

## Known gaps to plan for

From the translator's "Open after the plan"; each needs a task or a named home if W2-1 meets it.

- Translator gaps outside today's census: `subx`/`negx` flags with X, register-count rotates past the size,
  `asr` by a register, `mulu`/`divu` with a memory source (`Stop::Unsupported`). W2-1's path may reach code
  level 1's did not.
- The interrupt is seen `IPL_DELAY` late at an instruction's last access, not at §2.8's boundary; `seam::walk`
  times every `dbcc` as `dbf`; `DEPTH_CAP` rests on static depth. Latent; a W2-1 case may expose one.
- The smoke set is already over its 15 minutes. W2-1's long cases go in the full set with a short smoke
  test beside them; ask the owner if the crucial ones do not fit.
- Conversation logs: keep them where cleanup does not reach, as level 1's brief asked.

## Open, for the planner to settle with the owner

Proposals only.

1. **Hand-written W2-1 or translated only.** Level 1's hand engine was built before the translator existed.
   Proposed: translated only, no hand rewrite in this plan, unless a routine cannot be made to pass by a
   general fix; then the owner chooses that routine.
2. **How the owner reaches W2-1 in the app.** Proposed: `--level 2` and Game ▸ Start Level… offer it, reached
   as `world-2-start` does (the title's Start with `world_index` set at `load_world`), so no new start path.
   The title's level codes (KODY) are the title plan's.
3. **What "done" counts.** Proposed: every W2-1 case passes translated, level 1's 50 stay as they are, and the
   end-to-end test reaches world 3's load.

## Out of scope

Levels 3 to 6, the title screen and menus (its own plan, [title-brief.md](title-brief.md)), the ending,
the whole-game test, map editing, widescreen.
