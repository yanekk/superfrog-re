# T09 — start-level-worlds

**Phase:** 3 · **Depends on:** T08 · **Weight:** medium

## Goal

The owner asked for Game ▸ Start Level… to offer worlds 1 to 6 and a "Cheat on" box, started by setting
the world and cheat directly as `load_world` is entered, and `--level N` to take 1 to 6 the same way,
with `--cheat` for the cheat (owner, plan review).
Wire both into fd-app on the translated program, with recordings from them in T08's format.

## Design sections this implements

DESIGN §2.5, §2.6.

## Files

- `games/forest-dumb/engine/crates/fd-app/src/recorder.rs` (`spec`, `chosen`), `src/args.rs`
  (`--level`, `--cheat`), `src/translated.rs` (`to_world`, replacing `to_level_1`'s body), `src/main.rs` and
  `src/menu.rs` where the start is made, `src/script.rs` (`form cheat on|off`), `README.md`.
- `games/forest-dumb/engine/crates/fd-e2e/tests/start_level_worlds.rs` (new), labelled in
  `games/forest-dumb/smoke.toml`.
- `games/forest-dumb/CLAUDE.md` "Playing it": `--level N` and `--cheat`.

## Interface

```rust
// recorder.rs
pub fn spec() -> StartLevelSpec;            // translated: Level dropdown W1-1..W6-1, "Cheat on" (off),
                                            // "Enable recorder?" (on); --engine hand: as today
pub fn chosen(v: &Values) -> (LevelId, bool /*cheat*/, bool /*record*/);
// args.rs: --level 1..6; anything else exits 2 naming the range; --engine hand with --level 2..6
// exits 2 naming that the hand engine has level 1 only; --cheat turns the cheat on for --level N,
// and without --level or with --engine hand exits 2 naming why
// translated.rs
pub fn to_world(&mut self, level: LevelId, cheat: bool) -> Result<u64, String>;
```

Script lines: `form level W3-1`, `form cheat on|off`, `form record on|off`, `form start`.

## Tests

- [ ] `spec`/`chosen` unit tests: six levels, the cheat box off by default, hand engine unchanged.
- [ ] `--level` parsing 1 to 6, 0, 7 and a word refused; `--engine hand --level 2` refused;
      `--level 3 --cheat` gives world 3 with the cheat, `--cheat` alone and `--engine hand --level 1
      --cheat` refused.
- [ ] `to_world` headless for each world reaches its first frame of play with `world_index` set; with
      cheat on, `cheat_flag` set.

## Done when

- [ ] Start Level… and `--level N [--cheat]` start every world 1 to 6 on the translated program, cheat
      as chosen.
- [ ] The end to end tests below pass; each file is labelled, the short ones smoke.

## End to end (the worker drives this)

- suite: fd-e2e, script layer, and the OS layer for the window (`FD_E2E_OS=1`) · sizes: default scale,
  400×300, 1200×900
- [ ] `menu start-level`, `form level W3-1`, `form cheat on`, `form start` → world 3's play, the app's
      state equal to a headless `to_world(W3_1, true)` every step.
- [ ] `--level 4` → world 4's first frame equal to headless; `--level 4 --cheat` the same with
      `cheat_flag` set, equal to headless `to_world(W4_1, true)`.
- [ ] Start Level… W1-1 → level 1 on the hand engine through the seam, as `--level 1` today.
- [ ] Recorder on from a world 5 start, a few hundred steps, save → `fd-conform bug replay` reaches
      the app's last state.
- [ ] OS layer: the window shows six levels and the cheat box, unclipped at each size; Cancel leaves
      play as it was.
