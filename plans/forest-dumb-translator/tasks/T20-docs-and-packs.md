# T20 — docs-and-packs

**Phase:** 3 · **Depends on:** T18, T19 · **Weight:** medium

## Goal

Make "translate with the translator first" what a session meets wherever it starts (DESIGN §2.12): the game's `CLAUDE.md`, the translation guide, the tools README and every task pack. Muse's procedure becomes history; Haiku's role is written as T19 left it.

## Design sections this implements

DESIGN §2.12, §2.13; the owner's tier decision (DESIGN §7).

## Files

- `games/forest-dumb/CLAUDE.md` (Notes and Tools lines, a "Turning the original's code into Rust" section near the top).
- `games/forest-dumb/docs/11-translation.md` (the procedure rewritten: translate, judge, rewrite only where chosen, the seam; Muse sections moved under "History").
- `games/forest-dumb/tools/README.md`, `tools/task_pack.py` (`reshape_how` and the listings text replaced by the translator procedure), `tools/test_task_pack.py`.
- `toolkit/PLAYBOOK.md` step 6: one `learned on:` line, "translate first, port by choice" (the tier-name check allows it in a marked line).
- `games/forest-dumb/docs/ROADMAP.md`: what a level plan now is.

## Tests

- [ ] `test_task_pack.py`: a porting task's pack holds the translator procedure and no Muse procedure.
- [ ] `tools/check_tier_names.py` passes.
- [ ] A doc test: no doc outside "History" sections and `spikes/` tells a session to call Muse.

## Done when

- [ ] Every file above says translator first; packs carry the procedure.
- [ ] The tier-name check and the doc test pass.
