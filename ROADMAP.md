# Roadmap

Ideas settled with the owner on 2026-10-15, kept for after the current build. None of this is a
plan yet: each item becomes one through `pir plan` (or `@repo/single` when small), where its open
questions are answered. The owner decides what and in which order; the order below is the
proposal.

## Where things stand

Milestone 1 is the `rust-engine` plan: Superfrog reimplemented in Rust, running natively on macOS,
verified headless against the original running on vAmiga. It is being built on `pir/rust-engine`
and lands on `main` when the run finishes. On 2026-10-15 its bar changed from frame-exact to
**faithful** (that plan's `DESIGN.md` §2.1): game state equal pass by pass, picture and sound within
a tolerance no player perceives, and the engine run alone meeting the original's milestones (coins,
deaths, kills, exits, screens). Everything is proven by agents; the owner is never asked to play.

The items below assume that build exists. Paths such as `sf-core` and `sf-conform` refer to its
crates under `engine/`.

## Milestone 2: report a bug, have it reproduced, fixed and proven

**Goal.** The owner plays, sees something wrong, presses one key, and hands the result to an agent.
The agent reproduces it headlessly, decides whether it is a real bug, fixes it, and leaves a
permanent test. The owner does not verify the fix.

**Why it works.** The engine is deterministic: the same start, RNG seed and per-frame inputs give
the same game. A recording of the seed and the inputs is a few KB a minute and replays exactly.

**What is missing today.** Normal play seeds the RNG from the host clock and does not record it.
`superfrog --capture` writes a PNG per frame (built for the e2e tests; unusable for a real session).
There is no replay mode for the app.

**Proposed shape.**

1. A flight recorder in the app: the seed, every frame's input and each level start kept in memory
   while playing; nothing written until asked.
2. A report key (for example ⌘B): writes a bug folder with the recording from the current level's
   start, a screenshot, the location as `W1-1:94,51` (the project's tile addressing) and a
   one-line note typed by the owner.
3. `superfrog --replay <folder>`: plays a recording back in the window, for anyone who wants to see
   it.
4. The fix flow, run as `@repo/single fix the bug in <folder>`:
   - replay the recording headlessly and confirm the same frame shows the problem;
   - play the same inputs on the original in the oracle from that level's start (the original lags
     where the engine does not, so long stretches drift; a level start keeps the comparison short);
   - the original does the same: a quirk of the real game, kept (the engine keeps original bugs),
     reported back;
   - the original differs: `sf-conform --strict` names the first differing frame and variable; fix;
   - the recording becomes a conformance case that fails before the fix and passes after;
   - fresh review; the commit carries an `Escape:` trailer.

**Open questions for planning.** Which key; how far back the recorder reaches (the level start, or a
fixed time with periodic snapshots); what goes in the folder; whether the recorder is always on;
whether a detected anomaly may export automatically.

**Reference.** `DKC1Recomp` (next to this repo) ships this already: about a minute of input plus
periodic snapshots in memory, exported on F9 into a bundle that a tool replays headlessly, and an
optional automatic export when a culling glitch is detected.

## Candidates after milestone 2

### An explorer for routes and for finding bugs

Go-Explore (Ecoffet et al., 2019; "First return, then explore", 2021): an archive of reached
"cells" (a coarse summary of the state), each with its snapshot and the shortest inputs reaching it.
The loop picks a rarely visited or new cell, restores its snapshot, explores with short input
bursts, and records new cells or shorter routes. The engine is deterministic and cheap to clone, so
returning costs nothing. For Superfrog a cell is the frog's tile area, coins taken (in bands),
switches pressed and items held; that granularity is the main thing to tune.

- **Routing.** The completion cases' scripts come from `examples/plan_exit`: a beam search steered by
  a distance field, planned on the engine, confirmed on the original chunk by chunk. It sticks where
  the way on first leads away from the goal (switch detours, pockets; W5-4 in that plan's
  `FINDINGS.md`), and an agent adds waypoints by hand. An explorer has no sense of direction and
  finds those detours itself. Routes found on the engine are still confirmed on the original.
- **Testing.** Run without a goal, maximising cells covered, and compare every new cell with the
  original (the faithful checks, the engine run alone): an automated player that pokes at things a
  planned route never touches. Each difference it finds becomes a recorded case, using milestone 2.

A trained game-playing model is not proposed: with exact state, instant save states and a
deterministic engine, search does better and costs far less.

### Simplify the timing simulation

The frame-exact bar left code that models where in a frame things happen: the beam cost model
(`level/beam.rs`), the intro's cycle-by-cycle bus and blitter simulation (`front/story/sim.rs`,
`cpu.rs`), and copper timing to the DMA cycle (`render/copper.rs`). About 2 000–2 500 lines. Under
the faithful bar most of it can become a per-line model. Clean-up only; low risk.

Two deeper changes were weighed and are not proposed without a concrete goal:

- Drawing and sound expressed directly ("tile here, sample at this pitch") instead of copper lists,
  bitplane buffers and Paula writes. About a third of `sf-core` changes; worth it only for an HD
  renderer or a non-Mac build.
- Replacing the byte-exact memory image (game state at the original's addresses) with plain Rust
  structures. Touches every routine and breaks transplanting state to and from the original, which
  every conformance check relies on. For readable code (the editor, mods), add a named layer on top
  of the image instead.

### Widescreen margins (decide when T27 is built)

The renderer already draws widescreen's extra columns from the level map, and the simulation stays
untouched. Objects only exist inside the original 320-pixel window: they come to life as their
cells scroll into it, so the margins show terrain but no enemies, and objects pop in at the old
edge. The options:

- accept the pop-in (the current design, `DESIGN.md` §2.7);
- draw idle stand-ins from the map's markers until the real object comes to life: visual only, so
  still faithful (recommended);
- a "living margins" mode that activates objects earlier: changes gameplay, so a separate mode the
  faithful checks do not cover.

Also to decide: level edges (bars, a clamped camera, mirrored terrain), the fixed 320-pixel screens
(title, menus, tally, slot machine: pillarboxed), the 320-pixel status panel, and secrets that a
wider view reveals early.

### A shared Amiga toolkit, before a second game

About a fifth to a quarter of the engine is general to any Amiga game: `sf-oracle` (vAmiga headless
plus the Moira routine runner), `sf-paula`, the comparison engine in `sf-conform/src`, the `sf-app`
shell, `sf-e2e`, the ADF/zip/cache code and the OCS renderer (copper, bitplanes, sprites). The rest is
Superfrog: the game logic, its asset formats (the custom trackloader, `ATN!`, `T7MP`), the memory
map and all conformance cases. Before a second game, move the general parts into a toolkit both
games depend on. The method carries over as is, and a second game should start on the faithful bar.

Mashups with a recompiled game from another machine are not proposed as an engine merge: two
machines, two memory images, two engines each faithful to one game. Borrowing characters and
assets is realistic (DKC1Recomp's "Baby Kong" mod brings DKC3's frames into DKC1).
