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
| 2026-10-31 | 📌 | T02 cross-check: `xdftool unpack` on both real disks agrees with `fd-tools extract`: 61 files (12 + 49), names, sizes, all bytes identical. |
| 2026-10-31 | 📌 | Unused `amiga-*` profile overrides in `games/forest-dumb/engine/Cargo.toml` print eight cargo warnings on a non-quiet build until T08/T09 depend on those crates; harmless, `-q` hides them. |
| 2026-10-31 | 📌 | OFS verdict: `amiga-ffs` 0.4.0 (MIT/Apache-2.0, no dependencies, `std` off) chosen. Against `xdftool` on both real disks: 12 and 49 files, names, sizes, bytes identical. `affs-read` not needed. |
| 2026-10-31 | 📌 | `s/startup-sequence` sits in a subdirectory, so `OfsImage::files()` lists paths (`s/startup-sequence`), against T01's root-only doc. T02 must create parent folders under `out`; disk 1 lists 12 files. |
| 2026-10-31 | 📌 | `pir`'s hand_command cannot read `~/Documents` (macOS refused the zip copy); the owner copied them from outside. Zips now in the main checkout's `games/forest-dumb/`. |
| 2026-10-31 | 📌 | `toolkit/crates/dry-fit/tests/deps.rs` still has only the first game's `GAME_PREFIX`; left alone, outside T01's file list. |
| 2026-10-31 | 📌 | The stored setup/test lines pass in a fresh worktree: setup leaves the copy clean and the everything-run (toolkit, both games, root checks) is green in ~70 minutes. Re-measured because the plan adds a package folder. |
| 2026-10-30 | 📌 | The only permissive Rust OFS readers are `amiga-ffs` (MIT/Apache-2.0, 0.4.0) and `affs-read` (MIT, 0.5.1); the `format198x` ADF family is GPL-2.0-or-later. Found while planning, DESIGN §2.2. |
| 2026-10-30 | 📌 | `amiga-oracle` already reports each stepped frame's length in lines (`amiga_frame_lines`, shim.h: 313 long, 312 short), so T00 measures the frame length with no new patch. |
