# T01 — knowledge-tiers

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

Establish the three-tier knowledge system the owner settled in draft 6: split the root CLAUDE.md
into tier 1, create the tier-2 and tier-3 homes, move the twelve Claude memory files into their
tiers with each checked against the code first, write the four knowledge rules into the root
CLAUDE.md, and build the name check that makes rule 3 hold without goodwill. The memory directory
ends as a pointer at the repo.

## Design sections this implements

DESIGN §2.2, §2.3.

## Files

- CLAUDE.md — rewrite the top part into tier 1. Keep the installed pir-method text
  ("How we work together" and everything after it) byte-for-byte untouched. The tier-1 part
  holds: the repo line, the tier map (where knowledge lives, one short table), the four rules
  (Interface below), the faithful-bar one line, the disk-safety and branch conventions moved up
  from the project part, and a pointer to games/ for game work. No game facts stay at tier 1.
- knowledge/ (new) — knowledge/KNOWLEDGE.md (index, one line per note) plus the three notes
  moved from memory: self-testing-faithful-bar, owner-controls, pir-midrun-amendment. Each is
  checked against the code and the repo before it moves; a stale claim is corrected and the
  correction noted in the note. No game names in this folder (the check fails them).
- toolkit/CLAUDE.md (new) — tier-2 rules (what the toolkit is, that no game name enters it) and
  a one-line index of toolkit/docs/.
- toolkit/docs/ (new) — the two notes moved from memory: reuse-for-another-game (toolkit history
  and backlog) and explorer-go-explore-idea (backlog). Where they name a game, the line carries
  `learned on:` (DESIGN §2.3). Headed "backlog" where they are plans, not facts.
- games/superfrog/CLAUDE.md (new) — the root CLAUDE.md's project part (Orientation, What's
  known, Tile addressing, Next up, Conventions) minus what moved to tier 1, with every path
  written in the final layout (games/superfrog/engine/, games/superfrog/tools/,
  games/superfrog/docs/ — true once T02 lands, and no session touches these files before then).
- games/superfrog/docs/ (new) — the seven notes moved from memory: atn-decompression,
  t7mp-level-format, collision-and-secrets, milestone-2-bug-recorder, next-plan-difficulty-modes,
  video-ghost-trial, planner-stall-ask-owner. Each checked against docs/ and the code as it
  stands today (old paths — the facts, not the citations; the note cites its sources by final
  paths); backlog items headed "backlog". Indexed in games/superfrog/CLAUDE.md.
- tools/check_tier_names.py, tools/test_tier_names.py (new, at the root; T02 keeps them there).
- tools/empty_memory.sh (new — the §5.3 seatbelt script).

The memory directory is
~/.claude/projects/-Users-jan-krolikowski-src-superfrog-re/memory/ on this machine (verify it
exists; MEMORY.md lists the twelve files).

## Interface

```
tools/check_tier_names.py                    # exit 1, one line per offence: path:line
# Banned tokens, case-insensitive, [-_ ]+ interchangeable (so "forest dumb forever" matches
# "forest-dumb"): every games/*/ directory name, plus the literal prefix token "sf-",
# plus the studio tokens team17 / team 17 (folded in from toolkit's Rust name check).
# Exempt: the repo's own name token "superfrog-re", and any line containing "learned on:"
# anywhere in it (a comment leader ///, //, #, ;, * is ignored). Scanned: toolkit/ and
# knowledge/ only — games/, plans/ and the root CLAUDE.md are not scanned.

tools/empty_memory.sh                        # for each of the twelve memory files: fail loudly
# unless the repo already holds its content (the note file in knowledge/, toolkit/docs/ or
# games/superfrog/docs/), then delete it; finally rewrite MEMORY.md to a pointer at the repo.
# Idempotent: a second run finds no files and rewrites the same pointer.
```

The four rules, as they go into the root CLAUDE.md (the draft's words, flattened):

1. Write at the lowest tier where it is true. A Superfrog fact goes in games/superfrog/, even if
   it might hold elsewhere.
2. Promote when a second user confirms it. A lesson a game note holds that proves true for
   another game moves up to toolkit/ (or tier 1 for a way of working), in the commit that shows
   it.
3. No game names above tier 3. The name check enforces it: toolkit/ and knowledge/ must not name
   a game except in a marked "learned on:" line.
4. Claude's memory keeps only what is about this machine or this conversation; anything a future
   session needs goes in the repo.

## Tests

- [ ] tools/test_tier_names.py: a bare slug, an uppercase name, a hyphen/space variant, "forest
  dumb forever", the `sf-` prefix, team17 / team 17, a "learned on:" line exempt (mid-line
  marker and in a ///
  doc comment), the "superfrog-re" token exempt, knowledge/ scanned, toolkit/ scanned, games/
  not scanned, a synthetic games/<newname>/ directory makes its slug banned, exit codes.
- [ ] The checker passes over this branch (run `venv/bin/python tools/check_tier_names.py`),
  including the six existing "learned on:" lines in toolkit crate comments.
- [ ] The stored test block still passes on this branch (nothing has moved; it is the baseline
  T02 must preserve): `NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py'`,
  `cd engine && cargo test -q --workspace`, `cd toolkit && cargo test -q --workspace`.
- [ ] tools/empty_memory.sh refuses when a note is missing from the repo, and every one of the
  twelve files resolves to a repo note before the real run.

## Done when

- [ ] The tiers exist as above: root CLAUDE.md is tier 1 only with the pir-method text untouched,
  toolkit/CLAUDE.md and toolkit/docs/ hold their notes, games/superfrog/CLAUDE.md and docs/ hold
  the project part and all seven notes — twelve memory files accounted for in the repo, each
  checked against the code, backlog items marked.
- [ ] tools/check_tier_names.py and its unit tests are green, and the checker passes over the
  whole repo.
- [ ] The memory directory holds only MEMORY.md, which points at the repo's knowledge/ with the
  date.

## Outside actions

- empty the machine's memory directory — `worker` (DESIGN §5.3): commit the notes first, then run
  tools/empty_memory.sh. The script's own check is the seatbelt; the way back is copying the
  notes back from the repo.
