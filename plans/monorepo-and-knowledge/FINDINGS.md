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
| 2026-10-30 | 🔄 | Owner stopped T02's whole-suite run; it runs after the plan is built. move-disks.sh not run, since its seatbelt needs a green suite: run it after that run. Lookup falls back to the root meanwhile. |
| 2026-10-30 | 🔄 | Owner kept the six hardware chapters game-side: they name the game on 322 lines and own game files. Added T05 to carve the generic parts into toolkit/docs/architecture/. No `learned on:` marking to pass the check. |
| 2026-10-30 | 🐞 | DESIGN and T02 name tools/select.py, but that shadows stdlib `select` and breaks `import subprocess` for every script in tools/. Named tools/select_scope.py; test_select.py keeps its name. |
| 2026-10-30 | 📌 | Owner's existing bug zips stay in the main checkout's root bugs/; the app now saves to games/superfrog/bugs/. bug_zips.rs reads the game folder if present, else the root; move-disks.sh moves disks only. |
| 2026-10-30 | 📌 | T03: toolkit/PLAYBOOK.md and toolkit/template/ must be owned by a chapter (`owns` in games/superfrog/docs/architecture/verification.md) or test_architecture_docs.py fails. |
| 2026-10-30 | 📌 | check_tier_names.py tokens have no right boundary, and `team 17` no left one: `steam 17` and `team 170` fail it. Left as is, the error is loud and only ever on the strict side. Also empty_memory.sh leaves dotfiles alone. |
| 2026-10-30 | 📌 | T01 ran tools/empty_memory.sh: twelve memory files deleted, MEMORY.md is the pointer. The originals are gone, so a review compares notes with the code. Way back: copy the notes from the repo. |
| 2026-10-30 | 📌 | ~/Documents/Amiga is not listable by sessions (macOS TCC): a worker cannot check the Forest Dumb zip names itself. T04 records them from the north-star; the owner's cracked-copy answer confirms the set. |
| 2026-10-30 | 📌 | Fresh-copy run of the stored setup and test lines passes: Python 82 tests (2 skipped, 23 s), engine and toolkit suites exit 0; a warm engine run exceeded 30 min, so the everything scope is ~an hour. |
| 2026-10-30 | 📌 | The `learned on:` marker already exists in six toolkit doc comments (toolkit-extract); the tier-names check reuses it, and its tests must keep those lines passing. |
