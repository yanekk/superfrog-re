# T04 — fd-pack-check

**Phase:** 2 · **Depends on:** T02 · **Weight:** light

## Goal

Classify every file on both disks: packed or not, by which packer, with evidence, and unpack
anything packed into `build/` (DESIGN §2.5). The main program is stored unpacked on disk (the
north star's record), so the real question is the 48 data files, which the game loads through
dos.library and may well be plain data. This is playbook step 2: check known packers first.

## Design sections this implements

DESIGN §2.5, §2.10 (the unknown packer), §5 (the dependency policy).

## Files

- `games/forest-dumb/docs/02-disk-format.md` — a short section on what is packed and what is
  not, or a new numbered chapter if a packer turns out to apply.
- `games/forest-dumb/tools/unpack.py` — only if a packed file needs unpacking; its unittest
  beside it.
- `build/` — the unpacked outputs (never committed).

## Interface

`python games/forest-dumb/tools/unpack.py <infile> <outfile>` when it exists. If nothing is
packed, no tool is added and the classification lives in the doc and FINDINGS.

## The check, in order

1. Scan every file for known signatures: PowerPacker (PP20), RNC (method 1 and 2), Imploder,
   LZX, and the rest of the usual set, plus a size/entropy look at the level files.
2. A known packer: take in a decoder under the dependency policy (the `format198x` PowerPacker
   crate, an RNC decoder; DESIGN §5 admits exactly what this task names), unpack to `build/`,
   and cross-check by re-packing where the decoder allows or by comparing against the game's own
   loading behaviour.
3. An unknown packer: the game's own unpacking routine run under the emulator's routine runner is
   the reference (playbook step 2's lesson). Locating that routine in the executable is T06's
   territory; this task then stops with a FINDINGS row naming the signature and the files, and
   the unpacking resumes once T06's map hands back the routine. The unpacked outputs are the
   deliverable either way.

## Tests

- [ ] The signature scanner's unittest: each known header found in synthetic bytes, clean data
      not misclassified.
- [ ] If a decoder lands: a synthetic round-trip test, then the real files unpack with the
      expected sizes and the results re-check against the game's own reading (T03's agreement
      check still green on the unpacked set).

## Done when

- [ ] Every file on both disks is classified packed or not in the doc, with the evidence (the
      signature bytes or their absence).
- [ ] Packed files unpack into `build/` and the classification is a FINDINGS row; if nothing is
      packed, that is the row and the doc section, and no decoder is added.

## Outside actions

- Fetch a crates.io dependency — `worker` (DESIGN §5.3), only if a known packer applies.
