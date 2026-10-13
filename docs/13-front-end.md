# 13 — The front end

Everything around the levels, as `engine/crates/sf-core/src/front` ports it from the game
executable `d1_c03` (addresses `hN+$off`, docs/08): boot, the title and its intro, the menu, the
options and level codes, and attract mode up to its demo. Claims marked **(oracle)** are checked by
the `front-*` cases in `engine/conform/cases/` and `engine/crates/sf-conform/tests/front.rs`;
**(code)** comes from reading the listing.

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

The disk-3 story intro is below (T32, finished by T36–T38); the attract demo is T33.

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
`story-intro`); **(oracle)** up to frame 539, the rest **(code)** and the debugger's runs.

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
          disk 1" (c38e waits for fire), the hand-over to the game (T37)
```

The handler (h0+$b102, each frame at the top): frame flag and count, the shown buffer's plane
pointers (unless frozen), the music's sample stream, the fades (in, out, across: 4 records), fire
or the mouse button sets `intro_skip`, the music and effects (the game's own driver, T11, at the
intro's addresses: `AudioDriver::intro_from`), the timeline of sound events. A `SkipCheck` after
each scene goes to END once skip is set.

Timing the engine keeps (oracle, frames 0–539):

- **Handler length**: 12 lines, 17 with music, plus 34 lines per 32 colours a stepping fade
  changes. A fade's set-up runs with interrupts off: in 5 + 37/32 lines a colour, out 5 + 3/2.
- **Blits** on vAmiga's slow-blitter cycle counts (ABCD 4n+1, AD 2n+2, ABD 3n+2) on the bus left
  by refresh, audio (with music) and bitplane fetch (`story::clock`); 40 DMA cycles of CPU set-up
  per blit, 300 per object. The handler waits for the blit in progress.
- **`objects_init`'s CPU**: 656 124 DMA cycles plus 206 per animation frame it prepares
  (h0+$8724): scene 1's 145 frames and scene 2's 199 put the mask blits 3022 and 3071 lines on.
- **Unpacks** (`cf5c`): measured frames per call (`UNPACK_FRAMES`), ending on line 160.
- **Copper and Paula**: the copper's Paula writes before the CPU takes the interrupt (line 0,
  clock $14) belong to the frame before. Copper colour words a fade rewrites are read as the beam
  found them, component by component (`vbl::fade_write_times`; not yet exact, T36).
- **Schedule feeding** (owner, 2026-10-12): whether the original is still in a long job (a draw,
  a set-up) at each vertical blank is fed by conformance (`Pacing::Late`/`OnTime` from the
  original's call chain, as level lag is); play uses the engine's own timing (`Pacing::Unpaced`).
- **Loads** are instant as on disk 1: what runs on during them (the handler's sound variables,
  `RUNS_DURING_LOAD`) is taken from the original at the load, and the handler re-run.

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
