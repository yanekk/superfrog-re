# T00 wide-spawn-spike — results (2026-10-26)

Throwaway measurement for T03. The hack is `wide-spike.patch` (apply on `pir/real-widescreen` at
e99e2f8 with `git apply`); nothing under `engine/` is committed. Logs: `run-extra{10,20,30,40,60}.tsv`
(every level), `heavy-extra{50,60,80,100}.tsv` (the five heaviest), one row per level and mode.

## What the hack does

- A global switch `sf_core::spike::WIDE` (no `Screen`, no mode in `Game`). On: `scan_origin` 5 cells
  further left, `scan_edges`' right column and `scan_right_column` 5 cells further right, the row scan
  35 cells and `scan_screen` 34 per row (DESIGN §2.2); `obj_despawn_if_offscreen` right `$170+65`,
  left `$30+65`; `obj_free_if_left_behind` from `cx−65`. Vertical unchanged. Camera clamp, frog
  sprite bounds and the §2.6 objects (Spud, bonus shots, watchers, witch) not touched.
- `N` extra slots, bob records and free-list entries in a new LAYOUT range at `$600000` (never mapped
  by the original). Every slot loop in `level/objects.rs` and the five literal `(0..20)` loops in
  `enemies/` run over all `20+N`. An extra bob record is a copy of object bob record 1 when taken.
  The extras' updates, hitboxes, draw-list places, bob draws and the added scan cells spend no beam
  time (D11). Draw list capped at its 40 places (overflow counted, `draw_dropped`); hitbox list at 127.
- Counters: `find_free` returning none (`find_free_fail`) or an evicted slot (`evict`), the coin's
  force-out (`coin_forced`), no free bob record (`no_bob`), per-frame draw-list and hitbox entries.

## How each number was obtained

Runner `sf-plan/examples/wide_spike.rs`:
`cargo run --release -p sf-plan --example wide_spike -- ../plans/smart-planner/bench/routes/app conform/scripts [LEVEL…]`,
`SPIKE_EXTRA=N` sets the extra slots (default 60). Each level starts from `sf_plan::app::app_start`
(Start Level's defaults, the route's `# seed:`) and plays its inputs with `step_unseen`, to the route's
last input + 300 frames or until `level_state` leaves −2. Three runs per level:

- **4:3**: the hack off. Every route finishes as the bench says (the engine is unchanged with it off).
- **wide**: the hack on, the route's inputs as they are. Enemies differ, so most routes drift and the
  frog dies (`(died)`) or stops short (`(no end)`); this is the game as it plays, to that point.
- **ghost**: the hack on, with a 4:3 game stepped alongside and its frog block (`FROG..+$100`) and
  hearts copied into the wide game after every frame, so the wide camera travels the whole finishing
  route whatever the enemies do. These are the capacity numbers.

Per frame, after the step: live slots (`+$40 ≠ 0`, all slots). "In wide view" counts live slots with
x in `cx−$30−64−65 .. cx+$170+65` and y in `cy−$30−64 .. cy+$150`; the rest are stale objects the
camera left behind (chain links, spikes; `$262be`, `$25fa8`, `$2619c` updates), which the original
frees only by `find_free`'s eviction when it is full. Peaks are taken after each frame, not inside a
pass. Input sources: the smart-planner bench's app-timed routes (`plans/smart-planner/bench/routes/app/`)
for W1-1…W6-4; `bonus-fire.txt` with 60 lives poked (as `bonus-end-wb-1`) for WB-1; `final-win.txt`
for WW-1. The W6 alt maps were not run.

## Table (extra slots 60, the default, unless named)

| Level | Source | Frames (4:3 / wide / ghost) | Peak live 4:3 (frame) | Peak live wide (frame) | Peak live ghost (frame) | Peak in wide view, ghost (frame) | Peak draw list 4:3 / ghost (frame) | Peak hitboxes 4:3 / ghost (frame) | `find_free` fails / evictions 4:3 | same, ghost x60 | ghost fails at x10 / x20 / x30 / x40 | Panic |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| W1-1 | bench app route | 2844 / 2744 (died) / 2844 | 18 (1733) | 18 (1733) | 20 (2785) | 20 (2785) | 19 / 21 (2786) | 18 / 19 (2780) | 0 / 0 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W1-2 | bench app route | 4607 / 2604 (died) / 4607 | 12 (1036) | 15 (2391) | 15 (2858) | 15 (2858) | 13 / 16 (2858) | 12 / 15 (2858) | 0 / 0 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W1-3 | bench app route | 8612 / 3818 (died) / 8612 | 15 (823) | 15 (212) | 15 (212) | 15 (212) | 16 / 16 (212) | 15 / 15 (212) | 0 / 0 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W1-4 | bench app route | 10071 / 2500 (died) / 10071 | 20 (5730) | 17 (500) | 23 (5730) | 22 (5730) | 21 / 24 (5730) | 20 / 23 (5730) | 0 / 1 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W2-1 | bench app route | 2943 / 3316 (no end) / 2943 | 20 (884) | 22 (613) | 25 (2903) | 22 (613) | 21 / 26 (2904) | 20 / 22 (1598) | 1 / 4 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W2-2 | bench app route | 4183 / 4183 / 4183 | 18 (1621) | 27 (3930) | 27 (3930) | 20 (3930) | 19 / 29 (3932) | 18 / 18 (1606) | 0 / 0 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W2-3 | bench app route | 9222 / 2434 (died) / 9222 | 20 (4852) | 14 (1491) | 46 (8711) | 29 (7749) | 21 / 47 (8712) | 20 / 34 (8700) | 127 / 21 | 0 / 0 | 10 / 0 / 0 / 0 | none |
| W2-4 | bench app route | 9457 / 754 (died) / 9457 | 20 (465) | 21 (567) | 46 (9457) | 26 (3498) | 21 / 43 (9332) | 20 / 30 (9454) | 33 / 10 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W3-1 | bench app route | 3791 / 3791 / 3791 | 20 (251) | 46 (2861) | 46 (2861) | 36 (2861) | 21 / 47 (2862) | 20 / 27 (3738) | 174 / 5 | 0 / 0 | 49 / 0 / 0 / 0 | none |
| W3-2 | bench app route | 8377 / 854 (died) / 8377 | 20 (1398) | 19 (210) | 67 (8243) | 26 (7114) | 21 / 68 (8244) | 20 / 32 (7112) | 13 / 30 | 0 / 0 | 3 / 0 / 0 / 0 | none |
| W3-3 | bench app route | 6168 / 1178 (died) / 6168 | 20 (1799) | 18 (266) | 48 (4701) | 24 (2421) | 21 / 49 (4702) | 20 / 30 (4702) | 17 / 14 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W3-4 | bench app route | 15234 / 2544 (died) / 15234 | 20 (1268) | 27 (2037) | 42 (14269) | 27 (2125) | 21 / 41 (14270) | 20 / 30 (14270) | 38 / 12 | 0 / 0 | 99 / 0 / 0 / 0 | none |
| W4-1 | bench app route | 3249 / 3622 (no end) / 3249 | 19 (3144) | 19 (211) | 24 (3142) | 21 (3142) | 20 / 25 (3142) | 17 / 22 (3142) | 0 / 1 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W4-2 | bench app route | 8853 / 2761 (died) / 8853 | 20 (1) | 32 (1448) | 77 (7532) | 56 (7536) | 21 / 78 (7532) | 20 / 59 (7532) | 243 / 14 | 0 / 1 | 0 / 6 / 32 / 0 | none |
| W4-3 | bench app route | 5930 / 529 (died) / 5930 | 20 (943) | 17 (339) | 44 (4173) | 24 (4173) | 21 / 45 (4174) | 20 / 32 (4174) | 13 / 12 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W4-4 | bench app route | 14004 / 623 (died) / 14004 | 20 (703) | 16 (417) | 79 (8388) | 52 (8401) | 21 / 80 (8388) | 20 / 70 (8402) | 4 / 52 | 0 / 5 | 20 / 0 / 18 / 5 | none |
| W5-1 | bench app route | 2210 / 2583 (no end) / 2210 | 20 (1833) | 37 (2035) | 40 (1933) | 33 (1933) | 21 / 41 (1934) | 20 / 33 (1934) | 6 / 2 | 0 / 0 | 6 / 0 / 0 / 0 | none |
| W5-2 | bench app route | 5103 / 2416 (died) / 5103 | 20 (1306) | 19 (304) | 49 (4177) | 25 (4177) | 21 / 49 (4178) | 20 / 27 (4092) | 5 / 11 | 0 / 0 | 2 / 0 / 0 / 0 | none |
| W5-3 | bench app route | 8870 / 3214 (died) / 8870 | 20 (2840) | 16 (3094) | 45 (8517) | 26 (3879) | 21 / 45 (8498) | 19 / 24 (4650) | 3 / 29 | 0 / 0 | 4 / 0 / 0 / 0 | none |
| W5-4 | bench app route | 7124 / 2028 (died) / 7124 | 20 (2795) | 21 (1595) | 53 (6728) | 23 (2812) | 21 / 54 (6728) | 15 / 28 (6728) | 17 / 14 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W6-1 | bench app route | 8551 / 1417 (died) / 8551 | 20 (1070) | 23 (776) | 41 (3906) | 23 (1048) | 21 / 42 (3906) | 20 / 30 (3342) | 4 / 33 | 0 / 0 | 1 / 0 / 0 / 0 | none |
| W6-2 | bench app route | 8491 / 1146 (died) / 8491 | 20 (396) | 30 (388) | 80 (4843) | 23 (1132) | 21 / 81 (4844) | 20 / 51 (8484) | 68 / 71 | 0 / 11 | 74 / 21 / 36 / 21 | none |
| W6-3 | bench app route | 8400 / 1664 (died) / 1362 (died) | 20 (990) | 22 (1054) | 22 (1054) | 21 (1054) | 21 / 23 (1054) | 20 / 16 (1054) | 9 / 40 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| W6-4 | bench app route | 15084 / 1979 (died) / 15084 | 20 (779) | 23 (1372) | 54 (12187) | 26 (3769) | 21 / 55 (12188) | 20 / 54 (12188) | 26 / 60 | 0 / 0 | 23 / 0 / 0 / 0 | none |
| WB-1 | bonus-fire.txt | 8075 / 7917 / 7915 | 19 (2139) | 23 (144) | 23 (144) | 21 (144) | 20 / 24 (144) | 14 / 18 (144) | 0 / 0 | 0 / 0 | 0 / 0 / 0 / 0 | none |
| WW-1 | final-win.txt | 2264 / 2264 / 2264 | 1 (1) | 1 (1) | 1 (1) | 1 (104) | 11 / 2 (2) | 10 / 1 (2) | 0 / 0 | 0 / 0 | 0 / 0 / 0 / 0 | none |

Heaviest five at more extra slots (ghost runs, peak live / in wide view / draw list / hitboxes,
`find_free` fails / evictions):

| Level | x50 | x60 | x80 | x100 |
|---|---|---|---|---|
| W3-2 | 67 / 26 / 68 / 32, 0 / 0 | 67 / 26 / 68 / 32, 0 / 0 | 67 / 26 / 68 / 32, 0 / 0 | 67 / 26 / 68 / 32, 0 / 0 |
| W4-2 | 67 / 54 / 68 / 56, 0 / 2 | 77 / 56 / 78 / 59, 0 / 1 | 79 / 57 / 80 / 61, 0 / 0 | 79 / 57 / 80 / 61, 0 / 0 |
| W4-4 | 69 / 53 / 71 / 63, 0 / 15 | 79 / 52 / 80 / 70, 0 / 5 | 86 / 52 / 86 / 74, 0 / 0 | 86 / 52 / 86 / 74, 0 / 0 |
| W6-2 | 70 / 23 / 71 / 49, 0 / 16 | 80 / 23 / 81 / 51, 0 / 11 | 95 / 23 / 96 / 58, 0 / 0 | 95 / 23 / 96 / 58, 0 / 0 |
| W6-4 | 54 / 26 / 55 / 54, 0 / 0 | 54 / 26 / 55 / 54, 0 / 0 | 54 / 26 / 55 / 54, 0 / 0 | 54 / 26 / 55 / 54, 0 / 0 |

## What it says

- **No panic in any run** (every level, bonus and final, 4:3 / wide / ghost, extra 10–100). No
  softlock seen: the three `(no end)` wide runs are drift (W2-1's frog stops 850 px short of the exit
  when its inputs end; W4-1 and W5-1 have coins left, so the exit stays shut), the game running on.
- **Demand is not bounded by the view.** Given room, stale objects stay live: with 80 or 100 extras
  the runs are identical and the most ever live at once is 95 (W6-2, frame 8484), 86 (W4-4), 79 (W4-2).
  At most 57 are inside the wide view at once (W4-2). In 4:3 the original is saturated at 20 in 19 of
  the 24 levels and refuses spawns in 18 (W4-2: 243 refusals).
- **Refusals fall with room, but not monotonically**: each eviction changes the rest of the game
  (W4-2 refuses 6 at x20, 32 at x30, 0 at x40). At x40 W4-4 and W6-2 still refuse; at x50 and x60
  none of the heaviest five does, and at x60 no level does; from x80 nothing is evicted either.
- **The draw list overflows**: its 40 places (0xa8 bytes) hold far fewer than the peak 96 entries
  (spud + slot bobs + free bobs to erase). T03 must give it `1 + 20 + WIDE_SLOTS + free bobs + 1` places.
  Hitbox entries peak at 74 (W4-4), inside the original's 127-entry `$600` table, but a table full
  of multi-box objects could pass it: size it per slot, not from this peak.
- **Bonus and final levels** add little: WB-1 peaks at 23 live (21 in view), WW-1 at 1 (the witch).
- W6-3's ghost run lost a life at 1362 although its frog block was copied from the 4:3 run: a wide-only
  enemy hit it between copies. The game, not a fault (DESIGN §2.11).

## Recommended `WIDE_SLOTS`: 80

The measured peak is 95 live (75 extra) on W6-2, the most any route needs with no eviction at all;
80 extras gives a margin of 5 over it, and x80 and x100 played identically on the heaviest five, so
nothing more was ever wanted. The margin is small because past it the original's eviction still
applies: only objects off the wide view's despawn sides are reclaimed, and at most 57 objects were
ever inside the view: eviction starts only past 5 more live objects than W6-2's peak, and a spawn
can be refused only once the 100 slots hold no evictable object, 43 more in view than any route had. 60 also refused nothing on
these routes, but it evicts on W4-2, W4-4 and W6-2, and evictions make the wide game differ by how many
slots it has; 80 keeps every measured route eviction-free. Cost: 80 × (`$7c` + `$11c` + 4) bytes,
about 31 KB of state, and no beam time (D11).
