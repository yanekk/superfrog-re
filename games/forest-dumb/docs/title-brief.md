# Brief for `pir plan`: the title screen and menus of Forest Dumb Forever

Written 2026-11-20 for the planner. Slug: `forest-dumb-title`. The sequence around this plan is in
[ROADMAP.md](ROADMAP.md), which places it at any point after level 1, independent of the level plans.
Read that, `games/forest-dumb/CLAUDE.md`, [07-executable-map.md](07-executable-map.md) "The title screen's
code path", [14-translator.md](14-translator.md) (the app, the drill, "Open after the plan") and
`plans/forest-dumb-translator/REPORT.md` first.

## The goal

Everything the owner can do before and between the levels behaves as the original, judged by machine:
the disk 2 prompt, the title and its music, the three rows (START, KODY, KONIEC), entering a level code,
and where the game goes back to the title. Each has a case on the translated program against the
original, and an `fd-e2e` test drives it in the app.

## What changed since the roadmap

The roadmap (2026-10-31) expected this plan to build the title and menus. The translator built them: the
app runs the translated original from power-on, title and menus included, with its music. So this is
mostly a **checking plan**: write the cases the title never had, run them, and fix what fails by pattern in
the translator or the machine (translator DESIGN §2.2), never by a hint or an edit to generated Rust. No
hand-written title is expected (Open, 1).

## Where it starts

- `first-case` and `power-on-title` pass: power-on to the idle title, 280 frames of it, memory, picture and
  writes equal. Their notes still say `Fails today` (stale since T47).
- The drill (translator T41, worker-driven) found KODY's digit entry equal to the original and checked
  ten spike deaths to game over and restart, but no case holds either.
- KONIEC quits the app (owner, T41: the program's own return from its entry). Esc at the title "looks dead"
  (`docs/07`: the OS's keyboard handler is in place there), read, not run.
- `title_code_world` writes `world_index` and `cheat_flag` from a code; `cheat_flag` lets HELP end a
  level (`docs/09-play-measured.md`). Which codes exist and what each does is read from neither the code
  nor a run yet.

## Stages, in order of work

| Stage | Delivers | Proven by |
|---|---|---|
| The read | The title's state, rows, KODY's digit entry and the code table, read from the code and measured on the oracle; every code and its effect listed in `docs/` | Each listed code entered on the oracle does what the list says |
| Cases | One case each: the idle title past the song's loop; moving between the rows; KODY with each world's code, the cheat code and a wrong one; KONIEC; Esc; game over back to the title | Each case runs on the oracle and records a goal |
| Fixing | Every case passing on the translated program, each fix general | `fd-conform run-all` on the title cases; the 50 existing cases unchanged |
| The app | The same paths driven by key presses in the real app | `fd-e2e` tests, equal to a headless run each step; the short ones in smoke |

The planner orders the tasks and may run them side by side.

## Settled by the owner

- Faithful to the original, judged by machine on the oracle, never by the owner playing or looking
  (`knowledge/self-testing-faithful-bar.md`). The cracked `[cr WT]` set is the measuring stick, crack
  intro aside: the app starts at the program's entry, as today.
- The frame bar is the translator plan's DESIGN §2.7 as it stands, with its four owner allowances. No new
  allowance without the owner.
- KONIEC quits the app (T41). The app's other menus (Game ▸ Start Level…, Controls…) stay as they are.

## Reuse

| Need | Start from |
|---|---|
| Power-on and title cases | `cases/power-on-title.toml`, `first-case.toml`, `conform/scripts/boot-to-title.txt`, `boot-to-level1.txt` |
| Starting from the idle title | `world-2-start.toml`'s snapshot at frame 4990 |
| App driving | `fd-e2e`'s `translated_drill.rs`, `translated_short.rs`, `app_stop.rs` |
| Reading the code | `tools/re_listing.py`, `re/symbols.toml`, `tools/translate.py` for a routine's judge line |

## Known gaps to plan for

- The OS's interrupts run through the title on the original and never on the machine (translator DESIGN
  §2.8); the title's pace comes from the raster wait. A case that runs the title long may meet it.
- A code that jumps to a later world reaches play the level plans have not checked. Its case judges the
  world's start as `world-N-start` does (300 passes), no further.
- `--level 1`'s headless run through the title exits 1 with no zip on a stop (T39).
- Smoke is over its 15 minutes: long title cases go in the full set with a short test in smoke.

## Open, for the planner to settle with the owner

Proposals only.

1. **Hand-written title or translated only.** Proposed: translated only; nothing about the title needs Rust
   of its own.
2. **Level codes in the app.** Should Game ▸ Start Level… offer the worlds the codes reach, or the codes stay
   only on the title as in the original? Proposed: only on the title; Start Level… unchanged.
3. **The cheat code.** Proposed: in scope, case and test, since it is the original's; HELP ending a level is
   checked once, in W1-1.
4. **Size.** If the read finds the title small (few codes, no hidden screens), this could run as a few
   `@repo/single` changes rather than a plan. Proposed: a plan, since cases, fixes and app tests span
   several sessions; the planner says if the read shows otherwise.

## Out of scope

The levels' own play (each level's plan), the ending after world 6, the crack's intro and its picture,
sound beyond the title's music (the sound plan), widescreen.
