# 17 — Secrets: what the levels hide

What the six levels hide, how the map page lists it, and the evidence (plan `forest-dumb-map-page`,
T07, 2026-11-20). The rule is `fd_assets::secrets` (`engine/crates/fd-assets/src/secrets.rs`); the
oracle claims and the counts are `engine/crates/fd-conform/tests/secret_claims.rs`.

## What decides solid and passable

Read off the listing (`tools/re_listing.py --xrefs`) and measured in earlier plans:

- **Collision is `collision_map` alone**, a bit a pixel (docs/09). Its writers are `view_shapes`
  (level start), the edge redraws' shape copies (`h0_1390` and its twins) and `h0_62b6`, which writes
  it solid and clear under a vanishing block (`$30`-`$33`, docs/10). Its readers are `char_probe` and
  the creature probes `h0_3530` to `h0_35dc`. So what is solid is the cell's shape from
  `_GenN-1.Mapa`, except under a vanishing block.
- **The tile picture says nothing to the code** about solidity: tiles go to the play screen only
  (`view_tile`, the redraws' tile blits). The one way a picture acts is contact: a pixel of colour 29
  under the character's sprites hurts (CLXDAT, docs/10). That is a hazard drawn as such, not hidden.
- So a level hides a place exactly where its picture and its shape disagree: a wall picture over an
  empty shape (a passage) or an open picture over a solid shape (an invisible floor or wall).

## Hidden passages: the rule

A tile is a **wall picture** in a level when it is fully solid (shape all set bits) in at least two
of its cells and in at least a tenth of them. A **passage cell** is a cell of a wall picture whose
shape is fully empty and which holds no vanishing block. Passage cells are grouped 4-connected, and a
group is listed only when at least one of its cells has wall pictures on all four sides (the map's edge
counts as wall).

Why each part, from the counts and the renders (`build/renders/`, collision laid over the picture):

- **Not the majority.** The first count (44 of W1-1's 328 tiles with more than one shape) suggested
  "solid in most of its cells". That misses W1-1's tunnel: its stone tiles 301 to 310 are empty in 136
  cells and solid in 60, because the tunnel is most of where they are used; in the raw picture the
  tunnel is the hill's stone, nothing marks it. W3-1 builds its hidden rooms the same way. A tenth and
  two cells keep a background tile solid once by accident (a stray cell under a ceiling) out.
- **Slopes and ledges fall out**: a partial shape is neither fully solid nor fully empty.
- **Trim falls out** by the inside test: door frames, alcove edges and the exit's frame are wall
  pictures with empty shapes one cell thick beside open background, in every world (the exit's frame
  around the `$ff` block in all six). W1-1's red pillars at columns 27, 45, 71 and 180 (tile 30, solid
  in 57 of 77 cells) stand in an open corridor and are not listed either. A tunnel's mouth is listed
  with its tunnel, as one group.

## Counts (2026-11-20)

| Level | Cells | Groups |
|---|---|---|
| W1-1 | 133 | the tunnel under the hill `117,91` to `159,93` (121); the stone tower `121,79` to `124,82` (12) |
| W2-1 | 0 | only rims: the exit's frame, alcove and opening edges |
| W3-1 | 292 | seven: the stone room `67,25` to `77,28` with its shaft down `67,29` to `69,32`; `32,70` to `39,73`; `35,78` to `41,82`; `79,89` to `90,92`; `100,33` to `116,35`; `117,10` to `128,13`; `164,53` to `168,68` |
| W4-1 | 63 | `159,89` to `167,93`; `199,90` to `210,93` |
| W5-1 | 0 | only the exit's frame |
| W6-1 | 0 | only rims: the exit's frame and a row of three at `292,64` |

One listed cell holds an object: an orange (`$40`) at `W3-1:41,78`. A group's cells are the wall
pictures only: a cell inside a passage drawn as dark background is not listed.

## Oracle evidence (2026-11-20)

Each claim reaches the world's play on the original, sets `cheat_flag` (no life lost), carries the
character to a spot beside a passage three pixels a frame by poking `char_x`, `char_y` (the camera
follows and the edge redraws build `collision_map` as in play), lets it land and holds a direction for
200 frames. The original's own movement does the rest:

- **W1-1**, from `115,93` holding right: through every column of the tunnel, `117,93` to `159,93`, and
  out at `160,92`.
- **W3-1**, from the lava room at `80,28` holding left: through the room `77,28` to `67,28`, down its
  hidden shaft `67,29` to `67,34` and on down the slope beyond to `33,58`.
- **W4-1**, from `197,93` holding right: through `199,92` to `209,93` and out at `210,92`.
- **Control**, W1-1: the same walk with the tunnel's first three columns set to shape 0 in the game's
  loaded collision map (`h0+$28f18`) before they come into view stops at `116,93`.

The rule's counts above are held by `the_rule_lists_the_counts_docs_17_gives` (smoke); the four
claims are full.

## Looked for and not a kind

- **Invisible floors** (drawn open, solid): no group found. Solid cells whose tile is open background
  (empty in at least nine tenths of five or more cells) are three, all W3-1: `28,86` and `29,86` hang
  under a ceiling, `79,23` is a nub in a lava room's corner; none is a floor to stand on and none was
  confirmed on the original, so `SecretKind` has no such kind.
- **Warps and teleports**: none. `char_x` and `char_y` are written only by the movement routines
  (`h0_3ea8`, `h0_410c`..`h0_4b0e`), each world's start in `load_list_world_N`, and `h0+$2a98`, a
  leftover editor (joystick moves the character 16 pixels, fire writes a typed value into the object
  map) that nothing calls (no reference in the listing).
- **The camera** is written only by `camera_update`'s scrolls and clamps and reset by `session_loop`
  and `next_world`: it never jumps mid-level.
- **`world_index`** changes mid-play only through `next_world` (the exit, or HELP with the cheat). The
  title's codes set the start world and `cheat_flag` (`title_code_world`, `h0+$777e`): a front-end
  secret, not on any map.
- **Object values no picture shows**: every value in the six maps has a group (T01,
  `object_tables.rs`); none is hidden by the rule's means.
