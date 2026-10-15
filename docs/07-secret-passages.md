# 07 — Secret passages

Superfrog hides bonus rooms behind **fake walls**: stretches of the map drawn
with the exact same terrain tiles as the surrounding solid rock, but which the
frog can walk straight through. `tools/find_secrets.py` locates them all.

## The collision / secret model

Each `BODY` cell is `tile index (10 bits) | flags (6 bits)` (see
[`04-t7mp-level-format.md`](04-t7mp-level-format.md)). The 6-bit flag field is a
per-cell collision descriptor. Reverse-engineered from the flag distribution
across all levels (and confirmed against a real in-game screenshot):

| flag (bin) | meaning |
|-----------|---------|
| `0b000000` (0) | open air / background — frog passes |
| `0b111101` (61) | solid ground — the bulk of terrain interiors |
| `0b111111` (63) | solid **surface top** (bit 1 set) — the standable outline of terrain |
| `0b1XX11X` … | **solid** in general: a cell collides iff `flag & 0b11000 == 0b11000` (bits 3 **and** 4) |
| **`0b100110` (38)** | **fake wall / secret passage** — drawn as solid terrain, but passable |

A **secret-passage cell** is simply any cell placed with flag **38**. It is drawn
as ordinary terrain — visually indistinguishable from the wall beside it — yet the
frog walks through it into a hidden room or down a hidden shaft.

> Note: do **not** additionally require the tile to be "usually solid elsewhere".
> Vertical drop-shafts are built from tiles used *only* inside passages, so those
> tiles never read as solid and such a filter wrongly hides them.

## Evidence

- **World 1, Level 1** (2 passages): a 13-wide horizontal band of ordinary dirt
  tiles (flag 38) tunnels through a hillside into a hidden cave hut; **and** a
  vertical drop-shaft at col ~94 (≈ ⅓ across) that opens from the open air and
  falls straight down through what looks like solid ground.
- **World 4, Level 1** (`d2_c22`): a whole network of thin flag-38 corridors
  links a dozen isolated rooms packed with collectibles — a secret maze.
- **World 2** (castle): every level is riddled with flag-38 passages behind the
  brickwork.

## Results

`python tools/find_secrets.py` → `build/secrets/<level>.png` (each level with its
fake-wall cells tinted magenta and each passage boxed) + `SECRETS.md` (coordinates)
+ `_montage.png`. **144 secret passages across the 29 levels.** Per level:

| World | Level (chunk) | passages | fake-wall cells |
|------:|---------------|---------:|----------------:|
| 1 | d1_c21 | 2 | 69 |
| 1 | d1_c22 | 11 | 444 |
| 1 | d1_c23 | 2 | 42 |
| 1 | d1_c24 | 7 | 83 |
| 2 | d1_c31 | 5 | 164 |
| 2 | d1_c32 | 13 | 677 |
| 2 | d1_c33 | 18 | 742 |
| 2 | d1_c34 | 20 | 321 |
| 3 | d2_c12 | 2 | 40 |
| 3 | d2_c13 | 4 | 402 |
| 3 | d2_c14 | 2 | 69 |
| 3 | d2_c15 | 2 | 34 |
| 4 | d2_c22 | 11 | 1256 |
| 4 | d2_c23 | 10 | 563 |
| 4 | d2_c24 | 5 | 303 |
| 4 | d2_c25 | 3 | 112 |
| 5 | d2_c32 | 1 | 91 |
| 5 | d2_c33 | 5 | 256 |
| 5 | d2_c34 | 1 | 76 |
| 5 | d2_c35 | 2 | 98 |
| 6 | d4_c03 / c15 | 5 | 143 |
| 6 | d4_c04 | 4 | 141 |
| 6 | d4_c05 / c17 | 2 | 22 |
| 6 | d4_c02 / c14 | 0 | 0 |

(The bonus `LB` and world-map `LW` levels have none; a couple of World 6 levels
have no fake walls at all.)

## Rooms that cannot be reached in play

**W2-1, cols 45-51, rows 14-26** (floor row 27): the room with the 20 000-point
fruit 48,22 (handler h0+$f008), the fruits 48,23 and 48,26, and a column of coins.
No way into it was found on the original (T105, T133), and the map shows why:

- Its only ways in are fake walls S1 and S2. S1 (52-79, 16-19) is a corridor whose
  floor is row 20; it ends over shaft D (cols 80-84, rows 16-35). S2 (52-59,
  22-24) joins shaft A (cols 60-63, rows 22-39, open below onto the start's
  floor). All four are closed above: the room by row 13, the corridor and D by
  row 15, A by the corridor's floor (rows 20-21). Everything has to be climbed
  into from below.
- Nothing in there lifts the frog: no pad, spring, launcher or lift in the room,
  the corridor, A or D. A jump rises about 106 px. Wings only slow a fall: fire
  takes 2 off the falling speed, at most 8 times a jump (h0+$11056). The flyers
  hover within about 21 px of where they spawn, and none spawns in A or D.
- Shaft A: the furthest up it gets is the extra life at 63,35, through the ground
  probe's lift up the pillar's column. The best jump from there peaks with the
  frog's feet at row 28, four rows below S2. Its walls (cols 64-65) cannot be
  climbed.
- Shaft D: the nearest pad (91-92,35) throws the frog about 22 rows up its own
  column (89-93). That column's left wall (85-88) is solid apart from S5 (rows
  20-23). Going left through S5 puts the frog on S5's floor (row 24) or into D,
  below the corridor's floor. From there it needs about 64 px of height; under
  S5's solid ceiling (row 19) a jump gives about 30. Searches over bounces off the
  pad found none that carry the frog through S5 high enough.

The coin trails in A and D zig-zag downwards: the area looks built to be fallen
through from the corridor, which the shipped map never lets the frog reach. A map
editor should treat the room as dead. The level's other 20 000-point fruit, 260,25
(past the exit, which lets the frog through while coins are left), is reachable;
`items-fruit-w2-1` takes it.

## Caveats

- Flag **38** is the game's dedicated fake-wall value and gives clean results.
  Wall tiles carrying flag **0** (fully carved-open) also exist — some are visible
  cave mouths / shafts rather than true fake walls — so they are *not* counted
  here to avoid false positives.
- The exact bit meanings beyond "solid" and "fake wall" (the rarer flags 6, 8, 9,
  37, 55, …, likely hazards / one-way platforms / water) are not yet fully pinned.
