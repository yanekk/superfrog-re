# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**What goes here:** what the next session would otherwise rediscover · what you noticed and left
alone under the scope rule (an `owns` path that fits another chapter better, DESIGN §2.7) ·
answers from the user, dated.

**Whoever appends, compacts** (over 60 rows or 15 KB). Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-23 | 📌 | T18: no `owns` moved. `become_reward`, `LevelCtx`'s beam code, `frog_tiles`, `CpuWork`, `Routine`/`Cx`, `walk_step`, `DisplayData::new` and `Pacing`/`PassClock` are parts of files; `owns` assigns whole files, and each chapter already cross-links them. |
| 2026-10-23 | 📌 | T18: whole-file candidates also stay: `front/story/sim.rs` and `cpu.rs` (story-and-ending documents them in depth; cpu-time-and-beam links them) and `mem.rs` (memory-model counts its data half as `data` 15). A move would re-estimate both chapters' mix. |
| 2026-10-23 | 📌 | T18: the overview prints `totals`' combined ratings and, beside them, verification's split (T17 row): 21% of code carries as-is or with small changes, 16% as an approach, 63% superfrog-only. |
| 2026-10-23 | 📌 | T18: the T09 sweep's `hero_frames` slip in `copied-in-data.md` §8 stays, since T18 changes no chapter body but for ownership moves; the overview names it beside the open questions. |
| 2026-10-23 | 📌 | T18: the overview's Mermaid diagram rendered with `npx @mermaid-js/mermaid-cli` (worker-driven). An edge per subsystem and mechanism was unreadable, so that detail is a table under a simpler diagram. |
| 2026-10-23 | 📌 | T17: verification's combined `big-changes` covers 47 779 lines, but by its split 9 521 carry as-is or with small changes, 4 556 need big changes, 33 702 are superfrog-only. T18's totals may want the split, not the combined rating. |
| 2026-10-23 | 📌 | T17: check 4 is not address-free. `FrameState::read` reads 13 `LAYOUT` variables by original address and `SceneKind::classify` (`sf-core/src/scene.rs`) names screens by copper list address, so plays-alone also needs an adapter in a native rewrite. |
| 2026-10-23 | 📌 | T16: `sf-app` reaches past `Game` by original address twice: a script's `exit` pokes `level_state` (h0+$a06) and Start Level logs six `LAYOUT` variables via `Game::peek`. A native core needs named replacements; `memory-model` does not list them. |
| 2026-10-23 | 📌 | T14 review: the doc comment of `engine/crates/sf-core/src/bonus/mod.rs` says the enemies of markers 1–6; `enemies.rs` registers markers 1–15 and 55. Stale comment, left alone (no code changes in this plan). |
| 2026-10-23 | 📌 | T14: `bonus/death.rs` `become_reward` also serves worlds 1–6 (spud kills, via `enemies/common.rs`), and shooter mode lives mostly outside `bonus/` (`level/camera.rs`, `level/scroll.rs`, `frog/physics.rs`, `items/powerups.rs`). T18 may weigh moving `become_reward` to enemies-and-bosses. |
| 2026-10-23 | 📌 | T14: no check reaches the slot's free spin, a gamble lost above the bust, a 160 000+ win at the start, the jackpot after a level 5, or the level's sound after the slot returns; normal-play lamp cuts are unchecked. |
| 2026-10-23 | 📌 | T14: the slot repeats code: `rounds::rng` is `LevelCtx::rng_range` on `State`, and `slot::draw_lamps` (screen blank) is a second lamp drawer beside `draw::Draw`'s `Part::Lamps`. A rewrite can merge each. |
| 2026-10-23 | 📌 | T13: the darts' wait/fly and the swing's updates are ported twice (`w3::dart`, `w4::dart`; `w3::swing`, `w4::mace`). The dispatch keeps the last registration, W6's (W3's code), so W4's copies never run. Left alone. |
| 2026-10-23 | 📌 | T13: `standard_dispatch` (`level/mod.rs`) calls `enemies::register_all` twice, around `bonus::register_all`, uncommented; harmless while registrations replace. W1's `other_end_free` repeats `common::partner_right_free`; `witch.rs` repeats `follow_and_despawn`. |
| 2026-10-23 | 📌 | T13: unchecked: the swing's `$64` write with all 20 slots full, and that `w1_spout_turn` stays a `LAYOUT` variable (`w1_hover_phase` is asserted as W4's `SWOOPER_PHASE`). Enemy cycle costs are flat `Cost::Routine` but for the hover and watcher. |
| 2026-10-23 | 📌 | T15: `sf-assets/src/sound.rs` parses hunk 0 a second time (`game_hunk0`, `first_hunk`) beside `parse_exe`, and carries original addresses (`H0`, `H3`, `BANK_ADDR`). No check compares `SoundData::from_game` (the app's path) with `from_disks` (the tests'). |
| 2026-10-23 | 📌 | T15: `sf-assets/src/lib.rs`'s module comment says sound is carried undecoded until T11; `sound.rs` decodes it. Code comment, left alone. |
| 2026-10-23 | 🐞 | T11 review: docs/04 "Palettes" says a level's `PALA..PALD` are empty; `d1_c21_level_53588.bin`'s `PALA` holds colours at tag+$48, which `palette_from_map` reads (docs/08 agrees). docs/04 is stale; left alone. |
| 2026-10-23 | 📌 | T11: `LevelCtx` in `level/mod.rs` holds about 800 lines of beam machinery (frame ends mid-pass, `sfx_play`'s raster wait, copper interrupt masking), counted as level's `amiga` share. It is cpu-time-and-beam's mechanism; T18 may weigh a move. |
| 2026-10-23 | 📌 | T11: `panel_bitmap`, the display buffers and the scroller's blit queue are presentation pieces (`layout::presentation_piece`), left out of check 1. The panel has no routine test; only the picture check holds it. |
| 2026-10-23 | 📌 | T11: docs/07 and the project `CLAUDE.md` state collision as `flag & 0b11000 == 0b11000`; the engine's `collision.rs` uses a marker ladder instead (`$3a`+ ground, `$3e`/`$3f` landing, `$3d` step-up and walls). Docs left alone. |
| 2026-10-23 | 📌 | T12 review: `frog_jump_float`'s doc comment (`frog/anim.rs`) says the held jump "takes one more off the frog's rising speed"; it subtracts 1 from negative `vy`, speeding the rise. Chapter corrected, comment left. |
| 2026-10-23 | 📌 | T12: the frog's physics has no fixed point (task doc said so): whole pixels, sub-pixel motion by frame counters, 68000 `n_clear` sign tests. Gravity, `bounce` and the probes are `frog_tiles` in `level/collision.rs` (level); T18 may weigh moving them. |
| 2026-10-23 | 📌 | T12: `frog/physics.rs`'s module comment says it holds the camera that follows the frog; that is `camera_follow` in `level/camera.rs`. Code comment, left alone. |
| 2026-10-23 | 📌 | T10: `front/story/sim.rs` and `cpu.rs` serve more than the story: `slot/draw.rs` runs its draws on `sim::Sim`, and `front/fade.rs` times level fades with `vbl::fade_calls` and `cpu::Cpu`. T18 may move them to `cpu-time-and-beam` or `display-hardware`. |
| 2026-10-23 | 📌 | T10: `CpuWork` lives in `front/ending.rs` (owned by story-and-ending) but paces `front/title.rs`, `front/chain.rs` and `slot/rounds.rs` too; it is the front end's cycle charging. T18 may move it. |
| 2026-10-23 | 📌 | T10: the app never starts the story: `sf-app` only uses `Start::PowerOn` and `Start::Level`. `Start::Disk3` is reached from `sf-conform` (`power-on-disk3`) and one `sf-e2e` view test. The chapter states it; whether it is intended is not recorded. |
| 2026-10-23 | 🐞 | T09 review sweep: `copied-in-data.md` row "World files at h1 and h3" says `hero_frames` is compared by every level case; it is a `presentation` variable, left out of the faithful state check. Left for T18. |
| 2026-10-23 | 📌 | T09: `Routine`, `Cx` and `Front::step` in `front/mod.rs` (owned by front-end) also run game-flow's `chain.rs`, the slot machine and the level's exit and tally. For T18: the executor may deserve its own chapter or a mention in game-flow. |
| 2026-10-23 | 📌 | T09: the attract demo is proven only with the original's lag frames fed (`Pacing::Late`, check 4 included); unpaced, as in normal play, no check runs it. Aborting the credits reaches an unported title path; no case aborts them. |
| 2026-10-23 | 📌 | T08 review: `rng.rs`'s module comment says `rng_next` forces bits 0, 4 and 5 of the low byte; the code (`& 0x0E \| 0x20`) also clears bits 6 and 7. Chapter corrected, comment left. |
| 2026-10-23 | 📌 | T08: `FrameOut::disk_prompt` has one reader, `disk_prompts_are_raised_as_the_game_asks_and_need_no_answer` in `sf-conform/tests/flow.rs`; `sf-app` ignores it. Conformance compares `wanted_disk` (`Milestone::DiskAsked`) instead. The disk check itself is `Cx::read_entry` in `front/mod.rs`, owned by front-end. |
| 2026-10-23 | 📌 | T08: game-flow's dominant kind is `shell` (45): `recording.rs`, `savestate.rs`, `start_level`/`type_code` and tests. `chain.rs`'s frog walk (`walk_step`) is game logic a frog chapter might own; left in game-flow for T18. |
| 2026-10-23 | 📌 | T07: the app's mixer and `channels::engine_levels` never call `Paula::set_frame_lines` (`Pacer::set_frame_lines` only in its test), so every frame mixes as 313 lines though the game keeps stretches of short frames. Unmeasured whether audible. |
| 2026-10-23 | 📌 | T07: `sf-paula`'s ADKCON attach modes and audio interrupts are unchecked against vAmiga; Superfrog's driver writes no ADKCON. Left alone. |
| 2026-10-23 | 📌 | T06: the blitter is modelled three times (`Blit::run` in `front/blit.rs`, ascending-only `blit` in `level/objects.rs`, `blit` in `level/scroll.rs`), and `sprite_attach` in `level/bobs.rs` repeats `front/sprite.rs` `attach` (h0+$1b208). A rewrite can merge each. |
| 2026-10-23 | 📌 | T06: `DisplayData::new` (`render/mod.rs`) builds every subsystem's disk data (`rom`, `sound`, `front`, `story`, `levels`), so those lines count as display-hardware; T18 may weigh a move. `render/panel.rs` fetches every bitplane line, not only the panel's. |
| 2026-10-23 | 📌 | T04: `Pacing` and `PassClock` (`engine/crates/sf-core/src/game.rs`, owned by game-flow) are the beam mechanism's interface; `cpu-time-and-beam` describes them without owning them. Four separate beam models exist: level `Clock`, front `Cx`, `slot/draw.rs`, story `sim.rs`. For T18. |
| 2026-10-23 | 📌 | T04: T213 is ✅ in `plans/rust-engine/PROGRESS.md`, yet the project `CLAUDE.md` and the T04 task doc call it open. The chapter states it as ✅ and names the story `sfx_play` remainder. |
| 2026-10-23 | 📌 | T02 review: `layout.rs`'s `Level1Vector` doc names only `enemies::w3::swing` as writing `$64`; `enemies::w2::mace` does the same through a null link slot. Code comment, left alone. |
| 2026-10-23 | 📌 | T02: `mem.rs` is half copied-in data (`loaded_hunk0`, `boot_tables`, `tile_heights`, `Rom::new`) but owned by `memory-model`, which counts it as `data` 15. No check names the null-pointer reads at 0/4 or the `$64` write; T18 may weigh both. |
| 2026-10-23 | 📌 | T05: the keyboard poll is ported twice (`kbd_poll` in `level/clock.rs` and `front/vbl.rs`, state in `State::kbd_flag`/`kbd_acked` and `front::Keyboard`); a rewrite can merge them. No check moves the mouse (`JOY0DAT` always 0). |
| 2026-10-23 | 📌 | T03: `sf-core` never reads `sf_assets::Tables`. Templates, the marker jump table and the slot tables are decoded there, but the engine reads them by address from relocated hunk 0; only `catalogue` and tests use `Tables`. |
| 2026-10-23 | 📌 | T03: hunk 0 is held four times (`ExeImage`, `Rom::h0`, `FrontData::h0`, `SoundData`) by three relocators (`mem.rs` `loaded`, `front/data.rs` `relocate`, `story/data.rs` `data_hunk`); `sound.rs` repeats `H0`, `H3` and its own `file_table` reader. |
| 2026-10-23 | 📌 | T03: the engine reads original code as data: `Chain::run` steps through `top_loop`, `password::table` reads handlers' `move.l` immediates, `table_entries` scans for `lea`. A native rewrite replaces these before dropping hunk 0. |
| 2026-10-23 | 📌 | `tools/architecture.py totals` counts 111k code lines today (verification 47.8k, app 10.4k), against DESIGN §1's ~75k on `pir/rust-engine`. The T01 ownership table covered every tracked file; nothing was reassigned. |
| 2026-10-22 | 📌 | The stored `.pir/settings.json` test line runs only the Python suites (34 tests, ~36 s on a fresh worktree); the Rust suites are not in it. This plan changes no Rust, so the drift check joins the Python suites. |
| 2026-10-22 | 📌 | The Amiga imitation is woven through `sf-core`, not layered: byte-exact `State` at original addresses, `spend` cycle charging, cycle-run copper, and the story's own bus simulator (`front/story/sim.rs`, `cpu.rs`, `mem.rs`). Hence mechanism chapters. |
