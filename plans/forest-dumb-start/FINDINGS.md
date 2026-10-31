# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before
anything only a person can verify — a ✅ row is the *entire* record that something was seen
working for real.

**Newest first. Forty words a row, counted, not estimated.** The long version is already in
the commit message that carried the fix. This is the index, not the account.

**What goes here:** what the next session would otherwise rediscover · what you noticed and
left alone under the scope rule · every answer that came back from the user's own hands, dated.

**What does not:** a decision and its reasoning (that is the commit message) · anything the
code or a test now states for itself · a restatement of the row above.

## Writing a row

Flat prose. At most one bold phrase in a row, and none is better. No em-dash asides, no
clause that is there to sound right. If a row reads like it is arguing a case, it is too long.

> 🐞 `DRY_RUN=1` was ignored on an already-connected slug: the repair branch is decided above
> the dry-run check, so it opened a real browser. Fixed in review, +4 tests.

Thirty-two words, and it is a whole finding — what broke, why, and what happened about it.

## Keeping it short

**Whoever appends, compacts.** Before adding a row, if this file is over 60 rows or 15 KB,
spend two minutes shrinking it. That is the only maintenance it gets, and it is what stops
this file becoming the one nobody reads.

- **Merge** rows that are one lesson learned twice.
- **Drop** a row whose lesson is now enforced by code, by a test, or by a rule in `DESIGN.md`,
  and name where it went. The test is the better record.
- **Never drop a ✅ row or its date.** Shorten it; never lose it. `CLAUDE.md` makes this file
  the only record that anything was ever seen working for real.
- **Never drop what somebody would grep for** — the flag, the error string, the path.

**Promotion is a move, not a copy.** When a finding binds every session rather than one task
it goes to the project's traps list in `CLAUDE.md` as **one line** — the rule, without its
rationale — and the row here becomes a pointer to it. Two copies both grow, and that list is
loaded into every session whether or not it touches the thing.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-31 | 📌 | T00 review, worker-driven: with disk 2 in df1 the boot runs about 170 frames later, so the one-drive input script stalls on the crack picture. The title stands still to frame 14000 at least, past the 4078 frames measured. |
| 2026-10-31 | 📌 | Gate verdict go. Kickstart: `kick1.3.rom`, SHA-1 90933936cce43ca9bc6bf375662c076b27e3c458, boots the set first try; 2.04 and 3.1 not needed. |
| 2026-10-31 | 📌 | Drives: disk 1 alone in df0, then swap. The prompt (frame 3461) says insert disk 2 in DF0, press fire. Disk 2 in df1 from power-on still prompts; fire without the swap stays on the prompt. |
| 2026-10-31 | 📌 | Repeatable: two lockstep runs byte-identical every frame in chip, slow, picture, Paula, taps, mix and CPU: 3 000 frames idle and 7 000 frames scripted to the title. No configuration switch needed. |
| 2026-10-31 | 📌 | Boot to title: first title frame 4921 with Esc 800-810 (Show exits only on Esc), left mouse 1300-1310 (crack intro; a click at 1200 is ignored), `insert_disk(0,1)` at 3600, fire 3700-3710. Earlier or later inputs move the count. |
| 2026-10-31 | 📌 | Frame length: 313 every frame from the crack intro on, checked to 14 000 frames. 75 frames of 312 (677-825 alternating) occur only under Show's interlaced picture. VPOSW writes: 105 twice (8001), 1192 once (8000). |
| 2026-10-31 | 📌 | Input works: port 2 fire and joystick, keyboard, port 1 button. Down at the title changes the picture; right in level 1 changes RAM on its frame 10000. Fire at the title loads `SCENA 1`, fire again enters the level. |
| 2026-10-31 | 📌 | Shim reads work: chip, slow, frame, Paula writes, audio taps; no new core patch. Snapshots at 4000 (mid-load) and 6000 continue identically. Speed about 550-610 frames per second headless, with per-frame hashing. |
| 2026-10-31 | 📌 | Disk 2 is read after the intro (DSKLEN 3791-4916). At frame 5500 `_BALLA` is whole in chip and `_OPCJE` whole in slow RAM. The title picture stands still 4078 frames while RAM changes every frame. |
| 2026-10-31 | 📌 | In a level `Rgb12Frame` holds only the 320x56 status bar: the crop uses the DIW at frame end, and the game moves it mid-frame. Title unaffected. Left alone; a picture comparison in play needs the crop changed. |
| 2026-10-31 | 📌 | Title reads COPYRIGHT '96 L.K. AVALON, so the year is 1996; `games/forest-dumb/CLAUDE.md` corrected. Crack intro: cracked 06-11-96, Wanted Team. Picture at `games/forest-dumb/build/spike/title.png`: title frame, witch, cauldron, brick wall, menu START, KODY 0556, KONIEC. |
| 2026-10-31 | 📌 | The stored setup/test lines pass in a fresh worktree: setup leaves the copy clean and the everything-run (toolkit, both games, root checks) is green in ~70 minutes. Re-measured because the plan adds a package folder. |
| 2026-10-30 | 📌 | The only permissive Rust OFS readers are `amiga-ffs` (MIT/Apache-2.0, 0.4.0) and `affs-read` (MIT, 0.5.1); the `format198x` ADF family is GPL-2.0-or-later. Found while planning, DESIGN §2.2. |
| 2026-10-30 | 📌 | `amiga-oracle` already reports each stepped frame's length in lines (`amiga_frame_lines`, shim.h: 313 long, 312 short), so T00 measures the frame length with no new patch. |
