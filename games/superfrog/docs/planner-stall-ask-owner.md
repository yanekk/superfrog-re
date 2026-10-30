# When the planner stalls, ask the owner to play it

Moved from Claude's memory 2026-10-30; checked against the tools and README. Corrected: the tools are
on main (feat/recorded-guide merged). Stays at tier 3 until the planner is generalised (DESIGN §7).

Owner rule (2026-10-25): when `sf-plan` struggles on a level (no exit on two seeds, or the same stall
twice), stop and ask the owner to play the stuck stretch, instead of adding per-level hints or tuning.
The procedure is in games/superfrog/CLAUDE.md ("When the planner struggles"); this is why and the order.

**Why:** W6-4 seed 3 failed identically on every run; one 44-second owner recording of the climb
(bug-W6-4-2026-10-25-1851) got it to the exit first time. The owner said: "if planner struggles, ask
me for help", and (2026-10-26) "I need to see where planner is stuck before I help".

**How to apply:**
1. Show the stall first: `cd games/superfrog/engine && cargo run --release -p sf-routeview -- route.txt [--guide g.txt]`
   (or `--planner`), page `build/route-viz/index.html`; name the spot `<level-id>:<col>,<row>`.
2. `python3 games/superfrog/tools/handover.py ROUTE.txt <level> <col>,<row>`: app script with fast-until, record,
   keys from the owner's settings.toml (the owner jumps with Space), and the owner's command.
3. `python3 games/superfrog/tools/recording_guide.py <zip>` → guide; `sf-plan <level> --guide-ordered --guide <it>`;
   show the new route in sf-routeview with the guide overlaid.

Related: [video-ghost-trial.md](video-ghost-trial.md).
