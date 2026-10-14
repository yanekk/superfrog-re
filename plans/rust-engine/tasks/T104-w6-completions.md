# T104 — w6-completions

**Phase:** 2 · **Depends on:** T40, T101, T102; blocks T18 · **Weight:** heavy

Added by T40 (owner, 2026-10-14). Carries T40's done-when "a scripted completion of each non-boss
level of the world matches the oracle": a W6 exit stays locked until `coins_left_for_exit` (h0+$832)
is 0, and at load that is every coin in the map (W6-1 97, W6-2 162, W6-3 138, W6-4 189), so each
completion is a full-coin run. Numbered T104: T103 is T41's world-4 completions.

## Goal

A scripted run of each of W6-1, W6-2, W6-3, W6-4 and the alt maps W6-2alt, W6-3alt, W6-4alt
(`sync = { level = "W6-2alt" }`; W6-1alt is the same map as W6-1) from its first frame through
every coin to the exit (`level_state` 1), matching the original frame by frame with no
exclusions, as cases `engine/conform/cases/enemies-w6-<n>[-alt]-complete.toml`.

## What T40 left

- `examples/plan_exit.rs`: plans on the engine from the original's memory, beam search toward
  `--via` cells then the exit over a distance field flooded through the map (solid cells cost
  60, so a switch's gate is crossed), plays each 48-frame chunk on the original and takes back one
  that costs a life; `--prefix` resumes from a script it printed. It does not collect coins yet:
  add a phase that steers to the nearest coin left (markers 8/9 in the live map; a taken coin's
  marker is cleared) until `coins_left_for_exit` is 0. About 2 minutes per 500 frames.
- `examples/map_route.rs`: the cheapest start→exit route and the solid cells it crosses (W6-1:
  gates of marker 61 at 111–112,20 and 271–272,35, opened by switches at 110,17 and 270,35).
- `examples/map_ascii.rs`: a map as text; `examples/w6_map.rs`: W6's markers per map.
- W6 is busy: the original's pass is late on about a third of frames (`enemies-w6-1`). A case
  that hits a T101/T102 lag fault stays outside the test block, naming that task.

## Tests

- [ ] one completion case per map above, green in `sf-conform -- all`.
- [ ] those not blocked by a T101/T102 fault run in the test block (`sf-conform/tests/`).

## Done when

- [ ] Every map's completion case matches the original to its exit.
