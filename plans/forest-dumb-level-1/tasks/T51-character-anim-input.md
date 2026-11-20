# T51 — character-anim-input

**Phase:** 2 · **Depends on:** T07, T40 · **Weight:** medium

Added by T34 with the owner's yes (2026-11-05): T08's share was 21 routines, over DESIGN §4.3's 15. T08 waits for it (`blocks T08`).

## Goal
Port the character's animation-frame setters and the joystick read: the routines `re/translate.toml` gives T51 (`char_step_4be6`, `h0_4c28`, `h0_4c6a`, `h0_4d32`, `h0_4d54`, `read_joystick`, 112 instructions). Most arrive as passing literal ports from T40; this task turns them into engine code on typed state and proves each on the rig, so T08's `char_step` calls them ported.
## Design sections this implements
§4.1, §4.3.
## Reuses
T40's ports and table (`docs/11-translation.md`), T33's service for a missing or failed port, T32's rig and T53's generators, T00's note (`docs/09-play-measured.md`: `read_joystick` reads `JOY1DAT` once a pass and appends to `input_log`), `amiga-input`.
## Files
`fd-core/src/character/{mod.rs,anim.rs,input.rs}`, `fd-core/src/ported.rs` (appends), `fd-core/src/layout.rs`, `fd-conform/tests/character_anim_routines.rs`, `re/symbols.toml`, `docs/` note, `smoke.toml`.
## Interface
The animation setters as functions on the character's typed state (`char_anim_ptr` and the frame tables they read), and `read_joystick` from a `FrameInput` to `joy_flags` and `input_log`. T08 builds `Character` on these and appends to `character/mod.rs`.
## Tests
- [ ] 10 000 generated states on each ported routine, through the rig, with T53's generator.
## Done when
- [ ] Every T51 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] The variables these routines touch are in `LAYOUT` and `symbols.toml`.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each fix carries a test that fails without it. Non-zero hits on the original are shown by T41 and T42.
- [ ] The routines translated by hand, and why, are listed in `docs/11-translation.md`.

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only. `fd-core/src/character/mod.rs` is created here and extended by T08.
