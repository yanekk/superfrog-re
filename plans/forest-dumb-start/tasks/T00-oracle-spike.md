# T00 — oracle-spike

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal

Prove, on this machine, that the oracle can serve as the reference for Forest Dumb Forever: boot
the owner's two OFS disks headless with the toolkit's amiga-oracle, run reproducibly, take input
per frame, and expose memory, picture and sound. This is the gate of DESIGN §2.1: if the answer
to any load-bearing question is no, the plan stops and the owner decides what changes. The code
is throwaway; only the dated FINDINGS rows, the captured title picture's description and the
game's CLAUDE.md year correction stay.

## Design sections this implements

DESIGN §2.1, §2.9 (the frame length), §2.10 (the gate's unhappy paths).

## Files

- `games/forest-dumb/CLAUDE.md` — the year line, once the title screen is read.
- Everything else is scratch under `games/forest-dumb/build/spike/` (git-ignored) or the task's
  own temp dir. Commit only the FINDINGS rows, the PROGRESS cell and the CLAUDE.md correction.
  Delete the scratch code when done.

## Questions it answers, and what each answer means

- Which Kickstart boots the set? Try `~/amiga-roms/kick1.3.rom` first (the oracle's default, the
  game's era), then 2.04, then 3.1. Record the file and its SHA-1. None booting: stop, report.
- Which drive arrangement reaches the game? Boot disk 1 in df0 alone; boot with disk 2 in df1
  (`OracleConfig::with_drives`); swap df0 on the game's request (`insert_disk`). Record which the
  game wants and at what moment.
- Is a run reproducible? Two runs of 3 000 frames from power-on with the same input must give
  byte-identical chip RAM and frame every frame. If not, find the source (the game's own
  randomness, drive timing) and whether a configuration switch fixes it. If nothing does: stop,
  report, because DESIGN §2.1 stands on repeatability.
- Does per-frame input work for this game (port 2 joystick and keyboard through `FrameInput`)?
- Do the shim's reads work on these disks: chip and slow RAM, the displayed frame, Paula writes?
  The core patches already exist; this is a smoke test, not new patch work. Note anything that
  needs a new core patch.
- Does a snapshot restore and continue identically?
- Speed: frames per second headless, to size later case runs.
- The frame length: read `amiga_frame_lines()` for every frame across the boot (and check the
  custom-write hook for `VPOSW` writes). Is it 313 every frame, 312, or alternating? Record it.
- Boot to the title: how many frames from power-on to the game's title screen, and what input
  the crack intro needs (a mouse click, fire, nothing)? Capture one title frame to
  `games/forest-dumb/build/spike/title.png` (never committed), read the title's copyright line,
  and correct the year in the game's CLAUDE.md (the sources disagree on 1996 vs 1998).
- Does the game read files from disk 2 after the intro (the raw material for T03)?

## Tests

Throwaway code has no permanent suite. The evidence is the dated FINDINGS rows.

- [ ] Each question above has an answer row in FINDINGS.md, with the Kickstart, the arrangement,
      the frame length and the boot-to-title frame count stated as numbers.
- [ ] The two reproducibility runs are byte-identical, or the row says what broke and what was
      tried.

## Done when

- [ ] Every question above is answered in FINDINGS.md, and the gate verdict (go, or stop with the
      reason) is written in the PROGRESS cell.
- [ ] The year in `games/forest-dumb/CLAUDE.md` matches the title screen, dated.
- [ ] Scratch code deleted; nothing derived committed; `git status --porcelain` shows only the
      intended files.

## Outside actions

- Fetch and build vAmiga — `worker` (already built; DESIGN §5.3 if a rebuild is needed).

## Needs a person

The spike cannot start without the disks, and a session cannot list the owner's folder. From the
owner's shell, in the main checkout:

```
cp "$HOME/Documents/Amiga/Forest Dumb Forever!!! (1996)(L.K. Avalon)(PL)(Disk 1 of 2)[cr WT].zip" ~/src/superfrog-re/games/forest-dumb/
cp "$HOME/Documents/Amiga/Forest Dumb Forever!!! (1996)(L.K. Avalon)(PL)(Disk 2 of 2)[cr WT].zip" ~/src/superfrog-re/games/forest-dumb/
```

Expect: the two zips listed in `games/forest-dumb/` (they are git-ignored, so nothing changes in
git). The worker verifies it can read them and proceeds; if the names differ from the record,
it asks with the names it can see.
