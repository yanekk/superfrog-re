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
