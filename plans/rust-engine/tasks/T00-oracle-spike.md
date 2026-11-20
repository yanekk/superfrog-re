# T00 — oracle-spike

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal

Prove, on this Mac, that the vAmiga core can serve as the frame-exact reference: boot the owner's
four disks headless with one of the Kickstarts in `~/amiga-roms/`, run reproducibly, take joystick
input per frame, and expose chip RAM, the displayed frame and Paula register writes after every frame.
This is throwaway code whose only output is answers; the permanent harness is T05.

## Design sections this implements

DESIGN §2.1, §3.2 (`sf-oracle`), §5, §7 (vAmiga decision).

## Files

- `tools/setup/install-brew-deps.sh` — kept: idempotent `brew install cmake`, used again by T05.
- Everything else is scratch, outside the repo: `~/.cache/superfrog-re/spike/`. Commit only the script,
  the FINDINGS rows and the PROGRESS cell. Delete the scratch code when done (keep the built vAmiga if it is at the commit
T05 will pin).

## Questions it answers, and what each answer means

- Does vAmiga build here (cmake via `tools/setup/install-brew-deps.sh`, which this task creates)? No → try the previous release tag; if none builds, stop and report.
- Which Kickstart boots the crack (try 1.3 first, the game's era, then 2.04, 3.1)? Record the file
  and its SHA-1. Also try vAmiga's built-in AROS ROM; if AROS works, the oracle needs no owner ROM.
- With machine model A500 / OCS / 512 KB chip + 512 KB slow, is a run reproducible: two runs of 3 000
  frames from power-on with the same input give byte-identical chip RAM and frame on every frame?
  No → find the source (time-of-day clock, uninitialised RAM pattern, drive timing) and whether a
  config switch fixes it. If nothing does, stop: DESIGN §2.1 needs the owner.
- Can input be set per frame (port 2 joystick, plus keyboard), with the emulator stepping exactly one
  frame at a time?
- Can the shim read: all chip and slow RAM; the emulator's final frame buffer for the display window;
  every Paula register write with the frame it happened in (a hook in the core, or polling its
  register state)? Note which needs a core patch.
- Can a snapshot be saved and restored, and does a restored run continue identically?
- Speed: frames per second headless on this machine, to size T05's trace cache and the test block.

## Done when

- [ ] Every question above has an answer as a dated row in FINDINGS.md, with the pinned vAmiga
      commit, the Kickstart that boots, and any core patch needed.
- [ ] One captured frame of the Superfrog title screen was viewed by the worker and matches the
      original's title (judged against `build/` assets or a known screenshot).
- [ ] Scratch code deleted; PROGRESS notes say which branch of "what each answer means" applies.

## Outside actions

- Install brew build deps — `worker`
- Fetch and build vAmiga — `worker`
