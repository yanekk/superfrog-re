# 13 — The front end

Everything around the levels, as `engine/crates/sf-core/src/front` ports it from the game
executable `d1_c03` (addresses `hN+$off`, docs/08): boot, the title and its intro, the menu, the
options and level codes, attract mode up to its demo, and the end sequence. Claims marked
**(oracle)** are checked by the `front-*` and `ending` cases in `engine/conform/cases/` and
`engine/crates/sf-conform/tests/front.rs`; **(code)** comes from reading the listing.

## Where the engine starts (owner, 2026-10-12)

The engine's power-on is the moment the game program starts (`init_and_script` h0+$26). The
Kickstart screens and the crack's intro on disk 1 are not the game's and are not reproduced. Disk
loads are instant: a frame that loads reports where the original's main program is once its load
is over (`FrameOut::load`: a return address it reaches, or a call it leaves), the runner skips the
original's frames until then, takes from the original what runs on by itself while it loads (the
keyboard poll's toggle and keys, the sound driver: `front::RUNS_DURING_LOAD`) and what a load
leaves that the engine does not model (the trackloader's sector buffer at h3+$35860, `HPIC` kept
packed in hunk 1: `front::LOAD_LEAVES`, `FrameOut::resync`), and finds the line the original's
load ended on by trying them (`Game::set_load_line`).

The disk-3 story intro is below (T32, finished by T46–T48); the attract demo is T33. The end
sequence (T35) is entered from the sync point `end-sequence` (below).

## Scene flow (code, oracle)

```
boot: init_save_boot_args, hw_takeover, Accessing-disk overlay (fades in), probe drives,
      load_hero, rng_seed_from_cia (+ the level display's tables), title_screen's loads
top_loop $5c:
  title_screen $109e:
    overlays_end (the overlay fades out, 16 steps a frame)
    $136c eyes' sprites, $1d0c title copper list on buffer 0, black
    title_intro_anim $3902: eyes bounce in from the right (h0+$3cca bounce table), music starts,
      the eyes blink (5 frames from h0+$3dd0..$3de4), bounce on left, line $a0: picture back under
      them; fire (not the mouse or a key) skips to the end (oracle). With music on (back from the
      options): none of it.
    $1c0e both halves fade in (records 0 and 1, every other frame copied at line $7c)
    idle 400 frames ($2ee with h0+$24), eyes blinking (timers h0+$1368/$136a, rng_range($4b)+$4b)
      fire -> menu $1406: START GAME / OPTIONS, flashing (h0+$168c), up/down
        OPTIONS -> $16be: lives, game mode, save high scores, level code; back -> $1122 (intro)
    $1180: title out (fade), music fade unless attract, game start values
  award_pages $1e98 (attract, or a new high score): AWA1, AWA2, 300 frames each
  hiscore_and_credits $20e0: HPIC unpacked, hall of talent (20 lines in, 350 frames, 20 out),
    credits (a line every second frame at line $c5, text every 24), fire/mouse/Esc abort
  then the attract demo (T33) or the game (T21)
```

## Timing the engine keeps (oracle)

The main program paces itself on the beam: `wait_frame_raster` (line `$ff`, then `$12b`), busy
waits on fade records (`$1519e`, `$3ca2`), and waits for lines (`$a0`, `$8d`, `$c4`/`$c5`). The
vertical blank runs at the top of each frame. The engine runs each routine as a state machine to
its next wait and draws the picture as the beam saw memory: copper words read before a write show
the old value, lines a main-program part ran under show what was there before it.

- **Interrupts off past a frame's end.** A fade set-up (`$151b2`, `$153a4`, `$155a0`: divisions,
  ~44–51 lines for 32 colours) or `music_start` (`p41_init`, 20 lines on `MUSC`) begun late in a
  frame runs into the next one; that frame's interrupt waits for it.
- **Blits with priority** (DMACON `$8400`) that run past a frame's end hold the CPU off the bus:
  the reference emulator's frame then ends when the CPU takes the interrupt, about line 30 of the
  next frame, after the copper's first writes of it (its DMACON and window).
- **CPU jobs spanning frames** (owner, 2026-10-12): unpacking the options picture (12 frames), the
  eyes (12), `HPIC` (49), drawing text: the engine does them at once; their memory is compared once
  the original is out of the call (`FrameOut::jobs`). The picture shows an unpacking arriving from
  its end at an even pace.
- **Blits under the display** (review, 2026-10-12): a blit the CPU waits for takes its words'
  cycles plus, on each line the display fetches bitplanes on, 1.23 cycles per fetch slot
  (`Cx::blit_wait`; measured on the intro's restore, lines 161–308 under six planes). It decides the
  frame the fade-in starts on when fire skips the intro and is held (`front-skip-*`).
- **The beam overtaking a blit**: the credits' scroll (272 lines of one plane, from line `$c5`)
  shows each word new only if the blitter wrote it before the bitplane fetch read it
  (`blit::word_times`).
- `clr.w` to a custom register writes twice on the real 68000: the prefetched word, then 0.

## The disk-3 intro (T32; code, oracle)

`Start::Disk3` is the moment disk 3's logo program `d3_c02` starts, after its bootblock: Kickstart
and the bootblock are the load. The engine runs it as `front::story` with its own 512K memory
(chip and slow), not the game's `State`. Frames below count from that start (case
`story-intro`); **(oracle)** up to frame 555 in the test block and to 3184 in the slow case
`story-scene1-out` (about 90 s, an ignored test: `sf-conform run` it), the rest **(code)** and the
debugger's runs.

```
0–66      Team 17 logo (d3_c02): hires, 4 planes, interlaced (two copper lists by field),
          fades in, loads the intro (d3_c04 ATN!, instant), fades out
67        intro_main (d3_c04 h0, hunks h0 $C00FA0, h1 $C53100, h2 $400 = d3_c03, h3 $18FD4)
68–105    text page 1: unpacked (cf5c, ~37 frames, part of the load), faded in (8 colours)
106–505   page shown, 400 frames or fire/mouse (WaitOrFire)
506–520   text_out: fade out
521–537   scene 1 set-up: buffers cleared (2 blits), objects_init (masks: CPU clear + 5 blits,
          frames 532–536), two draws, fade in (32 colours)
538–1241  scene 1 plays, music $c51e: a draw every second frame (double-buffered bobs)
1242–1257 scene 1 out (fade with wait, black list)
1258–1738 text page 2 (in, 400 frames, out)      1739–1756 scene 2 set-up   1757– scene 2
then      text 3 + scene 3, scenes 4–9 (banks unpacked from h1, backgrounds from h0), text 4,
          scenes 10–15 (MAIN in story/scenes.rs, 15 scenes and 4 text pages), END: "insert
          disk 1" (c38e waits for fire), the hand-over to the game (T47)
```

The handler (h0+$b102, each frame at the top): frame flag and count, the shown buffer's plane
pointers (unless frozen), the music's sample stream, the fades (in, out, across: 4 records), fire
or the mouse button sets `intro_skip`, the music and effects (the game's own driver, T11, at the
intro's addresses: `AudioDriver::intro_from`), the timeline of sound events. A `SkipCheck` after
each scene goes to END once skip is set.

Timing the engine keeps (oracle, frames 0–539):

- **Handler length**: 12 lines, 17 with music, plus 34 lines per 32 colours a stepping fade
  changes. A fade's set-up runs with interrupts off: in 5 + 37/32 lines a colour, out 5 + 3/2.
  It can run across a frame's end (scene 1's out, frame 1242), so it is a job: record 0 and the
  copper lists are compared once the original has left the routine (T49).
- **Blits** on vAmiga's slow-blitter cycle counts (ABCD 4n+1, AD 2n+2, ABD 3n+2) on the bus left
  by refresh, audio (with music) and bitplane fetch (`story::clock`); 40 DMA cycles of CPU set-up
  per blit, 300 per object. The handler waits for the blit in progress.
- **`objects_init`'s CPU**: 36 274 DMA cycles plus 206 per animation frame it prepares
  (h0+$8724), then the masks' clearing (h0+$8408, 40 960 turns of `rpw pnn npp`) timed on the
  bus: 15.13 cycles a turn on refresh's bus (scenes 1–3, black list), about 18 under scene 4's
  five planes, which it runs under (no black list after `scene_out`, T50).
- **Held blanks merge**: a blit that holds the CPU through a whole frame (scene 4's second mask
  blit, ~400 lines under the display) leaves that frame's blank request set, and the next blank's
  handler serves both. The oracle's step then runs two video frames (`Amiga::frames_run`), and
  conformance steps the engine twice for it (owner, T50). When the last mask blit started on a
  fetched line, DMACON = $400 comes before the handler it held (frames 3191, 3760, 4238).
- **A draw's start** (T54): a draw the handler lets go of (`draw_frames`' `vbl_flag` wait,
  `draw_next`'s `vbl_count` wait) starts from the handler's acknowledge (`INTREQ = $20`): its
  tail, the wait's last turn (a branch the interrupt came in before is taken), the calls and
  `draw_frame`'s set-up, then `bobs_draw` (`intro::handler_to_draw`, run on `story::sim`).
  Conformance feeds the acknowledge's beam and the PC the handler returned to (the entry does not
  tell `tst.w` from `beq.b`); normal play takes the acknowledge at the start of its line. A
  handler a draw holds off is taken at the first instruction end whose last access asked for the
  bus from line 0, cycle 10 on: one held off the bus by a blit across it ends without it.
- **Unpacks** (`cf5c`): measured frames per call (`UNPACK_FRAMES`), ending on line 160.
- **Copper and Paula**: the copper's Paula writes before the CPU takes the interrupt (line 0,
  clock $14) belong to the frame before. Copper colour words a fade rewrites are read as the beam
  found them, component by component (`vbl::fade_write_times`; not yet exact, T46).
- **Schedule feeding** (owner, 2026-10-12): whether the original is still in a long job (a draw,
  a set-up) at each vertical blank is fed by conformance (`Pacing::Late`/`OnTime` from the
  original's call chain, as level lag is); play uses the engine's own timing (`Pacing::Unpaced`).
- **Loads** are instant as on disk 1: what runs on during them (the handler's sound variables,
  `RUNS_DURING_LOAD`) is taken from the original at the load, and the handler re-run.

## The end sequence (T35; code, oracle)

```
top_loop $784 (final level) -> music_fade_wait -> end_seq_load $a65c -> end_sequence $a6e4
  -> game_full_mode set: top_loop again (the title); else $7c6, back to the system (not ported)
end_seq_load: logic off, blank list, Accessing-disk overlay, END1 -> h1+$0 and END2 -> h1+$13880
  (disk 3, kept packed), music_fade_wait, MUSC (disk 1) -> h3+$64c70, music_start, overlays_end
end_sequence:
  END1 unpacked to h3+$0 (blank display); end_play_end1 $a890: anim_open $ab02 into buffer A
    (h3+$3d090), copied to B (h3+$49890); A shown (list h2+$5efc, five planes); fade in to h0+$aa80;
    hold 200; 54 passes: anim_next $ab82 into B, 2 rasters, show B, next into A, 2 rasters, show A
  hold 100, fade out
  END2 unpacked (under the faded five-plane display); end_play_end2 $a95e: the same, hold 100,
    11 passes of 3 rasters, ending after the last pass's first half
  hold 60; palette white; one more frame; fade from white (h0+$aa40) to the picture, A shown;
    hold 450; fade out; blank list
  hunk 1 ($d4a8 longs) and hunk 3 ($192dc longs) cleared; music_fade_wait; load_hero;
    rng_seed_from_cia; password_buf six spaces
```

- **Entering it** (`sf-conform`'s `end-sequence` sync point): the final level is reached as T31
  reaches it, its exit forced on its first frame (`level_state` = 1); the case starts at the end of
  the first frame the original is in `end_seq_load`. The runner inserts each disk the game asks
  for, as a player would (`sync::Disks`; this case only: a disk inserted changes the timing after it).
- **CPU work over frames** (`front::ending::CpuWork`): unpacking, the first frame's ByteRun1 body
  and each delta are done at once and their cycles spent on the beam. A delta's and the body's
  cycles are the original code's own, instruction by instruction as the 68000 manual times them
  (`front::anim`). The main program has the CPU from line 22 (the vertical blank plays the title
  music throughout), and a lowres fetch past the fourth plane takes 3.6 CPU cycles a word from it
  (72 a line under five planes); a decode after a wait first spends 600 cycles on the wait's end
  and showing the last frame. Fitted to the original's registers at the frame ends inside a
  decode: all 129 decodes of `END1` and `END2` then end in the original's frame, on the original's
  side of line 255 (`wait_frame_raster` matches `$ff` exactly), the closest within 2 lines. Each
  decode's memory is compared once the original is out of `anim_next` (`FrameOut::jobs`).
- **Unpacking**: `END1` 24 080 000 cycles, `END2` 21 046 000 under the display (measured: where the
  original's body decode had got to at the end of the frame its unpacking ended in).
- **`music_stop`** runs about 15 lines with interrupts off: from `music_fade_wait`'s raster wait
  (line 299) it ends on line 1 of the next frame, whose interrupt waits for it.
- **`load_hero`** (`front::hero`, at boot too): `HERO` and a copy with each long bit-reversed (its
  32-pixel strips mirrored) cut into 2 × 96 frames of four attached sprites at h3+$4c0b0, their
  table at h0+$19ddc; it clears h1+$20d00..$30fe0, the level's map with it.
- **The RNG**: `rng_seed_from_cia` reseeds from CIA-B's time-of-day counter, which the engine does
  not keep. Conformance takes the original's state once the load before it is over; normal play
  goes on with the RNG as it is.
- **A frame that loads** drops the CPU's writes carried over from the frame before (the end of a
  `music_stop`): they were made in the frame the load began, which conformance skips.

## Game over (T34; code, oracle)

```
level_exit_fade $75f2 (attract mode off) -> game_over_seq $74d0 (over at once with lives left):
  frog_launched $1f4, frog_spring 1, frog_camera_held 1
  GAME OVER sign (template 1, init $23f3a: frame $2cdf8) at the camera + ($82, $80)
  logic_running 1; two passes, each on wait_frame_raster:
    obj_run_all, build_bob_draw_list, the sign's bob first in the list, bob_draw, $172b4, scroll_latch
  logic_running 0; wait for fire let go, then up to 350 frames for fire
then the exit fade; game_over_check $749c (ESC or lives < 0): music_fade_wait, jmp top_loop
  (game_full_mode 0: $7c6, back to the loader, not ported)
```

- **The level's interrupt runs on** while `logic_running` is set: frog, camera, clock, scroller and
  music at 50 Hz, as in the level (`front::gameover::level_vbl`).
- **A pass spans two frames**: it starts on line `$12b`, its first half (objects, draw list) runs
  past the frame's end, the rest after the next interrupt (which stops the main program
  meanwhile), and the blits end near line 255, so the next `wait_frame_raster` may wait a frame
  more. The original's frame can end in `obj_run_all` or `build_bob_draw_list`; their memory is
  compared once it is out of them (`FrameOut::jobs`), and conformance does not count those frames
  as level lag.
- **The title again**: `top_loop` runs `title_screen` with `music_fade_wait` (one frame, the music
  is off), the Accessing-disk overlay and the title's loads. The overlay is long faded in, so the
  original starts its fade-out's set-up in the frame its last load (`$1dd2`) ends in; the engine's
  load frame stands for that frame (`LoadEnd::Left(h0+$1102)`), not the boot's (`h0+$b7e2`).
- **Cases** (`engine/conform/cases/game-over-*`): the last life lost in W1-1 (sync point
  `game-over`: lives set to 0, the frog into the first pit; the case starts on the frame
  `game_over_seq` begins), untouched and with fire, ESC, and the time up, each to the title's idle.

## The slot machine (T71–T74; code, oracle)

After a level the tally offers the slot machine (up, then fire) for the coins the level's clock
left; `slot_machine` (h0+$49e2) is the whole of it and returns to the tally. The engine enters it
with `Game::enter_slot(coins)` and reports what it gave with `Game::slot_outcome`
(`SlotOutcome { award: Award { points, lives } }`).

```
slot_machine $49e2: Accessing-disk overlay, FRUT/FRUC/FRUM into the display buffers, the level's
  sound kept over its map, the slot's effects and music; the score shown; slot_init, digits,
  pointer; fade in; attract loop $4d24 (reels 0 9 $18, the lamps' pattern) until fire or mouse
  rounds $4d4a, while coins are left:
    unless a free spin: hold menu $5de2 (holds allowed when nothing was won or paid last round
      and rng(8) = 6), spin $5a06 (a coin; $18 $18 $18 spun on unless a reel is held or
      rng(100) < 10)
    payout $4eae: the line scored, its lamp flashed 10 times (4 frames off, 4 on)
    award $51c4 (below), the win counted into the score (1 000 a frame, at once from 160 000)
    free spin $4e28: after a win with coins left, rng(100) < 6 pays the same line again
  then 100 frames or fire, fade out, the score taken, the music faded, the level's sound back
```

- **Lines** (reels' positions, `slot_payout`; the last match wins): 8 $18 0 the hi-lo game,
  $18 $18 $18 the jackpot (rung 0), 8 8 8 rung 4, 0 0 0 rung 5, 8 8 $18 rung 7, 0 0 $18 rung 8.
- **The ladder** (`slot_ladder` h0+$57b6, best first): 0 the jackpot, 1 the score shown doubled
  (the win is the score shown), 2 a life, 3–8 75 000, 50 000, 40 000, 30 000, 20 000, 10 000, 9
  the bust. The award lights the rung (its lamp record's `+6`), then scans the ladder for it: the
  bust's goes to the bust (`$5712`: 20 flashes of its lamp, nothing won), the jackpot's to every
  lamp flashed 20 times and the jackpot; any other flashes the rung above and the one three below
  with the buttons, and the menu offers the gamble (the rightmost stop).
- **The gamble**: not taken (the spin button), the rung is collected (`slot_collect` $5528, effect
  $b). Taken, `rng(100) < 50` awards the rung above again; else the rung three below: the bust if
  the flashing list (ten records, read past its -1) holds the bust lamp, else 10 flashes of that
  rung and it is collected.
- **The hi-lo game** ($57e2) starts at rung 8: each turn flashes take (spin), higher, lower and
  the rung; higher or lower turns the number (1–10) `rng(50)+50` frames, never stopping where it
  began; right climbs a rung (rung 0 is the jackpot), wrong flashes the bust lamp 20 times and wins
  nothing. Take (the spin button, which also costs a coin if one is left) collects the rung.
- **The jackpot** ($5628): each coin left is won as 5 000 and counted, then 150 000; the next
  level's code (`password_for_level` $2464: `password_table`'s next entry) is printed under the
  machine and its lamp flashes until fire.
- **Drawing on the beam** (oracle): the lamps and reels are blitted from the raster wait on line
  299 into the next frame (`slot::draw`). A lamp drawn there before the frame's interrupt starts
  later when it is off (a second `move.w`); whether the sixth lamp starts before the interrupt
  turns on a few colour clocks, and the engine's cycle model does not always land on the original's
  side of it (T74, FINDINGS). A paying line's two `sfx_play`s run from line 299 past the frame's
  end, so its flash starts in the next frame.
- **Cases** (`engine/conform/cases/slot-*`, reached by forcing W1-1's exit; `start-w1-1-late` and
  `-blink1` move the RNG, which W1-1 never moves): `slot-attract`, `slot-fire` (rounds paying
  nothing), `slot-hilo-0`..`8` (taken at each rung, 0 the jackpot), `slot-hilo-wrong`,
  `slot-hold-20000`, `slot-hold-jackpot`, `slot-gamble` (won, then lost to the bust). Their scripts
  are planned on the engine by `examples/slot_plan.rs`. Not reached at these seeds: the free spin
  and a gamble lost above the bust.

## The keyboard (oracle)

`kbd_poll` (every second frame) reads CIA-A's serial register: the last byte the keyboard sent,
`!(code << 1 | released)`. Once it has handled a new byte it writes `$ff` there, which reads as
key 0 pressed: `key_down` and `key_up` both 0. At power-on the register holds `$7e` released.

## Level codes (code, oracle)

There is no password algorithm. `password_table` (h0+$24dc) holds 28 entries of six ASCII digits
and a routine; each routine is `move.l #world,start_world; move.l #level,start_level; rts`.
Entries 0–23 start W1-1 to W6-4 in order, 24–27 the bonus level. `password_check` (h0+$2432)
takes the first entry whose six digits equal `password_buf`; a code not in the table leaves the
start as the entry set it (0, 0). The entry (h0+$22ba) maps raw keys through h0+$2e30 (codes,
`$ff`-ended) to characters at h0+$2e0a; Backspace (`$41`) starts again; Escape (`$45`), Return
(`$44`) or fire end it, as does the sixth digit. `sf_core::front::password` reads the table from
the disks (DESIGN §2.3); all 1 000 000 six-digit codes agree with the original's routine.
