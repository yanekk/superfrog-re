# T82 — route-picture

**Phase:** 6 · **Depends on:** T48; blocks T29 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T82` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T48 with the owner's yes (2026-11-10). T48's route (`engine/conform/cases/route-W1-1.toml`) agrees with the original on every variable of all 21 764 passes. Its picture is compared on all 22 152 frames and fails the owner's tolerance, as the runner counts it, on 40 of them, which the case sets aside in ten boxes naming T82, each on its own frames.

## Goal
`route-W1-1` compares its whole picture with no exclusion naming T82, and passes. Each of the three kinds below is explained by a measurement on the original, then either fixed in the engine or, where it is timing inside a frame (DESIGN §2.1), accepted by a rule the owner has approved.

## The three kinds, with their frames (measured by T48, 2026-11-10)
Oracle frames; the sync point is 6301. Left of each diff picture is the original, the middle the engine.

1. **A taken item erased behind the beam.** Frames 15 813 (pass 9 511, 78 pixels, box `rows 88..97 cols 163..176`) and 20 693 (100 pixels, `rows 31..42 cols 115..130`). `h0_29ca`'s tile blit for the taken cell lands mid-frame: measured at 15 813, beam lines 142 and 152 for a cell on beam lines 135 to 150, so the frame still shows the item's top rows. The engine erases the cell whole (`objects::collectible::redraw_cell`, `Blit::Cell`).
2. **The countdown's and the score's last digits through one status period.** Frames 18 364 to 18 395, passes `32k + 1` to `32k + 31` from `irq_count` 12 065 (boxes `rows 226..228 cols 162..166` and `rows 236..244 cols 273..288`). The original shows 809 and the old score digit, the engine 808 and the new one, and no picture `Game::cut_redraw` draws for the measured cut matches (T71). Not measured: where the redraw was cut, and whether the numbers changed inside it.
3. **The first rows of the view at the map's top.** Frames 17 196 (102 pixels, `rows 0..9 cols 131..147`: the engine shows an object in the top row the original does not show yet), 18 896 (71, `rows 0..7 cols 92..108`: a sliver of a fruit), 19 454 (279, `rows 0..11 cols 228..260`: two oranges the engine shows and the original does not), 19 456 (223, `rows 0..9 cols 236..268`), 19 459 (103, `rows 0..5 cols 241..271`) and 19 463 (102, `rows 0..5 cols 242..271`). The camera is in the map's top rows there. Not measured: whether the original draws them later (a row redraw's bobs start near beam line 197) or never.

## What T48 found about the count, to settle first
`fd_conform::run_inputs` calls `PictureJudge::frame` only for a frame that is not `Same`, so the judge's run of glitch frames is ended by a few-pixel or colour-step frame and never by an identical one. "3 frames in a row" in a failure therefore means three glitch frames since the last few-pixel frame, however far apart: 15 813, 17 196 and 20 693 are single frames. Whether an identical frame ends a run is a comparison rule: ask the owner before changing it, with how many of the 40 frames each reading leaves.

## Rules the owner set (2026-11-10)
- Any new comparison rule this task needs is raised as a question before it is built.
- It follows T71's pattern: it accepts only what the original could show, proven per frame from a measurement on the original, and nothing is looser on any other frame.
- No existing task is edited.
- Any measurement added on the original must be shown to observe only (owner, 2026-11-09), as `edge_latch.rs`'s `sampling_for_the_edge_cut_leaves_the_original_unchanged_every_frame` holds for the edge cut.

## Design sections this implements
§2.1, §2.2, §4.1.

## Reuses
T71's cut redraw (`redraw_cut`, `Engine::cut_redraw`), T80's creature cuts (`creature_cuts`, `rows_behind_beam`, `Engine::creatures_uncut`: the beam against a blit's rows), `Amiga::cpu_writes` and `blit_events`, `Options::diff_dir` for a failing frame's picture, T48's frame-scoped boxes (`Exclusion::frames`), `games/forest-dumb/docs/13-full-route.md` ("The picture on the route").

## Files
`engine/conform/cases/route-W1-1.toml`, `fd-conform/tests/full_route.rs`, `fd-conform/src/lib.rs`, `fd-core/src/**` (fixes only), `engine/conform/README.md`, `games/forest-dumb/docs/13-full-route.md`, `smoke.toml` if a test is added.

## Interface
None new unless a rule is approved; then a method of `Engine`, forwarded as `creatures_uncut` is.

## Tests
- [ ] `route-W1-1` has no `exclude` and passes; `full_route.rs` asserts none.
- [ ] Each kind has a test that fails without its fix or its rule, on the frame named above.
- [ ] A frame differing the same way with no measurement behind it still fails (as `creature_cuts.rs` holds for T80).
- [ ] The level cases that compare the picture still pass (`picture_cases.rs`, `collect_cases.rs`, `death.rs`, `mechanic_cases.rs`, `enemy_cases.rs`).

## Done when
- [ ] The tests above pass and the case's comment and `docs/13` say what each kind was.
- [ ] Every rule added is in `engine/conform/README.md` with its measurement and the owner's yes, dated.
- [ ] If the three kinds need more than one session, the owner is asked to add a task for the rest, split by kind.
