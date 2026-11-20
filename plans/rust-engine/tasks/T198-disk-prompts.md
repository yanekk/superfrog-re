# T198 — disk-prompts

**Split from:** T21 (its "disk prompts" test, `DiskPrompt` in `FrameOut`, and the purity fix in
`front/chain.rs`)

**Phase:** 2 · **Depends on:** T21; blocks T23, T29 · **Weight:** small

## Goal

The engine raises the original's insert-disk prompt on the same frame of play as the original,
and `FrameOut` carries it for the app (DESIGN §3.3: `disk_prompt: Option<DiskPrompt>`). The work
was built and mostly tested in T21 and is in `plans/rust-engine/T198-wip.patch` (uncommitted when
pir's context warning stopped T21): `git apply` it first, then finish its regression run.

What the patch does (each measured on the oracle in T21):

1. `layout.rs`: `WantedDisk = "wanted_disk", h0(0xaf54), Width::L` (gameplay): the disk ID
   `disk_request` (h0+$ade4) last asked for. It changes only when the file's disk is in no
   drive (`track_read` h0+$b098 checks `drive_disk_cache` h0+$af64 first); after the boot it is
   always the disk in df0, so "asked for a disk ≠ `wanted_disk`" is the prompt.
2. `front/mod.rs`: `Cx::read_entry(entry)` / `read_entries(from, to)`: a `file_table` entry
   (h0+$cbfe, 6 bytes: sector, sectors, disk index into `disk_id_table` h0+$b11e, an **h0 offset**,
   not `at()`), found by scanning the original's `lea file_table+n,a2` ($45f9) in the routine's
   code. `file_names` and `file_table` are **not** parallel arrays: read pairs from the code.
   Called at every load: title (h0+$1dd2..$1e20), awards (h0+$1ee6, $1fa8), demo (h0+$a580..$a65c),
   world loads (split at their `bsr world_intro_show`: FX, MU before; LP in `WorldIntro`; BM, BO, MS
   after), level maps (first entry; W6's first set, both sets on disk 4), end pictures, the final
   level, `end_seq_load` (END1/2 on disk 3, then $1d6c's MUSC on disk 1), `load_hero` (h0+$9178),
   the slot machine (disk 1's copies when `wanted_disk` is disk 1, else disk 2's: h0+$4a96).
3. `game.rs`: `pub enum DiskPrompt { Insert(DiskSlot) }`, `FrameOut::disk_prompt` from `Cx`.
4. `milestones.rs`: `Milestone::DiskAsked { disk }` when `wanted_disk` changes.
5. `front/chain.rs`: h0+$172d6 (the map area's `clr.b` loop, $102e0 bytes) as `ending::work`
   CPU work before a world's tile loads: ~17–20 frames on the original under the loading
   picture (W6's disk 4 asked for at 8767, not 8748). `BONUS_PICTURE_FRAMES` re-fitted to 451.
6. `front/chain.rs`: the 24-element `LEVEL_LOADS` literal (purity, red since T190) replaced by
   `level_load_of`, read from `top_loop`'s code.

## Files

As the patch: `sf-core/src/{layout.rs,game.rs,lib.rs}`, `front/{mod.rs,chain.rs,title.rs,
attract.rs,ending.rs,hero.rs}`, `slot/mod.rs`; `sf-conform/src/milestones.rs`;
`sf-conform/tests/{chain.rs,flow.rs}`.

## Interface

`sf_core::DiskPrompt`, `FrameOut::disk_prompt`, `sf_conform::milestones::Milestone::DiskAsked`.

## Tests

- [ ] `flow.rs` `disk_prompts_are_raised_as_the_game_asks_and_need_no_answer`: W1 none, W4
      `[Insert(D2)]`, W6 `[Insert(D2), Insert(D4)]` (in the patch, green).
- [ ] `chain.rs` `a_level_code_starts_the_game_at_w3_1...`: `DiskAsked { disk: 2 }` at 1494 on
      both sides (in the patch, green).
- [ ] Regression with the patch: green in T21 were `chain`, `flow`, `front`, `bosses`,
      `game_over`, `alone`, `purity`, `slot_routines`, `cases`, `story`, `sync_points`,
      `tally_routines`, `frog_cases`, `enemies_w2_cases`. Still to run: `enemies_w1`, `enemies_w3`,
      `enemies_w5_cases`, `enemies_w6_cases`, `faithful`, `runner`, `beam` (`wanted_disk` is
      compared in every case now).

## Done when

- [ ] The above green; `purity` green; `DiskPrompt` exported for T23.
