# T133 — w2-1-fruit-room

**Phase:** 2 · **Depends on:** T105; blocks T21 · **Weight:** medium

Added by T105 (owner, 2026-10-14). T105 covered W2-1's extra life 63,35 (`items-life-w2-1`) but
found no way on the original into the room holding the 20 000-point fruit 48,22 (handler
h0+$f008), so no case takes that handler.

What T105 learnt about the room (markers from the map; walls are `$3d`, floors `$3f`):

- The room is cols 45-51, rows 14-26 (floor row 27). It joins two places, both through `$26`
  fake walls: the corridor at rows 16-19, cols 52-79 (floor row 20, walls under it), which ends
  over shaft D (cols 80-84, rows 16-35); and shaft A (cols 60-63) through rows 22-24, cols 52-59.
  The coin trails in D and A zig-zag down: the level looks designed to be fallen through from the
  corridor, not climbed into.
- D is entered from the pad 91,35's column (cols 89-93, closed at row 7) through the fake wall at
  85-88, rows 20-23. The frog lands on its floor (row 24) pinned at x 1407 by the `$3d`s under
  it. A jump from there rises about 30 px, and the corridor floor is about 64 px up. No bounce off
  the pad was found that carries the frog left into D high enough (`examples/route_oracle`,
  beam 64, from frame 1082 of the route in T105's notes).
- Shaft A is entered by the lift up the pillar's column 63 (`items-life-w2-1`, frame ~895), but
  its walls (col 64-65) stop the frog from the side, and jumps inside the wall column 64 do not
  climb it (they stay at row 34).
- The ground probe lifts a frog up through `$3d`/`$3f` cells from below (T85's way into the
  pillar). A lift facing left between two `$3d`s hangs the original (DESIGN §2.11): a route must
  face right there.

## Goal

1. **Find a way into the room on the original**, with world 2's enemies live (stomping a flyer,
   a lift, anything the game allows), and add a W2-1 case taking the fruit 48,22 with
   `resync = false`, its take read through `sf_conform::run_with` as `items_life_w2_1` does.
2. **Or, if the room cannot be reached**, cover h0+$f008 with a 20 000-point fruit in another
   level that can, and record in `docs/` (the level docs the map editor will read) that W2-1's
   room at 45-51,14-26 is unreachable in play, with why.

## Files

`engine/conform/{cases,scripts}/` (a new case); `engine/crates/sf-conform/tests/frog_cases.rs`;
`docs/` if the room is unreachable; `engine/crates/sf-conform/examples/route_oracle.rs` if it
needs another target kind.

## Done when

- [ ] A case green with no re-sync takes a 20 000-point fruit (h0+$f008), its score compared on
      the take frame or the first compared frame after it.
- [ ] Either that case is W2-1's fruit 48,22, or `docs/` records that W2-1's room cannot be
      reached and why.
