# Findings log

**What the build taught.** Read the rows touching the task you pick up; read it whole before anything only a person can verify. Newest first, forty words a row, counted. Whoever appends, compacts: over 60 rows or 15 KB, shrink first. Never drop a ✅ row or its date, or a term someone would grep for. Flat prose.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-11-11 | 🐞 | `pcode_export.py` `load_symbols` crashes with a bare `KeyError` traceback on a `symbols.toml` row missing `hunk`, `offset`, `name` or `kind`, not exit 2 naming the file (T03 review sweep). Predates T03, left alone. |
| 2026-11-11 | 🐞 | `pcode_export.py` `load_entries` (`--routines`, `translate.toml`) crashes with a bare `KeyError` traceback on a routine row missing `hunk` or `offset`, not exit 2 naming the file (T03 review sweep). Predates T03, left alone. |
| 2026-11-11 | 📌 | **T03 whole export:** 6 684 instructions (h0 5 774, h4 910), 143 entries (program-entry 1, symbol 142), vectors `h0+$1eda`, `$1f7a` (both symbols); 24 `unresolved_computed`, every one an OS call `jsr -n(a5|a6)`. Trial on it: 84 of 85. |
| 2026-11-11 | 📌 | **T02 code-seen:** 48 runs, 6 380 instructions, 135 entries (program-entry 1, symbol 134, vector 0, trace 0); every computed target is a named symbol. `world_index` poked before Start is reset to 0, so world starts use `next_world`. |
| 2026-11-11 | 📌 | `os-surface --worlds 1,3,6` (T01 review) lists `lea $0.l` to `$3.l` at `h0+$691e`..`$6930`, constants in play, as `[[read]]` OS memory with access `address`. Not in the committed worlds 1,2 file; not OS use. |
| 2026-11-11 | 📌 | docs/07 says the file routines open dos.library by `OpenLibrary`; they call `OldOpenLibrary` (-408), as `entry` does for graphics (T01 review, listing). docs/15 is right; docs/07 left alone. |
| 2026-11-11 | 🐞 | docs/07 says the OS's interrupts and keyboard run through the title. Measured (T01): none is taken, autovectors 2 and 3 are the program's (`load_list_title` ends in `system_off`), so the title's Esc route is live; not run. |
| 2026-11-11 | 📌 | Not in docs/07 (T01): `entry` opens graphics.library, calls `OpenFont` topaz (result read, never used) and graphics `FreeSprite` (-414) where `CloseLibrary` was meant; `h0+$51a` writes `ciab_prb` to latch the motor off. T26 answers these. |
| 2026-11-11 | 📌 | `world_index` cannot be poked before `load_world`: `title_code_world` writes it in the same step. T01 sets the immediate at `h0+$778e` (the no-code branch `h0+$778c`) instead; T33's `set` meets the same step. |
| 2026-11-11 | 🐞 | `translate.py` turns a Muse call error on every try into `fail`, not a missing-model result: the class T17's review fixed in generate and reshape (`no-model`). Left alone; T18 deletes Muse's translate.py. |
| 2026-11-11 | 📌 | `claude -p --tools ""` from a temp folder (claude 2.1.322): no tools, 631 input tokens, no user CLAUDE.md in context. `--max-turns` works though `claude --help` does not list it. |
| 2026-11-11 | 📌 | First real `generate.py` on Haiku: `h0+$513c` covered, 4 of 4 branches, 2 000 returns, 2 tries (the first left its name out of `ALL`), 22 s, $0.011 list. `claude -p` keeps no conversation; retries go as one transcript prompt. |
| 2026-11-11 | 📌 | `docs/11-translation.md` still says generate and reshape call Muse with translate.py's key and spend cap; T17 changed only `tools/README.md`. Left for T20 (docs-and-packs). |
| 2026-11-11 | 📌 | **T00 seam: mount `game_frame` whole.** Transplant in, `game_frame_on`, write back what differs: 2 000/2 000 `game_frame` states, 34.5 bytes changed a pass, max 59 (review's count). T35 mounts by address; T36 mounts only `h0+$1fec`. |
| 2026-11-11 | 📌 | Per routine also passes: `char_sprite_position` and `camera_update` mounted alone, and both inside translated `game_frame`, 2 000/2 000 each with no registers compared. All registers fail: ports leave scratch d0-d3, d7 (487/2 000 at `game_frame`'s exit). |
| 2026-11-11 | 🐞 | A write-back compares with memory's bytes, not the typed state before: `camera_update`'s `crossed` flags are bools, so a poked `$6c` stayed and 288/2 000 failed. Camera needed four values outside `LAYOUT` (`h0+$4d76`, `$4d7a`, `$51f8`, `frame_flag`). |
| 2026-11-11 | 📌 | Round trip (write back whole, transplant again) over `camera-right-left`'s first 100 steps: 0 equal. Not transplanted: `edges.pos` (`h0+$1e66`), pointers `p840`, `p83c`, the `input_log` bytes. Screen and plane share `h3+$fa10..$fa4f`; 4 bytes disagree. `game_frame` reads none. |
| 2026-11-11 | 📌 | Whole write-back is 129 500 bytes (display 75 101, object map 38 400, plane 15 152, outside-`LAYOUT` extras 539, `LAYOUT` 207, input log 101); the frame mount stores only what changed. |
| 2026-11-11 | 🐞 | `Cpu68k::call` with an odd entry runs the instruction at the even address below it (probe: call(0x2001) ran moveq at 0x2000); Moira's reset ignores bit 0. Fixed in `step` (T05 review), left open in `call`. |
| 2026-11-11 | 🔄 | Owner first chose retiring Muse and all four model tools, then the same session chose Haiku through the subscription for `generate.py` and `reshape.py`, after a trial (DESIGN §2.13). |
| 2026-11-11 | 📌 | `claude -p --model haiku --max-turns 1 --output-format json` works from a script here: model `claude-haiku-5-5`, 1.7 s, $0.003 list. Logged in through claude.ai. macOS has no `timeout`; bound runs in Python. |
| 2026-11-11 | 🐞 | Smoke in a fresh worktree (cold build) hit `tools/smoke.py`'s 1 200 s limit, no test failed: forest-dumb done at 783 s, Superfrog unrun. Owner chose to start the build anyway (plan review). Translated-program tests go in full. |
| 2026-11-11 | 📌 | `generate.py`, `reshape.py` and `translator_trial.py` import Muse's `translate.py` for the judge lock, `JUDGE_SECONDS`, `flow_text` and the model call; T17 moves these out before T18 replaces `translate.py`. |
| 2026-11-11 | 📌 | The OS is live through the title (`system_on`, the OS keyboard handler, `irq_count` still) and every file loads through `dos.library` (`docs/07-executable-map.md`). The machine answers these; T01 measures exactly which. |
