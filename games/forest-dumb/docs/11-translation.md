# 11 — Translation

Literal translation of the original's routines by Muse, judged on the routine oracle (plan
`forest-dumb-level-1`, DESIGN §4.3). This note holds the service (T33), the generator service (T61)
and the inventory (T34); the generators (T35 to T39, T53, T54, T56) and the batch's table (T40, at
the end) add their sections.

## The service: `tools/translate.py`

How a builder gets a literal Rust port of one routine of the main program without reading its
listing (plans/forest-dumb-level-1 T33, DESIGN §4.3). Measured background: `spikes/muse-spark/README.md`.

### Calling it

From the repo root, the routine quoted (`$5` is a shell parameter):

```
python3 -I games/forest-dumb/tools/translate.py 'h0+$5462' --gen char_sprite_position --max-usd 0.10
```

- The routine is `hN+$off` or its name in `re/symbols.toml`.
- `--gen` names a generator registered in `engine/crates/fd-conform/src/gen/` (T32, T35 to T39). No
  generator, no translation: write or ask for one first.
- `--tries` (3), `--max-usd` (0.10, DESIGN §5.2) and `--out` (`games/forest-dumb/build/translate`).
- Needs `OPENROUTER_API_KEY` in the environment or as an `export` line in `~/.zshrc`, the extracted
  program (`cargo run -p fd-tools -- extract` in `engine/`) and its listing (`tools/re_listing.py`).

It prints one line and nothing else on stdout; read that and the port, not the files beside them:

```
{"routine":"h0+$5462","result":"pass","tries":1,"tokens_in":1629,"tokens_out":7897,"usd":0.001742,"paths":4,"port":"games/forest-dumb/build/translate/h0-5462/port.rs"}
```

| `result` | Meaning | Exit | What the builder does |
|---|---|---|---|
| `pass` | `port` matched the original on all 2 000 states | 0 | turns it into engine code on typed state, proves that on the rig |
| `fail` | `--tries` answers were wrong, or `"stopped":"max-usd"` came first | 1 | translates it by hand |
| `too-big` | over 180 instructions with its callees, less its `split_at` callees; no call made | 1 | asks for `split_at` in `re/translate.toml` (T34), or translates by hand |
| `no-oracle` | the original returned on none of the generator's states, or started a line or fill blit on one; no call made | 1 | fixes the generator, or translates by hand |
| `no-key` | no key; stderr says where it looked | 1 | asks the owner for the key |

Exit 2 is a usage error with the reason on stderr: an unknown routine or generator, a missing
listing or disks, or a split callee with no passing port yet.

`paths` is how many distinct instruction counts the original ran over the states: a proxy for the
branches the generator reaches. A `pass` with few paths proves little.

### What it does

1. The listing by control flow from the routine (every branch both ways, calls followed), then the
   named labels it uses: the spike's `run.py --hw` extraction. Its size decides `too-big`. Only
   `Bcc`, `DBcc`, `bra`, `bsr`, `jsr` and `jmp` are followed; `btst`/`bset`/`bclr`/`bchg` on a label
   reach data, not code (T58).
2. `fd-judge --survey`: the original alone over the generator's states. It is `no-oracle` when it
   returns on none (a wait on DMA or an interrupt never ends in the routine oracle's hardware mode)
   or when any state starts a blit in line or fill mode (`RigReport::unmodelled_blits`): the
   oracle's blitter runs area mode only, so a port would agree with a blit that never happened.
3. Muse (`meta/muse-spark-1.3-contributor`, `max_tokens` 96 000) answers; the last ```` ```rust ````
   block is built into `fd-judge` (`FD_PORT`, `fd-conform/build.rs`) and judged against the original
   over 2 000 states: registers d0-d7 and a0-a6, every byte written off the stack, and every
   custom-chip write in order. A wrong answer gets the first failing case back; one that does not
   compile gets the compiler's errors; one with no code (cut off while reasoning) gets asked again.
   Each counts as a try. A 429, 502 or 503 is waited out and is not a try.
4. Before each call, its worst case (the conversation at three characters a token, plus 96 000
   answer tokens, at OpenRouter's listed price) must fit under `--max-usd` with what is spent.

The port is written against `Cpu { d, a, mem: Mem }` (`fd-conform/src/judge/mem.rs`), the spike's
model of memory and the custom chips. One judge build and run at a time, across processes
(`<out>/judge.lock`), since the port is compiled into the one binary.

#### A split routine

A routine T34 splits names its callees in `split_at`. Those are translated first; each must have a
passing port in its own folder. The prompt leaves their listing out and tells Muse to call
`port_hN_off(cpu)` where the original calls them, and the judge builds their ports in as modules.
The caller's `port.rs` is its own code only; its callees' are in their folders.

### What it cannot prove

- Only the generated states. A pass is equality on the generator's 2 000 states, nothing else: a
  branch no state reaches is untested. The reviewer of each generator checks branches against the
  listing (T35 to T39).
- Only what the routine oracle models. Line and fill blits are refused (`no-oracle`); a read of a
  write-only register, DMA and interrupt timing are not modelled on either side, so a port agreeing
  there proves nothing.
- Not the engine port. A literal port over raw memory is the input to a porting task, which proves
  its engine code on the rig (`fd-conform/src/routine.rs`).
- Condition codes and the stack bytes are not compared.

### Files

Under `--out`, never committed (they derive from the game's code): `calls.jsonl`, a line a call
(time, tries, tokens, reasoning, cost, outcome, the feedback's start); per routine `hN-off/` with
`prompt.md`, `answerN.md`, `tryN.rs` (the code), `judgedN.rs` (what was built, callees included),
`port.rs` (the passing answer only) and `result.json` (the result line, its generator and `split_at`).
A new run of a routine clears its folder first.

### Record

- 2026-11-05, T33: `h0+$5462` with `char_sprite_position`: pass on try 1, 1 629 tokens in, 7 897 out
  (7 131 reasoning), $0.0017, 67 s, 4 paths.
- 2026-11-05, T35: `h0+$4274` with `h0_4274`: pass on try 1, 1 482 tokens in, 4 299 out, $0.0010,
  6 paths.

## The generator service: `tools/generate.py` (T61)

Muse writes a routine's generator and the machine judges it on the original, so a builder reviews a
generator instead of writing one (plans/forest-dumb-level-1 T61, DESIGN §4.3). The builder sees the
result line and the generator, never the listing or the retries.

### Calling it

From the repo root, the routine quoted; `--name` is what the generator is registered as (the
routine's name in `re/translate.toml`):

```
python3 -I games/forest-dumb/tools/generate.py 'h0+$4e9a' --name h0_4e9a --max-usd 0.10
```

`--tries` (3), `--max-usd` (0.10, DESIGN §5.2) and `--out` (`games/forest-dumb/build/generate`). It
needs what `translate.py` needs: the key, the extracted program and its listing. One line on stdout:

```
{"routine":"h0+$4e9a","result":"covered","tries":1,"usd":0.0021,"branches":5,"uncovered":[],"returns":2000,"base":false,"gen":"games/forest-dumb/build/generate/h0_4e9a.rs"}
```

| `result` | Meaning | Exit | What the builder does |
|---|---|---|---|
| `covered` | a valid generator took every branch both ways | 0 | reads `gen`, commits it into `fd-conform/src/gen/` |
| `partial` | the best valid try left the ways in `uncovered` untaken | 1 | as `covered`; the reviewer judges each uncovered way and its `reason` |
| `fail` | no try was valid (compiles, returns, pokes only what is read), or `"stopped":"max-usd"` came first | 1 | writes the generator by hand |
| `no-oracle` | on every answer that ran, the original returned on no state or started a line or fill blit | 1 | the routine is out of the routine oracle's reach; by hand |
| `no-key` | no key; stderr says where it looked | 1 | asks the owner for the key |

`uncovered` lists `{"at":"hN+$off","never":"taken|not-taken|either"}`, with `"reason"` where Muse
said that way cannot be reached. `either` is a branch no state reached (one in the listing the rig
never counted included). `base` is true when the generator lays a base, which the reviewer reads: a
base is shown, not judged. Exit 2 is a usage error, as `translate.py`'s.

### What it does

1. The prompt: the listing by control flow with callees (`translate.flow_text`, no `split_at`: the
   original runs its callees), and up to three committed modules from `fd-conform/src/gen/` as
   examples. They are the modules registering the most routines of its `subsystem` in
   `re/translate.toml`, then the smallest. A module that registers this routine's name is never
   shown, since it holds the answer.
2. Each answer, a module shaped as the committed ones, is built into `fd-judge` (`FD_GEN`,
   `fd-conform/build.rs`; `generators::candidate`, ahead of every committed module) and judged by
   `fd-judge --cover` over its 2 000 states. A try is valid when it compiles and lists `--name`, the
   original returns on at least 95% of the states with no hang and no line or fill blit, and every
   byte a state pokes, or makes a hardware read return, is one the original read on some state
   (`Coverage::reads`) or on the rig's stack. An invalid try gets the reasons back.
3. A valid try is measured by `Rig::coverage`. A way of a `Bcc`/`DBcc` no state took goes back with
   the branch's listing line and the three before it, up to `--tries`. Muse may answer
   `UNREACHABLE hN+$off <way>: <reason>`; such a way is passed through and not asked again.
4. The result is the valid try with the fewest uncovered ways, the later on a tie, copied to
   `<out>/<name>.rs`. Spend, waits and `calls.jsonl` (`"service":"generate"`) are `translate.py`'s.

The candidate's judge build takes the same lock as `translate.py`'s default (`build/translate/judge.lock`),
since both build into the one `fd-judge`.

### What it cannot prove

- That the states are ones the game can be in. The judge checks returns and pokes; whether a
  state is plausible (and whether a `base` is right) is the reviewer's reading.
- That a claimed unreachable way is unreachable: the reason is Muse's, for the reviewer.
- Equality of anything: a generator only makes states. A port is proven on them by `translate.py`
  or the rig.
- Branch coverage is per branch, not per path: two branches each taken both ways may never be
  taken together.

### Files

Under `--out`, never committed: `calls.jsonl`, `<name>.rs` (the result), and per generator `<name>/`
with `prompt.md`, `answerN.md`, `tryN.rs` and `result.json`. A new run of a name clears its folder.

### Record

2026-11-05, T61, one live run per subsystem on a routine with a hand-written generator (`--max-usd
0.10`, a §5.3 `worker` action). Coverage is `fd-judge --cover` on both: branch ways untaken, and
poked bytes the original reads on no state ("outside").

| Routine | Subsystem | Muse: result, tries | Muse $ | Branches | Hand-written (module) |
|---|---|---|---|---|---|
| `h0_4e9a` | camera | covered, 2 (try 1 did not compile) | 0.0041 | 5 | covered, outside 0 (`camera.rs`) |
| `play_keys` | rest | covered, 1 | 0.0018 | 8 | covered, outside 0 (`rest.rs`) |
| `char_sprite_position` | drawing | covered, 1 | 0.0032 | 3 | covered, outside 0 (`drawing.rs`) |
| `h0_3566` | objects | covered, 1 | 0.0025 | 1 | covered, outside 5 102 (`objects_move.rs`) |
| `h0_4b0e` | character | covered, 1 | 0.0022 | 14 | `h0+$4cb8`, `$4d00` never taken (`character.rs`) |

Every Muse generator returned on all 2 000 states with no byte outside. Total $0.0139, 10 to 98 s an
answer, 9 000 to 12 600 tokens in. `h0_3566`'s hand-written `probe` lays all four collision bytes a
probe could read, so it fails this service's rule; `h0_4b0e`'s misses two branches in its callees,
which T35's table (its own flow, 3 branches) did not list. Whether Muse's states are plausible was not
judged here: that is the reviewer's reading.

## Generators: `character` (T35)

`engine/crates/fd-conform/src/gen/character.rs`, one generator for each of the 14 `gen_task = "T35"`
routines, named as in `re/translate.toml`. Its `COVER` table holds, per routine, what it reads (callees
included, since the original runs them) and the conditional branches of its own flow, read off the
listing. `tests/gen_character.rs` checks, on the program from disk 1:

- 2 000 states each, no hang, fault or unmodelled blit;
- every `Bcc`/`DBcc` in `COVER` goes both ways on some state, or is named unreachable. With no
  instruction trace in the routine oracle, the test detours one branch at a time through a stub
  that traps on one way and runs the other as the original would;
- each generator pokes only bytes its routine reads, and its base only `collision_map`.

`char_update` and `char_probe` lay `collision_map` as blocks of clear, solid and noise. The others
poke state variables, biased to the tables' `$ff` ends and to the signed compares; a quarter of
`char_step`'s and `h0_3ea8`'s states are set up for the slide, which random draws reach about once
in 2 000.

| Generator | Paths | Branches | Unreachable |
|---|---|---|---|
| `char_update` | 223 | 1 (`dbra`) | |
| `char_step` | 74 | 20 | |
| `h0_3ea8` | 28 | 16 | `h0+$3f42`, `$401a` taken: the raster wait's `beq`; VHPOSR steps on every read in the oracle |
| `h0_410c` | 9 | 1 | |
| `h0_416c` | 2 | 1 | |
| `h0_4274` | 6 | 5 | |
| `h0_4696` | 4 | 4 | |
| `h0_4718` | 24 | 9 | |
| `h0_47e2` | 3 | 2 | |
| `h0_48be` | 24 | 9 | |
| `h0_497c` | 3 | 2 | |
| `h0_4b0e` | 8 | 3 | |
| `char_probe` | 1 | 0 | no branches (`sne` only) |
| `char_probe_flags` | 15 | 15 | |

`char_probe` is `by_hand`; its generator is for the rig. `h0+$406c` in `h0_3ea8` loads `h0+$40ac`
over the `h0+$40cb` just chosen, so the right slide always reads the left table: a reading of the
original, kept by a literal port.

## Inventory: `game_frame`'s call tree and the edge redraws

Measured 2026-11-05 (T34) by `tools/call_tree.py` from the program `fd-tools extract` writes; the result,
one entry a routine, is [`re/translate.toml`](../re/translate.toml). Rerun with
`venv/bin/python games/forest-dumb/tools/call_tree.py`, which keeps each routine's classification by
address; `--check` compares the toml with the program, `--summary` prints the tables below.

- **The roots are `game_frame` and `redraw_edge_a..d`.** The edge redraws run from `main_loop`, not
  `game_frame`, but write `collision_map`, which is play state (T00), and T09's goal named them; the owner
  added them as roots on 2026-11-05, with T55 and T56 to take them.
- **87 routines, 4 041 instructions of their own.** `game_frame`'s tree is 73 of them and 3 105 (3 026
  with callees counted once; some routines share tails, so own flows overlap by 79); the edge redraws add
  14 and 936, sharing nothing with it. The spike's "about 8 300" counted address ranges, data included,
  and did not follow calls written as names.
- **No call or jump the scan could not follow.** No `jsr (an)`, no computed `jmp`, no Kickstart call
  is reached from the roots. The `[[unfollowed]]` table is empty.
- **The listing's linear sweep is not usable for this.** After data inside the code it decodes out of
  step: 7 of the tree's call targets (`h0+$2c7e`, `h0+$3698`, `h0+$5820`, `h0+$94ba` among them) fall
  inside an instruction there. `call_tree.py` decodes from each address the flow reaches instead.
  Since T40 `re_listing.py` passes every instruction that decoding reaches to the lister as a place
  to resync (`hunk_listing.listing(entries=)`), so the listing is in step over the whole tree and
  `translate.py`'s extraction counts each routine's `unit` exactly.
- Sizes agree with T00's for every routine T00 sized (`char_probe` 199, `char_step` 107, `h0+$3ea8` 93,
  `char_hit_check` 131, `h0+$26ec` 153, `camera_update` 48, `char_sprite_copy` 73).

Each entry has `own`, `instructions` (with callees) and `unit`: what one translation call sees, its
callees less `split_at` and what only they reach. `split_at` is chosen greedily, the callee whose removal
drops the unit most first, until the unit is 180 or less. Sixteen routines are split.

**Four routines are `by_hand`:** their own flow is over 180 with no callee left to split at, so T33
answers `too-big` and the porting task's builder translates them.

| Routine | Own | Port task | What it reads as |
|---|---|---|---|
| `char_probe` | 199 | T08 | the character's collision probe on `collision_map` |
| `h0_5e44` | 200 | T52 | tile copies restoring the screen behind creatures |
| `h0_62b6` | 224 | T25 | animated map cells, codes `$20`-`$28`; also writes `collision_map` |
| `redraw_edge_c` | 181 | T55 | one of the four edge redraws |

`h0_62b6` calls `h0_3ab4` (a blitter wait), which is split off; that changes nothing, since 224 is its
own flow.

### Subsystems and tasks

Classified from what each routine reads and writes, and from T00's note and the spike: a **reading**,
noted per entry where it is not measured. The subsystem picks the generator batch, by the kind of state
a generator has to build (blitter and screen for `drawing`). The port task is the task that owns the
behaviour, so drawing of collectibles, creatures and animated cells goes to T22, T52 and T25, which list
`render/objects.rs` in their files, not to T10.

- `sound`: `h0_2648` and `h0_3adc`, which only start a sample on audio channel 2. Out of scope, no
  tasks. Four routines also start a sound inline: `h0_21c0` and `char_hit_check` on channel 1, `h0_3ea8` on
  2, `h0_26ec` on 3. How a port treats those writes, with sound out of scope (§8), is its porting task's call.
- `rest`: `game_frame` itself goes to T08, the first task to edit `Game::step`; each later porting task
  adds its calls. `countdown_step`, `play_keys`, `esc_key`, `lives_minus_one` go to T23, `help_key` to T26.
- `char_hit_check` and its box test `h0_3dd8` are `objects` and T24: they read the five creature slots.
- `h0_24ce` (map codes `$10`-`$13` under the character) and `h0_21c0` (blits a tile at the cell it
  marked) read as a mechanic: T25. T21 confirms what those codes are.

Object routines reached only through dispatch tables are T21's and T49's, not this scan's (§4.3).
The edge redraws and their 10 helpers are `camera`, generators T56, port T55; T09 keeps the edge flags
that trigger them (`h0_4e9a` to `h0_5346`). Three helpers read as other subsystems' (`h0_681c`, `h0_6852`
touch the object slots; `h0_6882` adds to `h0_62b6`'s animated-cell list), which T55 checks. The rest of
`main_loop`, about 110 routines, is outside this plan's translation.

### Each task's share

Own instructions summed per task, `sound` left out; DESIGN §4.3's limit is 1 500 instructions or 15
routines. The first count put T08 at 21 routines, T24 at 17, T35 at 20 and T38 at 21. The owner added
T51 to T54 on 2026-11-05 to take the excess, and T55, T56 for the edge redraws, so every task now fits.

| Porting task | Routines | Own instructions | By hand |
|---|---|---|---|
| T08 | 15 | 741 | `char_probe` |
| T09 | 10 | 191 | |
| T10 | 7 | 237 | |
| T22 | 8 | 365 | |
| T23 | 4 | 55 | |
| T24 | 13 | 503 | |
| T25 | 3 | 446 | `h0_62b6` |
| T26 | 1 | 10 | |
| T51 | 6 | 112 | |
| T52 | 4 | 421 | `h0_5e44` |
| T55 | 14 | 936 | `redraw_edge_c` |

| Generator task | Routines | Own instructions | By hand |
|---|---|---|---|
| T35 | 14 | 705 | `char_probe` |
| T36 | 10 | 191 | |
| T37 | 14 | 1 070 | `h0_5e44`, `h0_62b6` |
| T38 | 10 | 548 | |
| T39 | 6 | 101 | |
| T53 | 6 | 112 | |
| T54 | 11 | 354 | |
| T56 | 14 | 936 | `redraw_edge_c` |

A `by_hand` routine still needs its generator: its port is proven on the rig like any other.

## Generators: drawing (T37)

`engine/crates/fd-conform/src/gen/drawing.rs`, one generator per `drawing` entry, named as in
`re/translate.toml`; `READS` there lists what each routine reads, the only places its states write.
Proven by `engine/crates/fd-conform/tests/gen_drawing.rs` on the program from disk 1 (2026-11-05).
T37 took the spike's `char_sprite_position` and `h0+$9754` over; the latter is now `h0_9754`.

**Branches measured by flipping.** The routine oracle has no instruction trace, so the test runs each
state three times: as the program is, with the branch made `bra`, and with it made never taken (a `nop`,
or `bra.b` over a `.w`'s displacement, so the instruction count holds). Equal to the first flip, the
state took it on every pass; to the second, on none; to neither, both ways (a loop); to both, it never
got there. `the_flips_see_a_branch_that_goes_one_way` checks the method on `h0_9754`.

Every conditional branch of each unit goes both ways on some state except these, which no state can
take on the oracle:

- **Blitter waits** (`btst #6,$dff002` then `bne` back): the oracle's blitter finishes each blit as it
  starts, so busy never reads set; a state setting it would wait for ever. In `h0_21c0` (5 with
  `h0_3ab4`), `h0_29ca`, `h0_3ab4`, `h0_5820` (3), `h0_5e44` (9), `h0_62b6`.
- **`h0+$23aa`** in `h0_21c0`, a raster wait: VHPOSR reads one more on every read, so two reads never match.

The `dbra`s count fixed loops (sprites, bitplanes, 32 colours, three lines) and are left out.

| Routine | States returned of 2 000 | Paths | Faults | Branches both ways | Named, not taken |
|---|---|---|---|---|---|
| `h0_21c0` | 2 000 | 6 | 0 | 5 | 6 |
| `h0_29ca` | 2 000 | 1 | 0 | 0 | 1 |
| `h0_2c7e` | 2 000 | 366 | 0 | 2 | 0 |
| `h0_2d4c` | 1 945 | 177 | 55 | 2 | 0 |
| `h0_3ab4` | 2 000 | 1 | 0 | 0 | 1 |
| `char_sprite_position` | 2 000 | 4 | 0 | 3 | 0 |
| `h0_5820` | 1 968 | 9 | 32 | 9 | 3 |
| `h0_5e44` | 2 000 | 20 | 0 | 8 | 9 |
| `h0_62b6` | 2 000 | 636 | 0 | 16 | 1 |
| `h0_94ba` | 2 000 | 162 | 0 | 8 | 0 |
| `h0_94fe` | 2 000 | 162 | 0 | 6 | 0 |
| `h0_9550` | 2 000 | 12 | 0 | 6 | 0 |
| `h0_9754` | 2 000 | 2 | 0 | 1 | 0 |
| `char_sprite_copy` | 2 000 | 4 | 0 | 3 | 0 |

No state hangs or starts a line or fill blit. The faults come from the few creatures given a kind of 8
or more (or, for `h0_5820`, one state in 50 a slot past the five), whose frame table pointer is read from
code past h0+$3a18's eight entries; the original takes an exception there. They are counted, never compared. `cargo test -p fd-conform --test gen_drawing -- --nocapture`
prints each branch's counts.

**Through the service:** 2026-11-05, `translate.py 'h0+$9754' --gen h0_9754`: pass on try 1, 2 177
tokens in, 8 052 out, $0.0018, 81 s, 2 paths.

## Generators: `camera` (T36)

`fd-conform/src/gen/camera.rs`, one generator per T36 entry of `re/translate.toml`, named as there.
The routines read only `char_x`/`char_y`, `camera_x`/`camera_y`, the edge flags `h0+$51f8..$51fb` and
the four latch words `h0+$5196..$519c`; no hardware, so no base. States lean to where the branches
turn: a camera on a 16-pixel boundary, at its clamp (`$17cf`, `$53f`, 0), negative (with the
`$8000..$853e` band where `cmpi.w #$53f` overflows and `h0_513c` sets `$53f`, not 0), and a character
either side of each dead-zone edge (`$94`, `$78`, `$5a`, `$46`).

`fd-conform/tests/gen_camera.rs` proves it. The rig has no trace, so a witness (a literal model of each
routine counting its branches) must match the original on all 2 000 states; then every conditional
branch in the routine's flow listing, callees included, is taken both ways. No branch is unreachable.
Measured 2026-11-05, 2 000 states each, no hang or fault:

| Generator | `RigReport::paths` | Branches | Fewest states one way |
|---|---|---|---|
| `camera_update` | 385 | 28 | 41 |
| `h0_4e9a` | 65 | 5 | 207 |
| `h0_4ea6` | 64 | 5 | 208 |
| `h0_4eb4` | 64 | 5 | 210 |
| `h0_4ec2` | 65 | 5 | 239 |
| `h0_513c` | 3 | 4 | 151 |
| `h0_51fc` | 4 | 3 | 25 |
| `h0_526a` | 4 | 3 | 32 |
| `h0_52d8` | 4 | 3 | 35 |
| `h0_5346` | 4 | 3 | 27 |

`h0_513c`'s 3 paths cover its 4 branches: a skipped `move` and a `clr` cost one instruction alike.

- 2026-11-05, T36: `camera_update` with `camera_update`: pass on try 1, 4 837 tokens in, 16 485 out,
  $0.0038, 115 s, 385 paths.

## Generators: `objects` (T38)

`fd-conform/src/gen/objects.rs`, one generator per T38 entry of `re/translate.toml`, named as there.
They read the byte map (`h0+$cb38`, `$60` cells a column), `char_x`/`char_y`, `camera_x`/`camera_y`,
the five slots (flags `h0+$67da`, eight bytes each at `h0+$67ec`: x, y, kind), the BCD counters
(score `h0+$2a66`, `score_add`, `countdown`, `items_left`, `lives`), `hits_taken`, `cheat_flag`, the
gates `h0+$4ed0`, `h0+$4b62`, `h0+$9b04`, and CLXDAT as a hardware read. The item blit (`h0_29ca`) reads a
tile word at `h0+$16318` and a table long at `h0+$3b988`; `char_object_cells` and `h0_26ec` lay
hunk 3 with noise as their base so the blit copies something. States lean to where branches turn:
each map code `h0_24ce` and `h0_26ec` test (N-flag ranges, so `$a0..$ff` read as below `$20`), a
slot on and either side of each camera-window edge, a box corner at `p - size`, `p - size - 1`,
`p - 1` and `p`, the countdown at `$0898`/`$0899`, `hits_taken` at 2, 3 and `$ffff`.

Counters are valid BCD on every state: the game writes only BCD there, and ABCD/SBCD on other digits
is undocumented, so a port judged on them would be judged on one emulator's choice.

`fd-conform/tests/gen_objects.rs` proves it, as `gen_camera.rs` does: a witness per routine matches
the original on all 2 000 states, and every conditional branch of the flow listing, callees
included, goes both ways, except five that can only go one way: the VHPOSR waits `h0+$265e`,
`$291c`, `$3af2`, `$3da4` (the rig's VHPOSR steps on every read, so two reads never match) and the
blitter wait `h0+$2a1c` (BLTBUSY set never clears: a hang). Measured 2026-11-05, no hang or fault:

| Generator | `RigReport::paths` | Branches | Fewest states one way |
|---|---|---|---|
| `h0_24ce` | 7 | 6 | 125 |
| `char_object_cells` | 108 | 29 | 44 |
| `h0_26ec` | 23 | 29 | 32 |
| `score_add_routine` | 1 | 0 | |
| `h0_2dd6` | 125 | 14 | 117 |
| `h0_2e0e` | 10 | 12 | 37 |
| `char_hit_check` | 105 | 30 | 32 |
| `h0_3dd8` | 4 | 4 | 235 |
| `countdown_plus_100` | 2 | 1 | 874 |
| `items_left_minus_one` | 2 | 1 | 525 |

`score_add_routine` has no branch: four ABCDs and a clear.

- 2026-11-05, T38: `h0+$2e0e` with `h0_2e0e`: pass on try 2 (try 1 differed on 9 of 2 000 states),
  9 772 tokens in, 23 238 out, $0.0056, 180 s, 10 paths.

## Generators: `objects` movement (T54)

`fd-conform/src/gen/objects_move.rs`, one generator per T54 entry of `re/translate.toml`, named as
there: the slots' movement `h0_2fae`, its per-slot body `h0_2fea`, the step-table helpers `h0_320a`,
`h0_3222`, `h0_32f8`, `h0_3322`, the `collision_map` probes `h0_3530`, `h0_3566`, `h0_35a0`,
`h0_35dc`, and the sort by y `h0_3698`. They read the five slots (flags `h0+$67da`; x, y, kind words,
a direction byte and a step counter at `h0+$67ec`), the slot index `h0+$2faa` (kind 7, `h0_2fea`
alone), `char_x` (kind 0) and `collision_map`. A probe reads `((y + dy) & $ff) * $2e + (x' >> 3)` with
x a level coordinate, so it reaches `$4dd2` bytes past `h3+$fa50`, not only the map's `$2e00`. Each
slot lays every collision byte its probes can reach after moving up to 5 pixels, clear or solid at
even odds. Kinds lean to 0..7, the counter to kind 7's `$1f`/`$20` edge, `char_x` to the slot's
`x + $10`; the sort's y words come from a few tied values in a third of the states, since its middle
two compares are `cmp.b`.

`tests/gen_objects_move.rs` checks them as `gen_rest.rs` does T39's: 2 000 states with no hang,
fault or unmodelled blit; every conditional branch of the flow, callees included, both ways through
a detour stub (first on the first 200 states, the full 2 000 only if a way is unseen); no poke outside
what the routine reads. No branch is unreachable. Measured 2026-11-05:

| Generator | `RigReport::paths` | Branches | Fewest states one way |
|---|---|---|---|
| `h0_2fae` | 211 | 21 | 307 |
| `h0_2fea` | 23 | 19 | 83 |
| `h0_320a`, `h0_3222`, `h0_32f8`, `h0_3322` | 1 each | 0 | |
| `h0_3530`, `h0_3566`, `h0_35a0`, `h0_35dc` | 2 each | 1 each | 960 |
| `h0_3698` | 15 | 5 | 1 085 |

## Generators: `rest` (T39)

`fd-conform/src/gen/rest.rs`, one generator for each of the six `gen_task = "T39"` entries, named as
there. `COVER` holds what each reads (callees included) and the conditional branches of its own flow;
`tests/gen_rest.rs` checks them as `gen_character.rs` does T35's (2 000 states, no hang, fault or
unmodelled blit; every branch both ways through a detour stub; pokes only what the routine reads).

- The five small routines read hunk 0 variables only: `last_key_byte` biased to `$41`, `$75`/`$74`,
  `$cd`/`$cc`; the flags 0, 1 or any; `countdown_tick` near `$3f`; BCD counters at 0, 1, a borrow, or
  any word in one state of eight, so a port's `sbcd` meets bad digits. `lives_minus_one` reaches
  `sbcd` with X as entered; the rig enters with `sr = $2700`, X clear.
- `game_frame` runs the whole pass, so its base is the original's chip and slow RAM at the level sync
  point (`sync::reach_level_1`, from the cache). It pokes its unit's own inputs only: the key and pause
  variables, the countdown, `char_x`/`char_y` and the camera within 24 pixels of the sync point's,
  `input_log`'s index (`$2ee0` or below), and JOY1DAT. Paused in a quarter of the states. No memory
  within a `bra.w` is known free there, so its one branch (`h0+$200a`) is shown by `illegal` over its
  target or fall-through, neither reached any other way.

Measured 2026-11-05, 2 000 states each, no hang or fault, no branch unreachable:

| Generator | `RigReport::paths` | Branches |
|---|---|---|
| `game_frame` | 661 | 1 |
| `countdown_step` | 3 | 2 |
| `play_keys` | 5 | 3 |
| `help_key` | 3 | 2 |
| `esc_key` | 4 | 3 |
| `lives_minus_one` | 2 | 1 |

- 2026-11-05, T39: `play_keys` with `play_keys`: pass on try 1, 1 716 tokens in, 4 349 out, $0.0010,
  43 s, 5 paths. `fd-judge 'h0+$1fec' --gen game_frame --survey`: 2 000 passed, 661 paths, 0.3 s.
- `translate.py 'game_frame'` exited 2 until T40: its `split_at` callee `h0+$2c7e` had no label in
  the listing (T34's lost-step sweep). The listing now resyncs there (see the inventory).

## Generators: edge redraws (T56)

`fd-conform/src/gen/edge_redraw.rs`, one generator for each of the 14 `gen_task = "T56"` entries, named
as there; `READS` lists what each routine reads when it runs, callees included. T56 took the spike's
`redraw_edge_a` and `redraw_edge_b` over; the spike's `h0+$774` stays, under that name, for `judge.rs`.

- The redraws read their position at `h0+$1e66`/`$1e68` (x, y), not `camera_x`/`camera_y`. Each draws
  23 or 16 tiles from `_MapaN-1` (`h0_774`) and as many shapes from `_MapaN-1.Mapa` into `collision_map`
  (`h0_7cc`), then scans 20 (a, b) or 13 (c, d) object-map cells: 0 and 7-`$1f` skip, `$ff` ends the
  scan, 1-6 and `$87`-`$fe` take a creature slot (`h0_681c`) and clear the cell, `$20`-`$3f` add an
  animated cell (`h0_6882`), `$40`-`$86` blit a bob.
- States set the position (biased to each guard: y around `$53c`, y 0-8, x 0 and `$8000`, the only x
  that passes `redraw_edge_c`'s `x >= 1` and fails its `tst.w`), the scanned cells, the five slot flags
  and the 48 animated-cell flags (all full one state in six). The base lays tiles, shapes and screen as
  noise and the tile and shape maps.
- `h0_7cc` indexes 32-byte shapes by a byte from `h0+$bda0`, so shapes past `$6b` are read from the
  object map at `h0+$cb38`.

`tests/gen_edge_redraw.rs` measures branches by flipping, as `gen_drawing.rs` does. `h0_3ac2` also
waits for VHPOSR to leave `$e7`-`$66`, and the oracle's counter runs on across calls, so every run there
first sets the counter by the state's index; `the_beam_sync_sets_the_raster_and_runs_repeat` checks it.
Every conditional branch of each unit goes both ways except the blitter-busy `bne` at `h0+$3ad6`, which
no state can take on the oracle (in every unit that holds `h0_3ac2`). `h0_7cc` and `h0_6852` have no branch.

Measured 2026-11-05, 2 000 states each, all returned, no hang, fault or unmodelled blit:

| Generator | `RigReport::paths` | Branches both ways | Named, not taken |
|---|---|---|---|
| `redraw_edge_a` | 1 174 | 10 | 1 |
| `redraw_edge_b` | 1 137 | 11 | 1 |
| `redraw_edge_c` | 1 095 | 9 | 0 |
| `redraw_edge_d` | 1 168 | 7 | 0 |
| `h0_774` | 3 | 1 | 1 |
| `h0_7cc` | 1 | 0 | 0 |
| `h0_a06`, `h0_ce4`, `h0_104c`, `h0_1390` | 2 each | 1 each | 1 each |
| `h0_3ac2` | 2 | 1 | 1 |
| `h0_681c` | 6 | 2 | 0 |
| `h0_6852` | 1 | 0 | 0 |
| `h0_6882` | 98 | 13 | 0 |

`cargo test -p fd-conform --test gen_edge_redraw -- --nocapture` prints each branch's counts.

## Generators: `character`, second share (T53)

`fd-conform/src/gen/character_anim.rs`, one generator per T53 entry of `re/translate.toml`, named as
there. The four walk-frame pickers read `h0+$4be4` (walk counter, wraps at `$28`), `$4ece` (facing),
`$4ed1` and bits 0-1 of `$4ed2`, drawn as T35's `character::Var` draws them; the pointer tables they
copy into `char_anim_ptr` are the program's and are not poked. `read_joystick` reads JOY1DAT (a
hardware read: half the states only bits 9, 8, 1, 0, half any word) and `input_log_index` (0 to
`$2ee0`, leaning to full and one short of it). No base.

`fd-conform/tests/gen_character_anim.rs` proves them as `gen_character.rs` does, by detouring each
branch through a trapping stub. `read_joystick`'s `beq.b`s, and its `beq.w`s out of a `bra.w`'s
reach of free memory, are detoured with the instruction before them moved into the stub; a test
checks a non-trapping detour runs as the plain program. No branch is unreachable. Measured
2026-11-05, 2 000 states each, no hang or fault:

| Generator | `RigReport::paths` | Branches |
|---|---|---|
| `char_step_4be6` | 2 | 1 |
| `h0_4c28` | 2 | 1 |
| `h0_4c6a` | 6 | 11 |
| `h0_4d32` | 1 | 1 |
| `h0_4d54` | 1 | 1 |
| `read_joystick` | 10 | 9 |

`h0_4d32`'s and `h0_4d54`'s one path covers their branch both ways: each way runs one `move.l` and
an `rts`.

## The batch: `tools/translate_batch.py` (T40)

Every routine of `re/translate.toml` through the service in one unattended run, before the porting
tasks start (DESIGN §4.3; `worker` bin, $5 cap, DESIGN §5.3):

```
python3 -I games/forest-dumb/tools/translate_batch.py --max-usd 5 [--parallel 8] [--resume]
```

- One `translate.py '<routine>' --gen <name>` process per routine, the generator named as the routine,
  up to `--parallel` at once. Model calls overlap; judge builds and runs take turns on `judge.lock`.
- A routine with `split_at` starts once its callees have results: `blocked` unless all passed.
- The cap is the run's own spend. Each routine gets min($0.10, what is left less what running routines
  may still spend); below one call's worst case (96 000 answer tokens) the next waits for a running one,
  and with none running nothing more starts and the rest are `not-attempted`.
- `--resume` keeps the rows of `build/translate/batch.json` that finished (a pass whose `port.rs` is on
  disk, `fail`, `too-big`, `no-oracle`, `no-gen`) and runs the rest.
- Writes `batch.json` after each routine and `table.md`; prints a one-line summary. Sound has no
  generator and makes no call (`no-gen`).

**The ports are in the build folder of the T40 worktree, copied to the main checkout's
`games/forest-dumb/build/translate/`** (git-ignored, 2026-11-05). A porting task copies its routines'
folders (`hN-off/`) from there, or re-runs `translate.py` on one routine when its checkout has none.

### The run, 2026-11-05

87 routines: 77 pass, 1 fail, 4 too-big, 2 no-gen, 3 blocked. This run spent $0.1596 of its $5 cap in 1102 s; the table's routines cost $0.1596 in all.

| Routine | Result | Port task | Port | Generator | Paths | Tries | Cost |
|---|---|---|---|---|---|---|---|
| `h0+$774` `h0_774` | pass | T55 | `build/translate/h0-774/port.rs` | `h0_774` | 3 | 1 | $0.0012 |
| `h0+$7cc` `h0_7cc` | pass | T55 | `build/translate/h0-7cc/port.rs` | `h0_7cc` | 1 | 1 | $0.0008 |
| `h0+$844` `redraw_edge_a` | pass | T55 | `build/translate/h0-844/port.rs` | `redraw_edge_a` | 1174 | 1 | $0.0042 |
| `h0+$a06` `h0_a06` | pass | T55 | `build/translate/h0-a06/port.rs` | `h0_a06` | 3 | 1 | $0.0025 |
| `h0+$b20` `redraw_edge_b` | pass | T55 | `build/translate/h0-b20/port.rs` | `redraw_edge_b` | 1126 | 1 | $0.0036 |
| `h0+$ce4` `h0_ce4` | pass | T55 | `build/translate/h0-ce4/port.rs` | `h0_ce4` | 3 | 1 | $0.0022 |
| `h0+$df6` `redraw_edge_c` | too-big | T55 |  | `redraw_edge_c` |  |  |  |
| `h0+$104c` `h0_104c` | pass | T55 | `build/translate/h0-104c/port.rs` | `h0_104c` | 3 | 1 | $0.0020 |
| `h0+$114a` `redraw_edge_d` | pass | T55 | `build/translate/h0-114a/port.rs` | `redraw_edge_d` | 1168 | 1 | $0.0037 |
| `h0+$1390` `h0_1390` | pass | T55 | `build/translate/h0-1390/port.rs` | `h0_1390` | 3 | 1 | $0.0015 |
| `h0+$1fec` `game_frame` | blocked | T08 |  | `game_frame` |  |  |  |
| `h0+$20b0` `countdown_step` | pass | T23 | `build/translate/h0-20b0/port.rs` | `countdown_step` | 3 | 1 | $0.0029 |
| `h0+$2104` `play_keys` | pass | T23 | `build/translate/h0-2104/port.rs` | `play_keys` | 5 | 1 | $0.0006 |
| `h0+$2146` `help_key` | pass | T26 | `build/translate/h0-2146/port.rs` | `help_key` | 3 | 1 | $0.0005 |
| `h0+$2176` `esc_key` | pass | T23 | `build/translate/h0-2176/port.rs` | `esc_key` | 4 | 1 | $0.0005 |
| `h0+$21c0` `h0_21c0` | pass | T25 | `build/translate/h0-21c0/port.rs` | `h0_21c0` | 6 | 1 | $0.0028 |
| `h0+$24ce` `h0_24ce` | pass | T25 | `build/translate/h0-24ce/port.rs` | `h0_24ce` | 7 | 1 | $0.0013 |
| `h0+$2648` `h0_2648` | no-gen |  |  |  |  |  |  |
| `h0+$2698` `char_object_cells` | pass | T22 | `build/translate/h0-2698/port.rs` | `char_object_cells` | 108 | 1 | $0.0011 |
| `h0+$26ec` `h0_26ec` | pass | T22 | `build/translate/h0-26ec/port.rs` | `h0_26ec` | 23 | 1 | $0.0039 |
| `h0+$29ca` `h0_29ca` | pass | T22 | `build/translate/h0-29ca/port.rs` | `h0_29ca` | 1 | 1 | $0.0017 |
| `h0+$2a6e` `score_add_routine` | pass | T22 | `build/translate/h0-2a6e/port.rs` | `score_add_routine` | 1 | 1 | $0.0018 |
| `h0+$2c7e` `h0_2c7e` | blocked | T52 |  | `h0_2c7e` |  |  |  |
| `h0+$2d4c` `h0_2d4c` | pass | T52 | `build/translate/h0-2d4c/port.rs` | `h0_2d4c` | 177 | 1 | $0.0020 |
| `h0+$2dd6` `h0_2dd6` | pass | T22 | `build/translate/h0-2dd6/port.rs` | `h0_2dd6` | 125 | 1 | $0.0026 |
| `h0+$2e0e` `h0_2e0e` | pass | T22 | `build/translate/h0-2e0e/port.rs` | `h0_2e0e` | 10 | 1 | $0.0024 |
| `h0+$2fae` `h0_2fae` | pass | T24 | `build/translate/h0-2fae/port.rs` | `h0_2fae` | 211 | 1 | $0.0009 |
| `h0+$2fea` `h0_2fea` | pass | T24 | `build/translate/h0-2fea/port.rs` | `h0_2fea` | 23 | 1 | $0.0036 |
| `h0+$320a` `h0_320a` | pass | T24 | `build/translate/h0-320a/port.rs` | `h0_320a` | 1 | 1 | $0.0005 |
| `h0+$3222` `h0_3222` | pass | T24 | `build/translate/h0-3222/port.rs` | `h0_3222` | 1 | 1 | $0.0005 |
| `h0+$32f8` `h0_32f8` | pass | T24 | `build/translate/h0-32f8/port.rs` | `h0_32f8` | 1 | 1 | $0.0009 |
| `h0+$3322` `h0_3322` | pass | T24 | `build/translate/h0-3322/port.rs` | `h0_3322` | 1 | 1 | $0.0008 |
| `h0+$3530` `h0_3530` | pass | T24 | `build/translate/h0-3530/port.rs` | `h0_3530` | 2 | 1 | $0.0006 |
| `h0+$3566` `h0_3566` | pass | T24 | `build/translate/h0-3566/port.rs` | `h0_3566` | 2 | 1 | $0.0007 |
| `h0+$35a0` `h0_35a0` | pass | T24 | `build/translate/h0-35a0/port.rs` | `h0_35a0` | 2 | 1 | $0.0017 |
| `h0+$35dc` `h0_35dc` | pass | T24 | `build/translate/h0-35dc/port.rs` | `h0_35dc` | 2 | 1 | $0.0006 |
| `h0+$3698` `h0_3698` | pass | T24 | `build/translate/h0-3698/port.rs` | `h0_3698` | 15 | 1 | $0.0017 |
| `h0+$3ab4` `h0_3ab4` | pass | T10 | `build/translate/h0-3ab4/port.rs` | `h0_3ab4` | 1 | 1 | $0.0005 |
| `h0+$3ac2` `h0_3ac2` | pass | T55 | `build/translate/h0-3ac2/port.rs` | `h0_3ac2` | 3 | 1 | $0.0016 |
| `h0+$3adc` `h0_3adc` | no-gen |  |  |  |  |  |  |
| `h0+$3b2a` `char_hit_check` | pass | T24 | `build/translate/h0-3b2a/port.rs` | `char_hit_check` | 105 | 2 | $0.0089 |
| `h0+$3dd8` `h0_3dd8` | pass | T24 | `build/translate/h0-3dd8/port.rs` | `h0_3dd8` | 4 | 1 | $0.0011 |
| `h0+$3e0c` `char_update` | blocked | T08 |  | `char_update` |  |  |  |
| `h0+$3e30` `char_step` | pass | T08 | `build/translate/h0-3e30/port.rs` | `char_step` | 74 | 1 | $0.0038 |
| `h0+$3ea8` `h0_3ea8` | pass | T08 | `build/translate/h0-3ea8/port.rs` | `h0_3ea8` | 28 | 1 | $0.0031 |
| `h0+$410c` `h0_410c` | pass | T08 | `build/translate/h0-410c/port.rs` | `h0_410c` | 9 | 1 | $0.0017 |
| `h0+$416c` `h0_416c` | pass | T08 | `build/translate/h0-416c/port.rs` | `h0_416c` | 2 | 1 | $0.0007 |
| `h0+$4274` `h0_4274` | pass | T08 | `build/translate/h0-4274/port.rs` | `h0_4274` | 6 | 1 | $0.0010 |
| `h0+$4696` `h0_4696` | pass | T08 | `build/translate/h0-4696/port.rs` | `h0_4696` | 4 | 1 | $0.0006 |
| `h0+$4718` `h0_4718` | pass | T08 | `build/translate/h0-4718/port.rs` | `h0_4718` | 24 | 2 | $0.0037 |
| `h0+$47e2` `h0_47e2` | pass | T08 | `build/translate/h0-47e2/port.rs` | `h0_47e2` | 3 | 1 | $0.0009 |
| `h0+$48be` `h0_48be` | pass | T08 | `build/translate/h0-48be/port.rs` | `h0_48be` | 24 | 1 | $0.0032 |
| `h0+$497c` `h0_497c` | pass | T08 | `build/translate/h0-497c/port.rs` | `h0_497c` | 3 | 1 | $0.0010 |
| `h0+$4b0e` `h0_4b0e` | pass | T08 | `build/translate/h0-4b0e/port.rs` | `h0_4b0e` | 8 | 1 | $0.0016 |
| `h0+$4be6` `char_step_4be6` | pass | T51 | `build/translate/h0-4be6/port.rs` | `char_step_4be6` | 2 | 1 | $0.0008 |
| `h0+$4c28` `h0_4c28` | pass | T51 | `build/translate/h0-4c28/port.rs` | `h0_4c28` | 2 | 1 | $0.0009 |
| `h0+$4c6a` `h0_4c6a` | pass | T51 | `build/translate/h0-4c6a/port.rs` | `h0_4c6a` | 6 | 1 | $0.0009 |
| `h0+$4d32` `h0_4d32` | pass | T51 | `build/translate/h0-4d32/port.rs` | `h0_4d32` | 1 | 1 | $0.0004 |
| `h0+$4d54` `h0_4d54` | pass | T51 | `build/translate/h0-4d54/port.rs` | `h0_4d54` | 1 | 1 | $0.0004 |
| `h0+$4d7e` `camera_update` | fail | T09 |  | `camera_update` | 385 | 3 | $0.0184 |
| `h0+$4e9a` `h0_4e9a` | pass | T09 | `build/translate/h0-4e9a/port.rs` | `h0_4e9a` | 65 | 2 | $0.0035 |
| `h0+$4ea6` `h0_4ea6` | pass | T09 | `build/translate/h0-4ea6/port.rs` | `h0_4ea6` | 64 | 1 | $0.0018 |
| `h0+$4eb4` `h0_4eb4` | pass | T09 | `build/translate/h0-4eb4/port.rs` | `h0_4eb4` | 64 | 1 | $0.0017 |
| `h0+$4ec2` `h0_4ec2` | pass | T09 | `build/translate/h0-4ec2/port.rs` | `h0_4ec2` | 65 | 1 | $0.0025 |
| `h0+$4f84` `char_probe` | too-big | T08 |  | `char_probe` |  |  |  |
| `h0+$4f90` `char_probe_flags` | pass | T08 | `build/translate/h0-4f90/port.rs` | `char_probe_flags` | 15 | 1 | $0.0009 |
| `h0+$513c` `h0_513c` | pass | T09 | `build/translate/h0-513c/port.rs` | `h0_513c` | 3 | 1 | $0.0007 |
| `h0+$51fc` `h0_51fc` | pass | T09 | `build/translate/h0-51fc/port.rs` | `h0_51fc` | 4 | 1 | $0.0008 |
| `h0+$526a` `h0_526a` | pass | T09 | `build/translate/h0-526a/port.rs` | `h0_526a` | 4 | 1 | $0.0009 |
| `h0+$52d8` `h0_52d8` | pass | T09 | `build/translate/h0-52d8/port.rs` | `h0_52d8` | 4 | 1 | $0.0010 |
| `h0+$5346` `h0_5346` | pass | T09 | `build/translate/h0-5346/port.rs` | `h0_5346` | 4 | 1 | $0.0012 |
| `h0+$5462` `char_sprite_position` | pass | T10 | `build/translate/h0-5462/port.rs` | `char_sprite_position` | 4 | 1 | $0.0012 |
| `h0+$5820` `h0_5820` | pass | T52 | `build/translate/h0-5820/port.rs` | `h0_5820` | 9 | 1 | $0.0034 |
| `h0+$5e44` `h0_5e44` | too-big | T52 |  | `h0_5e44` |  |  |  |
| `h0+$62b6` `h0_62b6` | too-big | T25 |  | `h0_62b6` |  |  |  |
| `h0+$681c` `h0_681c` | pass | T55 | `build/translate/h0-681c/port.rs` | `h0_681c` | 6 | 1 | $0.0011 |
| `h0+$6852` `h0_6852` | pass | T55 | `build/translate/h0-6852/port.rs` | `h0_6852` | 1 | 1 | $0.0007 |
| `h0+$6882` `h0_6882` | pass | T55 | `build/translate/h0-6882/port.rs` | `h0_6882` | 98 | 1 | $0.0013 |
| `h0+$69d6` `lives_minus_one` | pass | T23 | `build/translate/h0-69d6/port.rs` | `lives_minus_one` | 2 | 1 | $0.0022 |
| `h0+$6a04` `countdown_plus_100` | pass | T22 | `build/translate/h0-6a04/port.rs` | `countdown_plus_100` | 2 | 2 | $0.0043 |
| `h0+$6a4c` `items_left_minus_one` | pass | T22 | `build/translate/h0-6a4c/port.rs` | `items_left_minus_one` | 2 | 1 | $0.0042 |
| `h0+$8dfc` `read_joystick` | pass | T51 | `build/translate/h0-8dfc/port.rs` | `read_joystick` | 10 | 1 | $0.0015 |
| `h0+$94ba` `h0_94ba` | pass | T10 | `build/translate/h0-94ba/port.rs` | `h0_94ba` | 162 | 2 | $0.0038 |
| `h0+$94fe` `h0_94fe` | pass | T10 | `build/translate/h0-94fe/port.rs` | `h0_94fe` | 162 | 1 | $0.0020 |
| `h0+$9550` `h0_9550` | pass | T10 | `build/translate/h0-9550/port.rs` | `h0_9550` | 12 | 1 | $0.0013 |
| `h0+$9754` `h0_9754` | pass | T10 | `build/translate/h0-9754/port.rs` | `h0_9754` | 2 | 1 | $0.0018 |
| `h0+$9b06` `char_sprite_copy` | pass | T10 | `build/translate/h0-9b06/port.rs` | `char_sprite_copy` | 4 | 2 | $0.0029 |

Translated by hand, under the porting task that owns each:

- T08: `game_frame` (blocked: split callee h0+$2c7e is blocked); `char_update` (blocked: split callee h0+$4f84 is too-big); `char_probe` (too-big)
- T09: `camera_update` (fail)
- T25: `h0_62b6` (too-big)
- T52: `h0_2c7e` (blocked: split callee h0+$5e44 is too-big); `h0_5e44` (too-big)
- T55: `redraw_edge_c` (too-big)

- The four `too-big` routines are T34's four `by_hand`. The three `blocked` ones call one of them
  through `split_at`: `char_update` calls `char_probe`, `h0_2c7e` calls `h0_5e44`, and `game_frame` calls
  `h0_2c7e`. `translate.py` judges a caller only with its callees' passing ports in their folders, so
  each waits for its callee's hand port (a `port.rs` and a passing `result.json` there) or is
  translated by hand.
- `camera_update` failed three answers (1 409, 1 409, then 72 of 2 000 states differ) where T36's
  call passed it on try 1; T09 may re-run `translate.py 'camera_update' --gen camera_update` first.
- Two first answers did not compile (`char_hit_check`, `h0_94ba`) and passed on try 2. 86 calls in
  all: 211 547 tokens in, 702 210 out.

## Engine ports: `camera` (T09)

`fd-core/src/level/camera.rs`, proven by `fd-conform/tests/camera_routines.rs` on 10 000 states of each
T36 generator. Behaviour and interruptible points: [09](09-play-measured.md), "The camera".

- **By hand: `camera_update`**, because the batch answered `fail` (above). It was written from the listing
  and T36's witness in `tests/gen_camera.rs`, which already matched the original on 2 000 states; the
  service was not re-run. It matched on the first check: 532 paths.
- The other nine came from T40's passing ports, reshaped onto `Camera` (words at the original's width,
  one `scroll` and one `follow` for the four directions). Each matched on the first check.
- An engine port models no scratch register. `judge::check_regs` with `routine::RegsCompared` names the
  registers compared: `d0` for the scrolls and flag steps (a flag step's loop tests a scroll's `d0`), none
  for `camera_update` and `h0_513c` (`level3_handler` saves and restores every register around
  `game_frame`; `camera_update` reloads `d0`, `d1` after each flag step). `check` still compares all.

## Engine ports: edge redraws (T55)

`fd-core/src/level/redraw.rs` (and `level/collision.rs`), proven by `fd-conform/tests/edge_redraw_routines.rs`
on 10 000 states of each T56 generator; paths 3, 1, 2 680, 2, 2 633, 2, 2 110, 2, 2 119, 3, 2, 6, 1, 127 in
`re/translate.toml`'s order. Behaviour and interruptible points: [09](09-play-measured.md), "The edge redraws".

- **By hand: `redraw_edge_c`**, because it is `by_hand` (181 instructions of its own flow, over 180) and the
  batch answered `too-big`. Written from the listing beside its twin `redraw_edge_d`; it matched on the
  first check.
- The other 13 came from T40's passing ports, reshaped onto `Edges`, `CreatureSlots` and `AnimatedCells`.
  The level buffers are read, and the outputs written, through `redraw::Io` at the original's offsets,
  because the original's long arithmetic runs past each map's end (a column of -1 reads the object map, a
  wild x the program after the buffers); the rig's `Io` is the state's memory, the game's `PlayIo`.
- The blits are a list (`Blit`), turned into the original's register writes in order by the rig's `Io`.
- `h0_681c` and `h0_6852` are also reached from `h0+$3a38`'s fixed creature starts, and `h0_6882` fills
  `h0_62b6`'s lists (T25): kept here as `CreatureSlots::start`, `fill` and `AnimatedCells::add` for T24 and
  T25 to call.
- Two kinds of state are left out of the check, each counted by a test: T56's `h0_6852` states with `d7` 5
  to `$3f` (1 in 50; the original writes past the five slots over its own code; its one caller passes 0 to
  4), taken into range; and redraw states where the engine's bob blit, masked to chip RAM, would land on
  the animated-cell table (hunk 2), which the original then reads back (35, 53, 0, 0 of 2 000 for a to d;
  wild x or the `d0` carry), drawn again. The rig's adapters store only the bytes the engine changed, so a
  blit's write is not undone by the store.
- Registers compared: `d0` after `h0_681c` and after the row bobs `h0_a06`, `h0_ce4`, whose high word the
  scan carries to the next bob; none for the rest (`main_loop` reloads what it reads).

## Engine ports: taking and the write-back (T22)

`fd-core/src/objects/collectible.rs` (`MapObjects`), proven by `fd-conform/tests/collect_routines.rs` on
10 000 states of each T38 and T37 generator; paths 115, 23, 1, 1, 142, 10, 2, 2 for `char_object_cells`,
`h0_26ec`, `h0_29ca`, `score_add_routine`, `h0_2dd6`, `h0_2e0e`, `countdown_plus_100`, `items_left_minus_one`.
Behaviour and interruptible points: [10](10-object-system.md), "Taking and writing back, ported".

- The object map, tile map and tile table are read, and taken cells and written-back creatures written,
  through `objects::Io` at hunk 0 offsets, as T55's redraws; the rig's `Io` is the state's memory, play's
  `objects::PlayIo`. The item redraw is a `Blit::Cell` (a source offset from the tile table, not a tile
  number); the sounds are `objects::Sound`, turned into the Paula writes by the rig and dropped in play.
- Bonus scores are read from `h0_26ec`'s own `move.l #amount, score_add` immediates
  (`collectible::BONUS_SCORES`): a 17-entry table in the code fails `purity.rs`.
- Registers compared: `d0` after `h0_29ca` (`h0_26ec` compares the bonus value in it); none for the rest.

## Object routines (T49)

T49 was to translate the object routines T21 found reached only through dispatch tables, which T34's
call-tree scan cannot see. T21 found none (docs/10, "No dispatch tables") and appended no entry to
`re/translate.toml`, so T49 wrote no generator and made no call: $0.

Checked 2026-11-05: the 36 routines `re/objects-W1-1.toml`'s `routines` and docs/10 name are all in
`re/translate.toml` (enforced by `fd-conform/tests/ported.rs`) and all in the batch table above. 33
pass; `h0_2c7e` (blocked), `h0_5e44` (too-big) go by hand under T52, and `h0_62b6` (too-big) under
T25, as listed above.

## Ported (porting tasks)

What each porting task did with its share of the table above, and which routines it translated by hand.

- **T51** (2026-11-05): `char_step_4be6`, `h0_4c28`, `h0_4c6a`, `h0_4d32`, `h0_4d54`, `read_joystick`. None by
  hand: each was rewritten from its passing T40 port and the listing as `fd_core::character::{Anim,
  JoyFlags}` methods, and passes the rig on 10 000 states (`tests/character_anim_routines.rs`; paths 2, 2,
  6, 1, 1, 10). The literal ports needed no fix.
- **T08** (2026-11-05): the fourteen character routines, as `fd_core::character::Character` methods
  over `State.motion` (paths on the rig: `char_update` 301, `char_step` 91, `h0_3ea8` 29, the rest 1
  to 24). By hand: `char_probe` (too-big) and `char_update` (blocked behind it), from the listing; the
  other twelve rewritten from their passing T40 ports and the listing. `game_frame` (blocked) went to
  T63 (owner, 2026-11-05): its rig proof needs every routine it calls ported first.
- **T22** (2026-11-06): the eight routines above. By hand, as Muse's examples: `add_score`
  (`score_add_routine`) and `redraw_cell` (`h0_29ca`), the smallest two, one showing the BCD helpers and one
  `Io`'s blit. From Muse (`reshape.py --task T22 --all`, then `--only object_cells` once `take` had passed):
  `object_cells`, `take`, `write_back`, `write_back_slot`, `add_time`, `count_item`, each passing with no
  quality issue; $0.039 in all, 16 calls. Fitted by hand: `write_back` calls `write_back_slot` (Muse's
  first answer inlined it, written before that unit passed), `write_back_slot` folded to one try list, and
  the 17-entry bonus table moved into the program's own code (purity). The first run's prompts lacked the
  rig table lines (rustfmt had wrapped them); the adapters are named since, and `test_reshape.py` passes.
- **T23** (2026-11-06): `countdown_step`, `play_keys`, `help_key`, `esc_key`, `lives_minus_one`, as
  `fd_core::character::Play` methods (`tests/death_routines.rs`, 10 000 states; paths 3, 5, 3, 4, 2).
  `help_key` is T26's in the table, but `play_keys` runs it whole, so T23 ported it with its caller.
  Muse wrote `countdown_step`, `play_keys` and `help_key` (`reshape.py --task T23 --all`, all passing,
  no quality issue, $0.0080 in all); fitted: `play_keys` calls `help_key` and `esc_key` instead of
  inlining them, and `countdown_step` uses the file's `sbcd`. By hand: `esc_key` and `lives_minus_one`,
  the two examples Muse is shown. `sbcd` follows the rig's 68000 digit by digit: a carry-chain formula
  differs on bad digits (`$be - 1`).
- **T26** (2026-11-06): nothing for Muse. Its one routine, `help_key`, was ported with its caller by
  T23. The rest is `main_loop`'s level-end test (`h0+$286`..`$298`) and `next_world`'s first
  instruction (`addi.l #1, world_index`), outside `re/translate.toml` and five instructions in all,
  written by hand in `fd_core::level_end`, so no `[T26]` table in `re/reshape.toml`; $0 spent. They
  are proven by the `level-end` case (`tests/level_end.rs`), not the rig: `main_loop` is foreground
  code no rig unit holds.
- **T25** (2026-11-06): `h0_24ce`, `h0_21c0` (`fd_core::objects::spring`) and `h0_62b6`
  (`fd_core::objects::animated`), `MapObjects` methods (`tests/mechanic_routines.rs`, 10 000 states;
  paths 7, 6, 791). Muse wrote `spring_redraw` (`h0_21c0`; `reshape.py --task T25 --all`, pass on try 2,
  no quality issue, $0.0089); fitted by hand: the timer's count written plainly, the second band's 16
  named. By hand: `spring_trigger` (`h0_24ce`), the one example Muse is shown, since T25 has only two
  routines with a literal port; and `animate` (`h0_62b6`), too-big for the service. Its value compares
  test the N flag only (`cmpi.b #8; bpl`), so `$a0` to `$a7` end the list rather than draw as items; a
  mutant in the test holds it.

## The engine-shape trial: `tools/reshape.py` (T62)

Whether Muse can also write the engine-shaped port, judged by the porting task's own rig test, for
less than Opus spends (DESIGN §4.3, owner 2026-11-05). Measured on T08's and T09's routines, whose
engine code and rig tests exist. A measurement only: nothing in `fd-core` was replaced.

### Calling it

```
python3 -I games/forest-dumb/tools/reshape.py 'h0+$4274' --task T08 --max-usd 0.10
python3 -I games/forest-dumb/tools/reshape.py --opus-cost
```

One result line as `translate.py`'s, plus `routines` and `fn`; the answer is
`build/reshape/<slug>/port.rs`, an `impl` block. `--tries`, `--max-usd`, the key, the model call and
the spend guard are `translate.py`'s.

### What it does

- A **unit** is one engine function and the routines its rig test checks with it (`re/reshape.toml`
  since 2026-11-06; `TASKS` in the tool before). T09's `follow` serves four flag steps and `scroll` four scrolls, so each is one unit judged on
  its four routines, and its prompt holds four literal ports. `char_update`, `char_probe` and
  `camera_update` have no passing literal port and are not units.
- **Leave one out.** The prompt holds the unit's literal ports (T40's, from `build/translate` here or in
  the main checkout), the subsystem's typed state as a skeleton (structs, enums, consts and `LAYOUT` in
  full; every other function's doc and signature, no body), and one other unit's engine function,
  literal port and rig line as the example. The unit's function, the helpers only it calls
  (`step`: `dying_fall`, `rise`; `idle`: `slide`, `slide_sound`; `scroll`: `set`) and its rig lines are
  left out; `test_reshape.py` checks this on every built prompt.
- **The judge** copies `engine/` to `build/reshape/scratch/engine` (path dependencies outside it made
  absolute, `re/` linked), cuts the unit's functions out, appends the answer, limits the rig test's loop
  to the unit's routines (`RESHAPE_ONLY`) and runs that test: 10 000 states a routine with its
  generators. A pass counts only if the test reports paths for every routine of the unit; a name
  missing from the rig table is a usage error, not a pass of nothing. The copy must stay inside the checkout, because fd-tools finds the zips through git.
  Warm, a build and run takes about 6 s.

### The run, 2026-11-05

Worker action, DESIGN §5.3 row "Muse writes an engine-shaped port", `--max-usd 0.10` a unit:

| Task | Routines | Function | Result | Tries | Muse $ | Paths | s |
|---|---|---|---|---|---|---|---|
| T08 | `char_step` | `step` | pass | 1 | 0.0033 | 91 | 76 |
| T08 | `h0_3ea8` | `idle` | pass | 2 | 0.0041 | 29 | 196 |
| T08 | `h0_410c` | `land_or_fall` | pass | 1 | 0.0025 | 9 | 82 |
| T08 | `h0_416c` | `fall_step` | pass | 1 | 0.0016 | 2 | 26 |
| T08 | `h0_4274` | `rise_step` | pass | 1 | 0.0017 | 6 | 34 |
| T08 | `h0_4696` | `move_x` | pass | 1 | 0.0014 | 4 | 16 |
| T08 | `h0_4718` | `left_pressed` | pass | 1 | 0.0027 | 24 | 83 |
| T08 | `h0_47e2` | `run_left_step` | pass | 1 | 0.0018 | 3 | 36 |
| T08 | `h0_48be` | `right_pressed` | pass | 1 | 0.0027 | 24 | 48 |
| T08 | `h0_497c` | `run_right_step` | pass | 1 | 0.0024 | 3 | 45 |
| T08 | `h0_4b0e` | `up_pressed` | pass | 1 | 0.0017 | 8 | 23 |
| T08 | `char_probe_flags` | `probe_flags` | pass | 1 | 0.0018 | 15 | 31 |
| T09 | `h0_4e9a`, `h0_4ea6`, `h0_4eb4`, `h0_4ec2` | `follow` | pass | 1 | 0.0023 | 260 | 47 |
| T09 | `h0_513c` | `clamp` | pass | 1 | 0.0010 | 3 | 29 |
| T09 | `h0_51fc`, `h0_526a`, `h0_52d8`, `h0_5346` | `scroll` | pass | 1 | 0.0012 | 16 | 28 |

15 of 15 units (21 routines) pass, $0.0324 and 800 s in all. Every first answer passed: `idle`'s first
try was a call error, not a wrong answer.

### Against Opus

`--opus-cost` sums `costUSD` over every session of a task in the run's conversation logs (implement and
review, restarts included) and divides by the routines it ported to engine code:

| Task | Sessions | Opus $ | Routines | $ a routine |
|---|---|---|---|---|
| T08 | implement-1, review-1 | 12.50 | 14 | 0.89 |
| T09 | implement-1, implement-2, review-1 | 9.68 | 10 | 0.97 |
| T51 | implement-1, implement-2, review-1 | 9.39 | 6 | 1.56 |

Muse: $0.0015 a routine (21 routines, $0.0324). The Opus figure is the whole task, not only reshaping:
interruptible-point notes, `symbols.toml`, `LAYOUT`, the rig tests and mutants, docs, the review, and
the by-hand ports of `char_probe`, `char_update` and `camera_update`. The trial needed those tests to
exist; a task using `reshape.py` still writes them and reviews the answers.

**What the answers look like.** On typed state throughout: no raw memory, no `unsafe`, the tables read
through `table()`. But less factored than Opus's: 717 lines against 441 (Opus's with doc comments).
`scroll` is four near-identical arms where Opus wrote one; `idle` repeats the slide for each side and
inlines `slide_sound`; table offsets are bare numbers where the file has named constants; no doc
comments. Passing the rig, they are not yet the code `fd-core` keeps.

### The owner's decision

Whether T22, T24, T25 and later porting tasks use `reshape.py` is the owner's. Asked 2026-11-05 with
the recommendation "Muse drafts, the builder tidies" (the builder writes the rig test and the tool's
`TASKS` entry first, then tidies and re-proves the passing answer). The owner was asleep, so the
coordinator set the question aside as T66, `blocks T22`.

Answered 2026-11-06 (owner): yes, and Muse tidies first. Steer the prompt toward the file's own style
and treat Muse as a service with an automatic quality check that sends a passing answer back with what
to fix, so the builder gets polished code and tidies only what is left. How: the next section.

## Reshaping by Muse (porting tasks, owner 2026-11-06)

Every porting task from T22 on (T22, T23, T24, T25, T26, T52) has Muse write its engine functions and
the builder fit, finish and prove them (DESIGN §4.4). By hand only after Muse fails a unit.

1. **Pack.** `python3 -I games/forest-dumb/tools/task_pack.py T22` and read the pack whole: the task,
   the plan's notes, each routine's listing, these steps.
2. **By hand, before any call.** The interruptible-points notes (`toolkit/docs/porting-routines.md`
   §1), `symbols.toml` and `LAYOUT` entries, the subsystem's typed state, and the rig test with its
   routine table (`fd-conform/tests/<subsystem>_routines.rs`, as T08's `character_routines.rs`). Write
   two small units by hand as the examples Muse is shown (a unit's prompt shows an example other than
   itself; each example's routine needs a passing literal port). Write every other engine function as
   a stub: its `///` doc, its exact signature, `todo!()` as the body, called by the rig test's table.
   The rig test prints `paths: <name> N (h hangs), ...` or `<name>: N paths` per routine it ran.
3. **The table.** Add `[T22]` to `re/reshape.toml` (its header says what each field is): the source
   file, the impl header, the rig test and its loop line, the state files, the harness items, the
   example, one unit per engine function.
4. **Muse.** `python3 -I games/forest-dumb/tools/reshape.py --task T22 --all --max-usd 0.10` (worker
   bin, DESIGN §5.3). It skips the `examples`, prints one result line a unit as it finishes, and runs
   each failed unit once more when another passed: the judge's copy fills a still-stubbed function
   with that unit's passing answer, so a caller judged before its callee gets a second chance.
   `--only fn_a,fn_b` reruns some. Each unit's answer is `build/reshape/<slug>/port.rs`, an `impl`
   block that passed the rig on 10 000 states.
5. **Fit and finish.** Replace each stub with its answer, fix what the result's `quality` list still
   names, rerun the rig test. Rerun a unit that failed with `--only` once the answers it calls are
   fitted; then it is the builder's, from the listing, and is listed as by hand below with why.
6. **Record** under "Ported" below: which functions came from Muse, its spend, and which by hand.

**What Muse is told and checked for.** The prompt's style rules: a `///` doc on every function naming
the original routine, names not numbers (struct fields, the state's consts, a new `const` for a new
meaningful number), no register shuffling or memory by address, mirror-image cases as one helper, the
example's style and length. After a pass, `quality()` checks the answer without a model call: missing
docs, `cpu.`/`mem.` or `h(0, 0x...)` left in, numbers that are a named const, other bare hex numbers,
a block of four lines repeated (numbers and left/right aside), and the compiler's warnings on the
answer's lines. Issues go back to Muse (`--tidy 2` rounds by default); every tidied answer must pass
the rig again, and the kept answer is the passing one with the fewest issues. On T08's and T09's
hand-written functions the check finds nothing in 12 of 15 (`test_reshape.py`).

**First run with the style rules and the loop, 2026-11-06** (worker action, `--all --only`, 7 units of
T08 and T09, against their committed rig tests): all 7 pass on Muse's first answer; 5 had no issue,
`idle` and `scroll` were clean after one tidy round; $0.0216 in all. 417 non-blank lines against Opus's
294 for the same functions (T62's answers, without the rules: 717 against 441), with more `///` docs than
Opus's; `scroll` is one function over a direction, where T62's answer had four near-identical arms.

The judge's copy needs `build/toolkit` linked to the checkout's toolkit since T14's build tag hashes
`<workspace>/../../../toolkit`; `scratch()` makes the link (the first run failed every unit on it).
