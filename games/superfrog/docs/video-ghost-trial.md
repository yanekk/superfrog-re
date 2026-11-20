# Video ghost trial

Moved from Claude's memory 2026-10-30; checked against git branches and sf-plan. Corrected:
`--guide-ordered` reached main through feat/recorded-guide, fix/world6-palette is merged, and
experiment/ghost-guide no longer exists (it was folded into worktree-video-ghost-trial, unmerged).

Owner asked 2026-10-25: trial on W1-1, then "find the level the smart planner struggles with most
and recreate it with the ghost approach". Video: World of Longplays (youtube 5GQqdGzaARY), owner's
pick, yt-dlp (brew), files in `~/.cache/superfrog-re/video/` (not in git); results in `.../trial/`.

Hardest level = W6-4 (fails sets A, B, C; stalls 13-48 cells short). In the video at 1:56:48-2:02:00.

The trial's code is on one unpushed branch, `worktree-video-ghost-trial`, on local-only history:
`tools/video_trail.py` (tracker, `--palette`, `waypoints`), examples `video_truth`, `level_palette`,
`ghost_chase` (beam chaser, failed), and sf-bench example `verify_route`. `sf-plan --guide-ordered`
(guide index inherited along the trajectory, exit only at the guide's end) is on main.

Results: W1-1 ordered 78% of the player's path incl. the tunnel. W6-4 ordered, `--shorten-share 0`:
seed 1 exit 15642 frames, real-app check passed, 4 hearts; seed 2 exit; seed 3 stalled 48 short.
Loose guide with the same trail: no exit.

**Why:** owner wants agents to replay human play. **How to apply:** using video trails in the planner
is the owner's call (plan change). Related: toolkit/docs/explorer-go-explore-idea.md.

2026-10-26: tried `--guide-only` (no compass pull while following an ordered guide); W6-4 seed 3 still stalled 48 cells short, so it was dropped from both branches at the owner's call. The stall is the video trail's precision on the climb, not the exit pull; the owner's recorded climb fixed it (see [planner-stall-ask-owner.md](planner-stall-ask-owner.md)).
