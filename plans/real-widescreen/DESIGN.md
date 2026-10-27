---
setup:
  - python3 -m venv venv && venv/bin/pip install -q --disable-pip-version-check -r requirements.txt
  - [ ! -x tools/setup/oracle.sh ] || tools/setup/oracle.sh
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
  - cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-core -p sf-disk -p sf-app
  - cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-conform --lib --test purity --test script --test 'recorder_*'
  - [ -z "$(ls engine/crates/sf-conform/tests/widescreen_*.rs 2>/dev/null)" ] || (cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-conform --test 'widescreen_*')
  - cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-e2e --test window --test menus --test view --test 'recorder_*'
  - [ -z "$(ls engine/crates/sf-e2e/tests/widescreen_*.rs 2>/dev/null)" ] || (cd engine && CARGO_TERM_COLOR=never cargo test -q -p sf-e2e --test 'widescreen_*')
---

# Real widescreen — Design

This plan builds on the finished `rust-engine` plan and on `bug-recorder`. `plans/rust-engine/DESIGN.md`
is sealed and still binds everything here that it governs (cited `RE §n`): faithful (RE §2.1), the
copyright boundary (RE §2.3), the menu bar (RE §2.7), the purity boundary (RE §3.1), the seatbelts
(RE §5.2). This plan overturns one rule of it, RE §2.7's "widescreen is visual only", for widescreen
mode alone (owner, 2026-10-22, D1). Bug-recorder's design is cited `BR §n`.

## 1. Purpose

Widescreen today widens the window to 464×261 and paints 65 extra columns each side from the level
map, so the side strips show terrain and nothing that moves: no enemies, no coins, no platforms, no
frog near a level's end, and black past the map at a level's left and right ends. The owner wants
the strips to be real game. That needs two things: the game itself must keep alive what is in the
strips (the original only wakes things within about a tile of its 334-pixel window), and the picture
must draw what is alive there. With widescreen off the engine stays the original, judged against it
as before; with it on, the engine plays a deliberately different game, one whose screen edges are
65 pixels further out.

### Success criteria

- Widescreen off: every existing conformance case and test passes unchanged; no 4:3 state, picture
  or sound differs from today's engine.
- Widescreen on: every live object, bob and the frog shows in the side strips, pixel-identical to the
  same thing drawn in the centre, with no seam where it crosses; nothing in any level shows black
  past the map.
- Objects wake as their cell reaches the wide view's edge and are freed after leaving it, by the
  original's own margins measured from the wide edge (§2.2).
- Switching mid-level, recordings and replays are deterministic: same start, inputs and switches give
  the same game.
- Every level, the bonus level and the final level are finished in widescreen by the smart planner,
  each kept as a test (T12, after smart-planner is merged).

### Stance

The 4:3 game is the original and is never touched by widescreen work: every widescreen change is a
branch taken only when the mode is on, and with it off the code computes exactly what it computes
today. Widescreen is the original's rules with the screen edges moved, not a redesign: no margin,
speed or rule is retuned beyond moving an edge.

---

## 2. Behaviour specification

### 2.1 Two modes

The core has one switch, `widescreen: bool`, and a constant `WIDE_EXTRA = 65`: the extra lores
columns each side, the app's `window::extra_columns(261)`. The app's window is always 464×261 in
widescreen, at every scale and fullscreen, so one number serves; a mode carrying a column count
would let a recording or a test ask for a geometry the app never shows. With the switch off, every
rule below reduces to the original's constants. A test pins that `WIDE_EXTRA` equals
`sf-app`'s `window::extra_columns(GAME.1)`, so the picture and the game cannot drift apart.

### 2.2 The geometry rule

The original's horizontal screen edges are 0 and 334 window pixels from `camera_x` (window pixel 0
shows level x `camera_x`, docs/12). In widescreen they are −65 and 334+65. Every horizontal margin
the original measures from an edge keeps its distance from that edge: a test against `cx + $170`
(the right despawn, 36 past the right edge) becomes `cx + $170 + 65`; one against `cx − $30 − width`
becomes `cx − $30 − 65 − width`. Spawn scans are cell-based, so a scan column at `n` cells from the
origin moves out by `ceil(65/16) = 5` cells, and the origin moves left by 5 cells (clamped at 0 as
now); a scan row widens by 10 cells. Moving the column a whole cell further than 65 pixels is
deliberate: an object must be live before any of it can show, as in the original, where the column
sits past the edge. Vertical margins are unchanged, because the picture's height is.

Rationale: one rule for every site means a reviewer checks each site against the rule, not against a
taste, and nothing is retuned (Stance).

### 2.3 Spawn and despawn

Sites (survey, 2026-10-22): `scan_origin`, `scan_edges` (column and row), `scan_screen`,
`scan_right_column` (bonus), `obj_despawn_if_offscreen` (about 60 call sites, one function),
`obj_free_if_left_behind` (bonus, 7 sites), `find_free`'s off-screen eviction and the coin's
force-out of a visible slot (`level/objects.rs` 63–81, 197–224). Each takes the mode and applies
§2.2. The spawn column and the despawn threshold must move together: a column at cx+401 under a
right threshold still at $170 would free each object the pass it spawned.

### 2.4 Capacity

The original has 20 object slots and 21 bob records, at fixed addresses with code on both sides, and
objects refer to each other by absolute address (bob pointer at slot+$50, free-list entry at +$4c,
hitbox +8, chain links +$64, the bonus level's `SHOTS`). Widescreen keeps up to roughly twice as much
alive. In widescreen the engine has `WIDE_SLOTS` more slots, bob records, free-list entries, draw-list
room and hitbox entries, in a new LAYOUT range at an address the original never maps, so every
pointer stays a plain address and the original's code paths work on them unchanged. Every loop over
the slots runs over the original's 20 first, then the extras while the mode is on or any extra slot is
still live (after a switch off, §2.7). A game that never had widescreen on never touches them and
their bytes stay 0; spawning into an extra slot happens only with the mode on. `WIDE_SLOTS` is set from T00's measured peak with margin. The
extra slots are `presentation`-free game state: saved in save states, compared nowhere (there is no
original to compare with).

Rationale: without more room the wider view would evict or refuse objects, and the original's
eviction can take a visible object in the centre, which would make widescreen worse than 4:3 where it
matters most.

Widescreen's extra work costs no beam time (owner, plan review 2026-10-22, D11): every `spend` for an
extra slot (its update, hitbox, bob draw) and for a scan cell widescreen adds beyond the original's
counts is skipped, so the extras never push a pass past its frame. The original 20 slots and the
original's scan cells cost what they cost; they may hold different objects in widescreen, so a busy
scene can still lag differently from 4:3, but not because of the extras. Rationale: the owner wants
widescreen no slower than 4:3, over keeping the original's cost rules.

### 2.5 Camera and the level's ends

In widescreen the camera is clamped so the wide view shows no more of the map than the original's
view can at its limits: the original's camera stops at `cx ≥ $10` (view from level x 16) and
`cx ≤ level_width − $168` (view to level x `level_width − 26`), so widescreen's stops at
`cx ≥ $10 + 65` and `cx ≤ level_width − $168 − 65`. The two copies of the clamp (`keep_inside`,
`start_point`) change together. Near an end the camera stops and the frog walks off-centre into the
strip. Keeping the original's outer limits, not the map's edge, is deliberate: the original never
shows level x 0–15 or the last 26 pixels, and what those cells hold has never been seen.

The frog's sprites are hidden past screen x $150 (or left of −$20) in the original, by parking them
at x $150 (`frog/anim.rs` `OFF_SCREEN`); in widescreen the bounds and the parking spot move by 65
(§2.2), since a sprite parked at $150 would show in the right strip. In the bonus level the frog's clamp (`cx − 8 .. cx + $12a`) widens by 65
each side, so the frog flies into the strips. The final level's camera does not follow (it stays at
its start point); in widescreen it is only clamped as above. If that moves it off the original's start
point, the witch's path (relative to the camera) moves with it, which is the §2.2 rule applied.

### 2.6 Objects with their own edge tests

From the survey: Spud's parking (`spud_update`, x outside `[cx, cx+$150)`), the bonus shots
(`spawn_shots` at cx+$160, re-fire after cx+$150, park at cx+$200), the watchers' direction clamp
(`common::direction`, screen x 0..$154), the witch's loop (path from cx), the bonus chain's path
(offset from its spawn x, which comes from the right column), and `set_cell_tile`'s redraw window.
Each follows §2.2. Consequences the owner was told (2026-10-22): a thrown Spud and the bonus shots
fly on to the wide edge before they are free again, and the bonus frog may enter the strips. The
bonus level's map markers 11 (the chains fly off) and 55 (the level is over) act when the right-hand
scan column reaches them, which in widescreen is 5 cells further on: the bonus level ends about 3 s
sooner (the camera creeps 25 px/s) and the chains fly off sooner. Accepted as §2.2 applied (owner,
plan review 2026-10-22, D12).
`set_cell_tile`'s window only decides which cells are redrawn into the display buffers; widescreen's
strips draw from the map, so it may stay as is, and its task says which.

### 2.7 Switching mid-level

`Game::set_widescreen(on)` takes effect at the start of the next step. Switching on in a level runs a
spawn scan of the whole wide view (the `scan_screen` scan, widened) in that step, so whatever is in
the strips is live at once (owner, 2026-10-22). Switching off does nothing more: the extras are freed
by the 4:3 despawn rule as the camera moves off them, and an object in an extra slot keeps running
until then. The camera is clamped by the new rule the next time the clamp runs, which may move it by
up to 65 pixels in one step at a level's end; that jump is accepted. Outside a level the switch only
changes the picture, as today. Once a level has been played with widescreen on, the game is no
longer the original's, even if switched off: the extra slots stay live until freed, so a 4:3
comparison from that point is meaningless and the recording says so (§2.8).

### 2.8 Recordings and bug fixing

A recording (BR §2.6, `sf-core/src/recording.rs`) carries the mode: format version 2 adds a header
field `widescreen on|off` (the mode at the level's start) and an input token `W` that toggles the mode
once, at the first frame of the line carrying it; unlike the held tokens it is not repeated while the
line holds, and the writer starts a new line at every toggle frame. Version 1 files still parse, as
widescreen off. The app records each switch made
while recording; today a switch leaves the recording running and saves the 4:3 picture
(`recorder_app.rs` `a_recording_in_widescreen_saves…`), which this replaces: the replay plays the
mode as recorded and draws for it: `sf-conform bug replay` renders the wide picture while the mode is
on, and the app's `--replay` sets the game's mode and the view from the recording at its start and at
each toggle, leaving the saved setting alone. `sf-conform bug compare` and `bug case` on a recording
with any widescreen in it exit with a one-line message saying a widescreen game has no original to
compare with (a case is an oracle comparison), and
`docs/BUG-FIX.md` says: a bug that only happens in widescreen is ours to fix, with a widescreen test
in place of a conformance case.

### 2.9 The picture in the strips

The strips are composed from the map (today's `render/playfield.rs` path), then every bob that is
live in the shown buffer's pass, then the hardware sprites. The centre stays what the display buffers
hold, untouched. Bobs cannot be drawn into the display buffers past their 384-pixel rows, so at
`bob_draw` time (25 Hz) the engine records, per display buffer, a draw list of every live bob in
draw order: its level position as drawn (the latched camera's), its frame record, and its
background/priority flags. The renderer draws the strips from the list of the buffer being shown, so
a bob crossing the centre's edge is drawn from the same pass, position and frame on both sides of the
seam. Bob pixels are cookie-cut with the sheet's mask exactly as `draw_bob` does, so a strip bob is
pixel-identical to the same bob in the centre. The list is presentation state: kept in save states so
a loaded state draws at once, never compared.

The frog is two attached hardware sprite pairs placed by `place_sprites`; the renderer already places
sprites from their control words. If a frog position in the right strip overflows the hardware's
9-bit HSTART, the renderer places the frog from the engine's wide screen x instead (T09 measures it).

Non-level screens (title, map, slot machine, tally, story) keep colour 0 at the sides, and the top
panel's lines keep today's sides (owner, 2026-10-18, rust-engine FINDINGS).

### 2.10 Where it applies

Every scene that runs `level_frame_loop`: W1–W6, the bonus level (shooter mode), and the final level.
Nothing outside a level changes.

### 2.11 The unhappy paths

- An enemy woken earlier than the original ever woke it may act on a position or timer the original
  never reached (falling before it is seen, a partner search finding a partner that 4:3 never had).
  That is the widescreen game, not a bug, unless it crashes, softlocks or breaks a level's finish;
  T11's drill and T12's runs find those, and each one found becomes a test.
- The extra slots fill up: `find_free` then evicts by the original's rule over all slots. T03 asserts,
  in tests, that T00's heaviest scenes stay below `WIDE_SLOTS`.
- A save state from widescreen loaded with widescreen off, or the reverse, loads; the current setting
  applies from the next step (§2.7 switch rules), as the owner decided for save states (2026-10-22).
- A recording made before this plan (version 1) replays as widescreen off.

---

## 3. Architecture

### 3.1 The boundary

RE §3.1 holds unchanged: `sf-core` is pure (no clock, no I/O), enforced by `sf-conform/tests/purity.rs`;
if it fails the fix is to move the code, never to relax the test. The widescreen mode reaches the core
as a field of `Game` copied into `LevelCtx` each step, never from a global or the app's settings, so
every widescreen rule is testable headless in milliseconds.

### 3.2 Modules

- `sf-core/src/level/screen.rs` (new): `Screen { wide: bool }` and every horizontal edge of §2.2 as a
  method (`right_despawn()`, `left_margin()`, `scan_column_cells()`, …). Each returns the original's
  constant when `wide` is false. Every site of §2.3–§2.6 calls it instead of a literal, so a reviewer
  greps for the literals and finds none left in a horizontal test.
- `level/spawn.rs`, `level/objects.rs`, `level/camera.rs`, `level/load.rs`, `items/powerups.rs`,
  `bonus/{shots,enemies}.rs`, `enemies/common.rs`, `bosses/witch.rs`, `frog/anim.rs`: the sites.
- `layout.rs`: the extra-slot range (§2.4) and the per-buffer wide draw lists (§2.9).
- `render/{mod,objects,playfield}.rs`: the strips drawn from the map, the draw list and the sprites.
- `game.rs`: `set_widescreen`, `widescreen()`; `set_view` stays the picture's.
- `recording.rs`: format v2 (§2.8).
- `sf-app`: the menu toggle sets both the game mode and the view; the recorder records switches.
- `sf-conform`: `bug replay` applies the mode; `bug compare` and `bug case` refuse widescreen.

### 3.3 Data flow

App setting → `Game::set_widescreen` → `Game.widescreen` → `LevelCtx.screen` each step → the edge
sites. `bob_draw` → wide draw list in `State` → renderer. Recording header/tokens → replay →
`set_widescreen` at the recorded frames.

### 3.4 Storage

The mode is not in `State`: like the view, the app sets it from its setting, so a loaded state plays in
the current setting (§2.11). The extra slots and the draw lists are in `State`, so save states carry
them; the state's size grows and a state from an older build is refused as today (RE §2.8).

---

## 4. Testing

- 4:3 untouched: the existing suites. The block runs the scoped lines; every task that changes `sf-core`
  level code also runs `cd engine && cargo test -q --workspace` once and records the result, because
  the conformance cases that pin the 4:3 game are outside the block (bug-recorder's measurement: about
  50 minutes; `whole_game.rs`' wall-clock test fails on a loaded machine and is noted, not fixed).
- Widescreen rules: unit tests per site in `sf-core` (an object at the old and new edge, both modes).
- Widescreen picture: `sf-core` render tests (a strip bob equals the centre bob; seam) and
  `sf-e2e/tests/widescreen_*.rs` driving the real app.
- Widescreen games: `sf-conform/tests/widescreen_*.rs`, engine-only (no oracle): determinism, switch,
  replay, and T12's finished levels.
- None of these can say the widescreen game is "right" in feel; T11's drill judges it, worker-driven.

---

## 5. Environment — read this before running anything

RE §5 holds: macOS 26.5.1 arm64, rustc/cargo 1.99.0, Python venv from `requirements.txt`, vAmiga
pinned in `~/.cache/superfrog-re/vamiga/`, Kickstart in `~/amiga-roms/`, disks in the main checkout
root (sf-conform and sf-e2e find them there from a worktree).

**The test command.** The block above. Measured 2026-10-22 in a fresh worktree from main: setup 8 s and
clean; Python 37 s; `sf-core sf-disk sf-app` 148 s; the sf-conform line 226 s; the sf-e2e line 714 s;
all green. Re-measured at plan review (2026-10-22, fresh worktree, machine load 13–18): all green;
`recorder_app`'s `every_ending_action_discarded…` once hit its 150 s `--timeout` under that load and
passed alone (85 s) and on a re-run of the line. New test files are named `widescreen_*.rs` so the guarded glob lines pick them up; the
guards skip a line until such a file exists, because cargo fails on a `--test` pattern matching
nothing. `.pir/settings.json` carries only the Python line. Quiet: `cargo test -q` prints dots and one
result line per binary; for detail add `-- --nocapture`. Colour is off inside the commands
(`NO_COLOR=1`, `CARGO_TERM_COLOR=never`); the session sets only `COLORTERM=truecolor`, which forces
nothing.

**Dependencies.** RE's policy: anything allowed, named with a line of why in the task that adds it. None
expected.

**End to end.** `sf-e2e` exists and carries this (no rig task): it launches the real app with the asset
cache in a temp home and drives it by script events (`menu widescreen` exists), captures every frame and
`Game::probe`. Sizes: 2× widescreen (464×261 × 2) and fullscreen widescreen, as RE §5. The free backend is
the asset cache; nothing here reaches a network.

**The smart planner (T12 only).** `sf-plan` (crate `engine/crates/sf-plan`) lands with the
`smart-planner` plan, on branch `pir/smart-planner`, not yet merged on 2026-10-22. pir merges the base
branch into a build's feature branch only after every task is done, so T12's branch never receives
`sf-plan` by itself. Once smart-planner is merged into main, the owner stops this run, a session the
owner asks merges main into `pir/real-widescreen` and brings the live task branches up to it, and the
owner restarts the run (D5). T12's worker starts only when `engine/crates/sf-plan` exists on its own
branch; otherwise it asks the owner and waits.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| Whether the widescreen game is fun to play | A real person's reaction; not a gate (the owner judges by playing when they choose) |

### 5.2 Seatbelts

RE §5.2 holds: every app run in tests is bounded by frames (`--frames`, script length) in a temp home.
T12's planner runs carry `sf-plan`'s own `--budget-min` and `--max-mem-gb`.

Nothing in this plan touches anything outside this Mac and repo, so there is no §5.3 (owner confirmed
at plan review, 2026-10-22). The setup's `tools/setup/oracle.sh` fetch is RE §5.3's `worker` row,
unchanged. The hand-merge of main before T12 (D5) is the owner's, not a live action.

---

## 6. Recovery

Widescreen is a menu setting; turning it off returns the 4:3 original from the next level start (the
extra slots drain as §2.7 says). A broken `settings.toml` falls back to defaults (RE §2.9).

---

## 7. Decisions and rationale

- **D1 Widescreen changes the game** (owner, 2026-10-22). Options were: draw only what the original
  keeps alive (pop-in about a tile inside the right strip); add frozen stand-ins; or wake things at the
  wide edge. The owner chose the third, overturning RE §2.7's "visual only" for widescreen alone.
  4:3 stays the original and is still judged against it.
- **D2 Switching takes effect at once** (owner, 2026-10-22), over "from the next level start" and
  "locked during a level". §2.7.
- **D3 Camera stops earlier at a level's ends** (owner, 2026-10-22), over repeating the edge column
  outward, which invents scenery. §2.5. Keeping the original's outer limits (x 16, width − 26) rather
  than the map's edge is the planner's: those cells were never shown.
- **D4 More than 20 live objects in widescreen** (planner, owner told at the requirements checkpoint).
  §2.4. The extras live in a new LAYOUT range rather than a non-memory field, because the code refers to
  slots and bobs by absolute address everywhere; a separate table keeps every pointer a pointer.
- **D5 Every level finished in widescreen is in this plan, after smart-planner merges** (owner,
  2026-10-22): T12 waits for `sf-plan`. At plan review (2026-10-22) the owner kept T12 here over moving
  it to a follow-up, knowing the run stays open until smart-planner merges and main is then merged into
  the feature branch by hand (§5).
- **D6 Strip bobs are drawn by the renderer from a per-buffer draw list** (planner). Alternatives: wider
  display buffers (every blit, scroll and erase routine re-strided, risking the 4:3 game), or drawing from
  each slot's current position (out of step with the centre, which shows a pass-old buffer). The list
  records exactly what the centre's pass drew.
- **D7 One `Screen` type for every edge** (planner). §3.2: a literal left in a horizontal test is a
  missed site, findable by grep.
- **D8 Mode is a bool with a fixed 65** (planner). §2.1.
- **D9 A widescreen-only bug is ours** (owner, 2026-10-22). §2.8.
- **D10 Edge-tied rules all use the wide screen, including Spud's and the bonus shots' flight and the
  bonus frog's reach** (owner's requirement 2, consequences told at the task checkpoint, 2026-10-22).
- **D11 Widescreen's extra work costs no beam time** (owner, plan review 2026-10-22), over charging it
  as the original's rules would (more lag in busy widescreen scenes). §2.4.
- **D12 The bonus level ends about 3 s sooner in widescreen** (owner, plan review 2026-10-22), over
  keeping markers 11 and 55 at the original's distance as an exception to §2.2. §2.6.

---

## 8. Explicitly out of scope

- Any change to the 4:3 game: it is the original (D1).
- Vertical changes: the picture's height is unchanged, so no vertical margin moves.
- Retuning anything in widescreen beyond moving edges (Stance): no speed, damage or timing changes to
  compensate for more enemies.
- Widescreen on non-level screens and the top panel: they keep today's sides (owner, 2026-10-18).
- Comparing widescreen games with the original: there is none (D1, D9).
- Other aspect ratios or a user-chosen strip width (D8).
