# Brief for `pir plan`: real widescreen for Forest Dumb

Written 2026-11-21 for the planner, a draft. Slug: `forest-dumb-widescreen`. Read
`games/forest-dumb/CLAUDE.md`, [11-translation.md](11-translation.md) ("The procedure", step 4),
[14-translator.md](14-translator.md) (the seam, the mount and "Open after the plan"),
`plans/forest-dumb-translator/DESIGN.md` §2.9, [10-object-system.md](10-object-system.md),
[06-graphics-and-rendering.md](06-graphics-and-rendering.md), [12-level-start-draw.md](12-level-start-draw.md),
and Superfrog's `plans/real-widescreen/DESIGN.md` and `REPORT.md` first.

## The goal

The owner turns on View ▸ Widescreen and plays any of the six worlds in 16:9. The side strips are real
game, as in Superfrog: creatures, collectibles, springs, water and every other object alive there, the
character walking into a strip near a level's end, nothing black past the map. With widescreen off the
game is the original, judged against it exactly as today.

This is the first change to Forest Dumb's game logic, and the first plan that rewrites translated
routines by hand **because the owner wants to change them**, not to port them. How the plan does that
is the point as much as the feature (the procedure below).

## The procedure (owner, 2026-11-21)

The translated program stays the engine. A rule the change touches is moved to hand-written Rust, and
nothing else is:

1. **Map.** List every routine that reads or enforces what widescreen changes: the screen's horizontal
   edges and every margin measured from them, the object capacity and every loop over it, the camera's
   limits, and any object with its own edge test. Name each by address, say what it tests and how the
   rule applies to it, and list the data the routines share. **The owner reviews the map before any
   rewriting starts.** A routine the map misses keeps the 4:3 rule silently, so the map is a
   deliverable with its own check: a test or measurement that would show a missed site (for example
   tracing which routines read the camera and the slot tables across every world's cases on the
   oracle, and checking each reader is either on the map or named as not edge-dependent).
2. **Rewrite faithfully.** Each mapped routine (or group sharing state, see Open 2) is hand-written
   starting from its translated routine (`tools/reshape.py` drafts it), on typed state, and held to the
   translated routine on the same states through the seam (`fd-judge --seam`). With widescreen off it
   is the original: every world's cases stay green with it mounted.
3. **Change behind the switch.** Only then the widescreen branch is added, taken only when the mode is
   on. With it off the code computes exactly what step 2 proved.
4. **Mount.** The rewrites go in through the seam (`fd-core` `PORTED`, `fd-translated/src/seam/`), in
   every world, not W1-1 only. The translated routines stay as the yardstick and the fallback.
5. **Test the changed mode on its own terms.** A widescreen game has no original to compare with; its
   tests are ours (Superfrog's `widescreen_*.rs` are the pattern).

No hint, address special case or edit to generated Rust (translator DESIGN §2.2): a widescreen
variant of a translated routine is a hand-written routine on the seam, never a patched translation.

## Where it starts

- The app runs the translated program from power-on (`fd-app`). W1-1 runs with level 1's hand engine
  mounted as the whole `game_frame` (`--mount level-1`); worlds 2 to 6 run translated.
- The app has no Widescreen item (`games/forest-dumb/CLAUDE.md`, "the original has no wider view").
  The toolkit's settings already hold a `widescreen` field, which `fd-app` reads and writes back
  unchanged; the view menu and the window's 16:9 size are the toolkit's (`amiga_app::view_menu`),
  Superfrog's `sf-app/src/window.rs` `extra_columns` the rule: at 320 x 256 it gives 67 columns a side.
- **Creatures spawn from the edge redraws, not a scan.** `h0_681c`/`h0_6852` fill one of **five**
  `creature_slots` when an edge redraw (`redraw_edge_a..d`, from `main_loop`) brings a creature's cell
  into view, and `h0_2dd6`/`h0_2e0e` free the slot and write the creature back into the map when it
  leaves the view (docs/10). Superfrog spawned from a cell scan the plan could widen; here widening
  where creatures wake means widening where the edge redraws run, or adding a wake-up for the strips.
- **The play screen is picture state** (docs/06): the display buffers are what the original's blits
  leave there, `collision_map` sits in their slack, and a blit word outside them is dropped. The
  centre must stay exactly what the buffers hold, so the strips are composed outside them, as
  Superfrog's were (its DESIGN §2.9). Creatures are cut onto the screen by `h0_5820` and restored by
  `h0_5e44`; contact is CLXDAT, a hardware collision of bitplanes against sprites, which sees only what
  is in the buffers.
- The mount spends the original's time by walking each routine's path (`seam::walk`, `seam/rules.rs`,
  `fd-conform/tests/mount_timing.rs`). Every routine this plan mounts needs a timing rule held to its
  translated routine, with widescreen off.
- Level 1's hand engine (`fd-core`) already ports the creature routines, the camera and the character
  for W1-1. What it covers that worlds 2 to 6 also run is the plan's head start; what other worlds add
  (the flame `$34` in worlds 3 to 6, docs/10) is not ported.
- The route planner (`fd-plan`) and the map page run on `fd-core`'s hand engine and cover W1-1 only.

## What it delivers

| Part | Delivers | Proven by |
|---|---|---|
| The map | Every edge-, capacity- and camera-dependent routine in all six worlds, by address, with its rule and shared data; reviewed by the owner | The completeness check of step 1; owner's yes recorded in the plan |
| Faithful rewrites | Each mapped routine hand-written and mounted in every world | `fd-judge --seam` per routine; every world's cases green mounted; `mount_timing.rs` for each new rule |
| The mode | One switch in the core; the geometry rule (Superfrog's §2.2: every margin keeps its distance from the moved edge); extra capacity; the camera's limits | Unit tests per site against the rule; widescreen off unchanged on every case |
| The picture | Strips composed from the map and every live object, pixel-identical to the same thing in the centre, no seam where it crosses | A test comparing an object drawn in a strip with the same object in the centre |
| App and recordings | View ▸ Widescreen, switching mid-level, bug recordings carrying the mode (Superfrog's format rule) | `fd-e2e` tests; a widescreen recording replays to the same end |
| Every world | Each of W1-1 to W6-1 finished in widescreen | A route a world kept as a test (Open 5) |

## Settled

- Real widescreen, the strips live, as Superfrog's (owner, 2026-11-21). A picture-only widescreen is
  not this plan.
- Widescreen off is the original, judged by the existing cases with every rewrite mounted.
- The procedure above (owner, 2026-11-21): map, rewrite faithfully, change behind the switch, mount,
  test the mode on its own. The translated program stays the engine for everything not mapped.
- Superfrog's rules are reused where Forest Dumb's original allows (the geometry rule, extras costing
  no time, recordings carrying the mode), each checked against this game's code, never copied (root
  CLAUDE.md, "Do not carry over a game's details").

## Open, for the planner to settle with the owner

Proposals only.

1. **How creatures wake in the strips.** The original wakes them from the edge redraws. Options: run
   the redraws (or their spawn part) at the wide edge in widescreen; or keep the redraws and add a
   widescreen-only wake-up over the strips' cells. Proposed: whichever the map shows touches less, the
   planner measuring both on paper first. Ask the owner if a creature would visibly appear or vanish
   differently between the two.
2. **Rewrite per routine or per frame.** Level 1 mounted the whole `game_frame` because its routines
   share state across a pass. Per routine keeps the rewrite small; per frame reuses level 1's mount
   and its timing walk but means `game_frame`'s whole tree in every world. Proposed: per routine where
   the map's routines share little, groups where they share slot or camera state; the map decides.
3. **Capacity.** Five creature slots will not cover a view about 40% wider. Proposed: extra slots in
   widescreen at addresses the original never maps, sized from a measured peak, as Superfrog's §2.4;
   their work costs no beam time (Superfrog's D11). Collectibles and other object groups the map finds
   with fixed tables get the same treatment.
4. **Contact in the strips.** Contact is a hardware collision, which only sees the display buffers.
   A creature in a strip is drawn outside them. Proposed: the character cannot reach a strip's creature
   without the camera bringing it into the centre first, so strip-only contact is never needed; the
   planner checks that on the code, and asks the owner if it does not hold.
5. **Proving every world finishes.** The planner (`fd-plan`) runs W1-1 only, on the hand engine.
   Options: extend it to every world on the translated program with the mounts (large); routes played
   by the owner for worlds 2 to 6 (Superfrog's W6-4); or a fixed widescreen stretch per world instead
   of a whole route. Proposed: ask the owner; the level plans may have extended the planner by then.
6. **Order with the level plans.** Worlds 2 to 6 are not yet checked end to end (`level-2-brief.md`).
   A widescreen rewrite on a world whose cases do not exist has nothing to keep green. Proposed: this
   plan covers the worlds whose cases exist when it starts, and each later level plan adds its world's
   widescreen check; or this plan waits for the level plans. Ask the owner.

## Out of scope

Picture-only widescreen, a widescreen title or menus (they keep 4:3, centred), retuning any rule
beyond moving an edge, porting routines the map does not name.
