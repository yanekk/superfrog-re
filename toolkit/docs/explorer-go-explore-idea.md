# Backlog: an explorer in the Go-Explore style

Moved from Claude's memory 2026-10-30; checked against the planner and plans/smart-planner.
Corrected: the routing use is built (below), and `plan_exit` is retired. The testing use stays
backlog; nothing has been built for it.

Owner asked 2026-10-15 to keep this for later; the testing half is not in any plan yet.

**Idea:** Go-Explore (Ecoffet et al., Uber AI 2019; Nature 2021 "First return, then explore"): an
archive of "cells" (coarse state summary), each with a snapshot and the shortest inputs reaching it;
loop: pick a rarely-visited/new cell → restore its snapshot (free: `Game` is `Clone`) → explore with
short random/sticky input bursts → add new cells, keep shorter routes. Deterministic engine means no
"robustify" phase. Cell = frog tile area + coins taken (banded) + switches pressed + items held;
granularity is the main knob.

**Use 1, routing (built):** was to replace hand waypoints when `examples/plan_exit` (distance-field
beam search, engine-planned, original-confirmed in chunks) sticks in pockets/switch detours (findings
2026-10-15). Explore on the engine, then confirm/repair on the original (the original lags; the engine
doesn't, so routes drift).
- learned on: Superfrog. Built as `sf-plan` (plans/smart-planner): 21/24 single levels on the original, 18/24 in a whole-game run, 22/24 at app timing, all guided, against targets of 24. Its archive and explorer are game code, not the toolkit's.

**Use 2, testing (backlog):** goal-free exploration maximising coverage, every new cell compared with
the oracle (faithful checks + plays-alone) = an automated "wandering player" bug finder. Natural
follow-up to the bug recorder (finds turn into recorded repro cases).

**Not recommended:** a trained game-playing model; search with save states beats it here.

**How to apply:** raise when planning testing, or when a new game's routing is the bottleneck.

**Update 2026-10-17:** the main proposed improvement was a movement-aware reachability graph (plan_exit's
field is geometry only; caused the T205–T207 stalls). Written up in `ROADMAP.md` on `roadmap/milestone-2`
(PR #2, still unmerged), with route repair (`--follow`, built in T207) and keeping the archive between coins.

**Update 2026-10-20:** brief for `pir plan` drafted (smart-planner): unaided benchmark (no per-level flags, 30 min/level, confirmed on original) on the 24 levels + whole-game legs; baseline of today's tools measured first; targets M1–M9 (24/24, stall levels 4/4, median ≤10 min, ≤25% longer, ≥20/24 zero-damage). Easy/hard deferred to after difficulty modes (owner to confirm).
