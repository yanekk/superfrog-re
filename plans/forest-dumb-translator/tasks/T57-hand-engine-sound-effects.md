# T57 — hand-engine-sound-effects

**Split from:** T38 (coordinator, on the owner's behalf, 2026-11-19).

**Phase:** 7 · **Depends on:** T38; blocks T40, T42 · **Weight:** medium

## Goal

With level 1's hand engine mounted (DESIGN §2.9), the app's level 1 makes the sound effects the original makes, at the same passes, so the app plays sound in level 1 as DESIGN §2.10 asks. Judged by machine: the in-order Paula writes of each pass with level 1 mounted equal the original's, on the level cases.

What T38 measured (2026-11-19): level 1 in the app is silent. It has no music: every frame of play the sound driver points each channel at the silent buffer `$113e8` with length 1 (14 Paula writes a frame: INTREQ, AUDxLC, AUDxLEN for the four channels). Its sounds are effects started inside `game_frame` (`h0+$1fec`), which the frame mount replaces, and the hand engine starts none: `fd_core::objects::Io::sound` takes `Sound::Item`, `Time`, `Spring` and play drops them (`objects/mod.rs`, "the plan is silent"); the character's slide and stomp sounds (`h0_3ea8`'s AUD2 and DMACON on the slide's step 3, `h0_3adc`'s, docs/09 and docs/10) have no hook at all. A jump, a slide and fire in level 1 made no audible sample (`fd-e2e/tests/sound.rs`).

**Overlap with T56 (mount-routine-timing).** None of T56's listed differences is a Paula write: the level cases compare `vars` and `picture`, and only the frame runner's `writes` key compares Paula's writes, which no level 1 case lists today (`power-on-level-1` and the world starts do). The one overlap is `power-on-level-1`: mounted, its first difference is Paula's writes at pass 1045 (frame 6301), and it fails translated alone there too (T53). T57 does not chase that case's difference; T55/T56 own it until it is shown to be a missing sound effect.

Approach: the mount (`fd-translated/src/seam/frame.rs`) answers each sound the hand engine starts with the Paula writes the original's routine makes for it, at the beam position the original makes them (the writes the routine rig already compares for `Item`, `Time`, `Spring`), and the hand engine reports the slide and stomp sounds it lacks. Any line waits the sounds spend are T55's time, not this task's.

## Design sections this implements

DESIGN §2.9 (the seam), §2.10 (the app plays sound).

## Files

- `crates/fd-core/src/objects/mod.rs` and the character's ports: the sounds reported, never mixed (fd-core stays pure and silent on its own).
- `crates/fd-translated/src/seam/frame.rs`: each reported sound becomes its Paula writes on the machine.
- `conform/cases/*.toml`: `writes` added to the level 1 cases that hear an effect (at least an item, the spring, a slide, a stomp).
- `crates/fd-conform/tests/mount_level_1.rs`; `crates/fd-e2e/tests/sound.rs`.

## Tests

- [ ] Full: the level 1 cases given `writes` pass `--engine translated --mount level-1` on Paula's writes in order (cases still failing for T55/T56 reasons named, not counted).
- [ ] Full, `fd-e2e/tests/sound.rs`: in level 1 a sound effect is heard in the app at the step the headless run plays it, and the app's samples equal headless (T38's second end-to-end check, which T38 closed silent).

## Done when

- [ ] Both tests pass; FINDINGS says which effects were found and which case hears each.
