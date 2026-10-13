# T42 — story-intro-scenes

**Phase:** 2 · **Depends on:** T32; blocks T21 · **Weight:** heavy

Split from T32 (owner, 2026-10-13): T32 got the logo and the intro frame-exact to frame 539 (the
first scene's play, case `story-intro`); the rest of the scenes are this task.

## Goal

Every frame of the disk-3 intro, untouched, from frame 540 to the "insert disk 1" page, matches
the original booted from disk 3 (picture, Paula writes, the story's variables), loads re-synced as
T32 does.

## Design sections this implements

DESIGN §1, §2.1, §2.3.

## Where T32 stopped (its debugger's scan to frame 4000)

- **540–553, 1244–1256, 1758–1772: colour race.** A fade step's colour writes against the copper's
  read: the original showed `$332` where the engine `$333`. T32 drafted the three loops' 68000
  cycles (in h0+$b7d2, out $ba58, across $b5d6): a component's body runs on the compare (in:
  colour ≠ target; out: ≠ 0), not on a changed value, and writes twice (`andi` clears, `add`/`or`
  sets). The draft moved frame 539: the runner's load-line search fits on the picture, so fade
  timing in a load frame changes the line found. Reverted; start from that.
- **2257–2269, 2865–2879, 3071–3097, 3295–3337**: picture misses in later scenes (set-up and draw
  timing; scenes 4–9 unpack from h1/h0; a scene with bitplanes on during `objects_init`'s CPU
  clear, which `BlitClock::cpu` does not slow).
- **3185–3207, 3432–3631, 3741–3832**: DMACON `$8400`, INTENA `$c000` and the sound's writes
  early or missing; then the effects' volume ramps a step apart (3872–3990).

## Interface

None new: `Start::Disk3` and the sync point `power-on-disk3` as T32 left them.

## Files

- `engine/crates/sf-core/src/front/story/` (`vbl.rs` fade timing, `intro.rs`, `clock.rs`,
  `scenes.rs`, `sound.rs`).
- `engine/conform/cases/story-*`; `docs/13-front-end.md` (the intro section's timings).
- `engine/crates/sf-conform/examples/story_dbg.rs` — T32's divergence finder (`SCAN=1`).

## Tests

- [ ] `story-intro` extended (or a second case) from the intro's start to the "insert disk 1" page.

## Done when

- [ ] The case is green; the test block keeps one short intro case.
- [ ] docs/13's intro timings cover every scene.
