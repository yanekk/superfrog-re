# T01 — ofs-reader

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

The toolkit learns to read AmigaDOS OFS (DOS\0) floppies, the first toolkit change a second game
forces (the playbook's "What the next game still needs"). The owner's dependency policy applies:
take in a good third-party reader behind `amiga-disk`'s own interface, evaluated on the real
disks rather than assumed good (DESIGN §2.2). The tier-2 name check applies: nothing here may
name the game.

## Design sections this implements

DESIGN §2.2, §3.1 (the pure side), §3.2.

## Files

- `toolkit/crates/amiga-disk/src/ofs.rs` — the reader, behind this interface (errors carry byte
  offsets, like the crate's other readers):

```
pub struct OfsImage<'a> { /* the parsed volume over `bytes` */ }
impl<'a> OfsImage<'a> {
    /// Parse an OFS (DOS\0) image. `bytes` is a whole ADF; anything else is ReadError.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ReadError>;
    /// The volume name from the root block.
    pub fn volume_name(&self) -> &str;
    /// The root directory's files: name and size in bytes, in directory order.
    pub fn files(&self) -> impl Iterator<Item = (&'a str, u32)>;
    /// One file's contents by name, matched case-insensitively like dos.library.
    pub fn read_file(&self, name: &str) -> Result<&'a [u8], ReadError>;
}
```

  Contiguous contents only: the game's files are small; a multi-extension-block file returns its
  bytes concatenated, and a non-contiguous layout is the caller's error if it happens.
- `toolkit/crates/amiga-disk/Cargo.toml` — the chosen third-party crate.
- `toolkit/crates/amiga-disk/src/lib.rs` — `pub mod ofs;`.
- `toolkit/crates/toolkit-checks/tests/purity.rs` — the allowlist gains the chosen crate and its
  dependencies, one purity-reason line each (the test currently fails on any unlisted dependency);
  GAME_PREFIX also gains `fd-` (the second game's crate prefix), so a toolkit crate depending on a
  game crate is caught whichever game it is.

## Interface

The block above. Why our interface: callers (fd-tools now, the engine later) stay on one API and
one error style whatever the underlying library, and the allowlist admits exactly what is chosen.

## Evaluation first, then implement

The candidate is `amiga-ffs` 0.4.0 (MIT or Apache-2.0, reads OFS), then `affs-read` 0.5.1 (MIT),
then a hand-written reader cross-checked against amitools if neither passes. The evaluation is a
scratch check, not committed code: list both real disks with the candidate and with amitools'
`xdftool` (in the repo venv), and compare file names and sizes exactly; read a few files with
both and compare bytes. The disks are found where the game's `docs/01-disk-location.md` says;
if the owner has not placed them yet, the evaluation waits on the same copy T00 asks for. Also
read the candidate's source to confirm its parse path does no I/O and its dependency tree is
small enough to justify on the allowlist. The verdict (which crate, why) goes in FINDINGS.md. If
neither crate passes, the hand-written fallback applies; the policy's trigger, a good reader
existing, did not fire.

## Tests

All on synthetic images built in-test from pieces (the toolkit rule: nothing from a game's disks
enters this tree). A small test helper builds OFS blocks by hand: a boot block with a valid
checksum, a root block with its hash chain, file header blocks and data blocks.

- [ ] Parse of a volume with several files lists the names and sizes in order.
- [ ] Read of a single-block file and of a multi-extension-block file returns the exact bytes.
- [ ] A `!EAGLE_SOFT!`-style name (exclamation mark, underscores) lists and reads.
- [ ] Case-insensitive lookup, as dos.library matches.
- [ ] A bad block checksum fails with the byte offset.
- [ ] A truncated image and a non-OFS dos type fail with the offset.
- [ ] The toolkit purity test passes with the new dependency admitted and `fd-` in GAME_PREFIX.

## Done when

- [ ] The interface above exists with the tests green, and the chosen third-party crate is the
      dependency, not the hand-written path, unless the evaluation verdict says why not.
- [ ] The purity allowlist carries the new entries with their reason lines.
- [ ] The evaluation verdict is a FINDINGS row (candidate, comparison result, licence); the name
      check (`tools/check_tier_names.py`) is green.

## Outside actions

- Fetch a crates.io dependency — `worker` (DESIGN §5.3).
