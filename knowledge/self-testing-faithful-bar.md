# Self-testing, the faithful bar

Moved from Claude's memory 2026-10-30; checked against plans/rust-engine/DESIGN.md §2.1 and the
conformance tooling. Corrected: generalised to any game's engine (rule 3); the bar was set on the
first game's Rust engine.

The owner wants a game's Rust engine to act and feel like the original, checked automatically and
headless by agents against the original on the oracle, never by the owner playing or looking.
Frame-exact was dropped on 2026-10-15 for "faithful" (rust-engine DESIGN §2.1: state per pass,
tolerant picture/sound, engine alone meets the original's milestones).

**Why:** frame-exact chasing (beam races, bus-cycle models) consumed days while the engine still
couldn't start a game from the menu; self-testing was the project's original idea.

**How to apply:** don't propose hand-verification steps or sub-frame timing work; propose
automated checks against the oracle instead. When amending a live pir run, see
[pir-midrun-amendment.md](pir-midrun-amendment.md).
