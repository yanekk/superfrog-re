# T06 — fd-exe-map

**Phase:** 4 · **Depends on:** T02, T03 · **Weight:** heavy

## Goal

Map the main program once, so every porting task starts from a shared map instead of finding the
main loop again (DESIGN §2.7, playbook step 4), and choose the variable the first conformance
case models, with evidence (DESIGN §2.8). The executable is a hand-written 68k AmigaDOS hunk
file of 5 hunks, stored unpacked on disk, so the existing toolkit tooling applies directly.

## Design sections this implements

DESIGN §2.7, §2.8 (the variable's criteria), §3.5 (regenerated listings stay out).

## Files

- `games/forest-dumb/re/symbols.toml` — the committed names: hunk, address, name, one-line note,
  Superfrog's schema.
- `games/forest-dumb/tools/re_listing.py` — regenerates the annotated listing into `build/`
  (never committed, it derives from the copyrighted executable), built on
  `toolkit/tools/hunk_listing.py`.
- `games/forest-dumb/docs/07-executable-map.md` — the map: the five hunks and the entry point,
  the main loop, the custom-chip write sites (the ~400 absolute `$dff0xx`/`$dff1xx` references),
  the dos.library usage and the file table (names crossed against the inventory and T03's scan),
  the title screen's code path, the memory layout.
- `games/forest-dumb/engine/crates/fd-tools/tests/exe_map_claims.rs` — asserts the map's claims
  against the real executable, the pattern of Superfrog's `exe_map_claims`.
- `games/forest-dumb/CLAUDE.md` — index lines.
- Ghidra is installed (`ghidraRun`, plus the MCP server); `toolkit/tools/ghidra_headless.py` and
  `ApplySymbols.java` are the import path. Decompilation output goes to `build/` only.

## Interface

`symbols.toml` uses Superfrog's schema (a list of `{ hunk, addr, name, note }`); the doc names
the hunks so an address reads `h0+$12de`-style in later tasks. The map doc's file table lists,
for each file the game opens: the name, which disk, when it opens it.

## The variable choice

With the map done, pick the variable the first case models. The criteria, all four must hold:
deterministic across the spike's two identical runs; visible at the idle title screen (no input
needed after the sync point); computable purely from the game's state at the sync point, so
`fd-core` can model it without I/O; small enough to model without embedding a table over the
purity limit (a bigger table comes from the disks through `fd-conform`, not from the crate).
Deliver the choice with the evidence: the oracle address (hunk-relative), the expected values and
range, and which frames it changes on, from a short oracle run of the title. If no candidate
holds all four, stop and ask the owner: the FINDINGS row names the near misses, and the question
presents the options (relax one criterion, or use another screen).

## Tests

- [ ] `exe_map_claims.rs` on the real executable: the entry point's first instructions match the
      map, every file name in the map's table appears in the inventory, and the variable's
      address falls inside its hunk.
- [ ] `re_listing.py` regenerates the listing on the real executable without error.

## Done when

- [ ] The map doc and `symbols.toml` are committed, the listing regenerates, and the claims test
      is green on the real executable.
- [ ] The modelled variable is chosen with the evidence above, recorded in the doc and in a
      FINDINGS row; or the owner was asked and the answer is recorded.

## Outside actions

None.
