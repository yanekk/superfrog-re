# T70 — playfield-contact

**Phase:** 5 · **Depends on:** T23; blocks T24, T46 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T70` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T23 with the owner's yes (2026-11-06). In the original a life is lost only in `char_hit_check` (`h0+$3b2a`): it reads CLXDAT, the hardware's sprite against playfield collision (docs/10 "Contact"), counts `hits_taken` and on the third contact pass sets `char_dying` and calls `lives_minus_one`. `re/translate.toml` gives it to T24, which depends on T23, and the engine has no model of CLXDAT, so T23 could not show a pit losing a life. T23 built everything around it: `lives_minus_one`, the countdown, Esc, `main_loop`'s game over check, the game over screen and the restart.

## Goal
The engine loses a life where the original does on W1-1's spikes and water: a model of CLXDAT from the engine's own sprites and play screen, and `char_hit_check`'s contact path (not the creature stomp, `h0+$3dd8`, which stays T24's).

## Design sections this implements
§2.4, §4.1.

## Reuses
T59's measurements and `object_claims.rs` (CLXCON `$37dd`, CLXDAT bits 1 and 2), `render::picture`'s sprite and bitplane fetch (`amiga-chips`), T23's `lives_minus_one` and death cases, T40's literal port of `char_hit_check`.

## Files
`fd-core/src/character/death.rs` (the contact path), `fd-core/src/render/` (the collision model), `fd-core/src/level/mod.rs` (the call in `game_frame`), `fd-core/src/ported.rs` (append), `fd-conform/tests/death_routines.rs`, `engine/conform/cases/death-*.toml`, `fd-conform/tests/death.rs`, `smoke.toml`.

## Interface
None new beyond `Event`s a lost life may raise. First measure which frame's sprites and screen a pass's CLXDAT read sees (the register latches through the frame and clears on read).

## Tests
- [ ] The collision model against the original's CLXDAT over a W1-1 run past the spikes and the water, frame by frame.
- [ ] `char_hit_check`'s contact path on the routine rig, 10 000 states, creature slots inactive.
- [ ] Falling into the spike pit loses a life as the original: `death-pit` case, variables and picture, through the hurt frame and the hold; the character is not moved (the original has no respawn).
- [ ] The last life lost on the spikes enters the game over screen as the original (T23's runner end at the high scores).

## Done when
- [ ] Cases pass; `stretch-pit` runs on past frame 6522 or says why not.
- [ ] Follows `toolkit/docs/porting-routines.md`; `PORTED` entry for `char_hit_check` with T24 left the creature half, named in docs/11.
- [ ] Engine functions written by Muse through `tools/reshape.py --task T70 --all` and a `[T70]` table in `re/reshape.toml`, as docs/11 "Reshaping by Muse".
