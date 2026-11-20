# Backlog: difficulty modes

Moved from Claude's memory 2026-10-30; checked against layout.rs, the conformance tests and the
bug-recorder and smart-planner designs. Corrected: the original's options screen offers only easy and
normal (bug-recorder D12: `opt_game_mode` stops at 1), and recorder_setup_start now also sets the
mode. Milestone 2 is built. Still unplanned.

Decided 2026-10-18: the `rust-engine` run finished without difficulty-mode coverage. Next, a plan
of its own: re-run existing conformance cases with `opt_game_mode` (h0+$9e6: 0 easy, 1 normal; the
note first had 2 hard, which no menu can select) set at the sync point, judged faithfully. No route
planning needed (same inputs both sides). All frame/completion/whole-game cases are normal; only
`restart_routines.rs` and `recorder_setup_start.rs` vary the mode.

Separately, completing levels in easy and hard mode is the proving ground for the smart
(movement-aware) planner, benchmarked against T205–T207's per-level times
(toolkit/docs/explorer-go-explore-idea.md); smart-planner DESIGN puts them after this plan's
conformance, the owner's order. Whether a hard mode exists to complete is a question for that plan.
Both written in `ROADMAP.md` on `roadmap/milestone-2` (PR #2, still unmerged).

**How to apply:** propose planning this when the owner asks what's next for Superfrog.
