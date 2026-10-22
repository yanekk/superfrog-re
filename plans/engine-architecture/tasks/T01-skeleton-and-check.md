# T01 — skeleton-and-check

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

Lay down `docs/architecture/` so that sixteen chapter tasks can run at once, each editing only its
own file: a stub per chapter with front matter that already owns every tracked engine file, the
section template, the glossary every chapter uses for the imitation mechanisms, and the drift
check with its helper tool. The glossary matters most: it is how seventeen parallel chapters end
up using the same words for the same mechanism.

## Design sections this implements

DESIGN §2.1–§2.4, §3.1, §3.2.

## Files

- `docs/architecture/README.md` (stub: one paragraph on what this is, links to the glossary,
  template and chapters; T18 completes it)
- `docs/architecture/glossary.md`
- `docs/architecture/chapter-template.md`
- `docs/architecture/{id}.md` for each of the 17 ids in DESIGN §2.3 (stubs)
- `tools/architecture.py`, `tools/test_architecture_docs.py`

## Interface

The front matter is DESIGN §2.4. The stubs carry `kind`, `mix` and `reuse` as `"tbd"`, their
`owns` as below, and a body of one line naming the task that writes it.

Ownership (prefixes under `engine/crates/sf-core/src/` unless written in full; the longest
prefix wins):

| id | owns |
|---|---|
| `memory-model` | `state.rs`, `layout.rs`, `mem.rs` |
| `copied-in-data` | nothing (a view, DESIGN §2.4) |
| `cpu-time-and-beam` | `level/beam.rs`, `level/timeline.rs`, `feed.rs` |
| `frame-interrupts-input` | `level/frame.rs`, `level/clock.rs`, `front/vbl.rs`, `input.rs` |
| `display-hardware` | `render/`, `level/bobs.rs`, `front/blit.rs`, `front/sprite.rs` |
| `sound-chip` | `engine/crates/sf-paula/` |
| `sound-driver` | `audio/`, `engine/crates/sf-core/tests/audio_driver.rs` |
| `game-flow` | `lib.rs`, `game.rs`, `scene.rs`, `rng.rs`, `recording.rs`, `savestate.rs`, `front/chain.rs`, `engine/crates/sf-core/Cargo.toml`, `engine/crates/sf-core/tests/savestate.rs`, `engine/crates/sf-core/tests/start_level.rs` |
| `front-end` | `front/`, `engine/crates/sf-core/tests/front.rs` |
| `story-and-ending` | `front/story/`, `front/ending.rs` |
| `level` | `level/` |
| `frog-and-items` | `frog/`, `items/` |
| `enemies-and-bosses` | `enemies/`, `bosses/` |
| `slot-and-bonus` | `slot/`, `bonus/` |
| `disks-and-assets` | `engine/crates/sf-disk/`, `engine/crates/sf-assets/` |
| `app` | `engine/crates/sf-app/` |
| `verification` | `engine/crates/sf-oracle/`, `engine/crates/sf-conform/`, `engine/crates/sf-e2e/`, `engine/conform/` |

No prefix covers `engine/crates/sf-core/` or `engine/crates/sf-core/src/` whole, on purpose: a new
top-level module must fail the check until a chapter claims it (DESIGN §2.4). If a tracked file
exists that this table misses (one added on `main` since the plan), assign it to the obvious
chapter and log a 📌 row.

`tools/architecture.py` exposes the functions in DESIGN §3.2. `ALLOW_TBD = True` is a module
constant in the test file.

The glossary has one short entry per mechanism, each naming where its code lives and which
chapter documents it: byte-exact state at original addresses, the layout table, transplant and
probe, the loaded hunk image (`Rom`), boot-built tables, cycle charging (`spend`), beam position,
the vertical-blank and copper interrupts, the 25 Hz main-loop pass, the copper run, bitplane
fetch, hardware sprites, the blitter, Paula writes, the CIA keyboard, disk prompts and swaps,
PAL pacing. Plus the four kinds and the reuse scale, by key. Entries are definitions, a sentence
or two each; the chapters carry the detail.

The template lists the deep-chapter sections of DESIGN §2.3 with one line each on what goes in
them, and says which a light chapter may drop.

## Tests

- [ ] the real tree: every tracked file under `engine/crates/` and `engine/conform/` is owned, no
      duplicate or stale prefix, all front matter valid, every cited path exists
- [ ] self-test: an unowned file is reported
- [ ] self-test: two chapters with the identical prefix are reported
- [ ] self-test: a prefix matching no tracked file is reported
- [ ] self-test: longest prefix wins (`front/` vs `front/story/`)
- [ ] self-test: a bad `tier`, `kind` or `reuse` value, and a missing key, are reported
- [ ] self-test: a `mix` summing to 99 is reported; `tbd` passes with `ALLOW_TBD` true and fails
      with it false
- [ ] self-test: a cited missing path is reported; `path.rs:120` and `path.rs::item` resolve to
      `path.rs`; a path with `*` is skipped; a Rust path `sf_core::level` is not treated as a
      repo path
- [ ] self-test: a Markdown file with no front matter (template, glossary, README) is not a chapter
- [ ] self-test: `line_counts` counts `.rs`, `.cpp`, `.h`, `.wgsl` lines and leaves `.txt`, `.toml`,
      `.patch`, `.md` out, reporting them as the separate uncounted total (DESIGN §2.2)
- [ ] `python tools/architecture.py totals` prints one row per chapter plus the uncounted row, and exits 0

## Done when

- [ ] The test command passes with the new test in it, quiet on success.
- [ ] Seventeen stubs exist whose `owns` match the table above; the glossary and template are
      written.
- [ ] Deleting one `owns` entry locally makes the test fail naming the unowned files.
