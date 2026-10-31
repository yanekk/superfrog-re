# Implementation plan

10 tasks in 6 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- **The oracle spike is the gate.** Phase 0 proves the emulator boots the disks headlessly,
  repeatably and inspectably before anything is built on top; without it "faithful" is a claim
  nobody can check (playbook step 5's lesson).
- **The playbook's opening steps in order.** Phases 1 to 5 walk the playbook's steps 1 to 5:
  disks and boot, packer, file formats, executable map, then the oracle and the first case. The
  next plans take over at step 6.
- **Understand before porting.** The map task runs before the one modelled variable is ported, so
  the first case starts from a shared map instead of rediscovering the title path.
- **The measuring machine before the measurement counts.** The frame parameterisation and the
  harness land before the first case is judged, so the case is checked by the same machinery the
  whole rebuild will use.

```
Phase 0  ▸  T00                    prove the oracle            throwaway
Phase 1  ▸  T01 … T03              disks and boot              OFS reader, import, boot chain
Phase 2  ▸  T04                    the packer                  classify, decode if packed
Phase 3  ▸  T05                    the file formats            render everything
Phase 4  ▸  T06                    the executable map          shared map, variable choice
Phase 5  ▸  T07 … T09              the measuring machine       frame, stub, first case
```

---

## Phase 0 — Prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-oracle-spike.md) | oracle-spike | — |

**T00 gates the whole plan.** If the two disks boot headlessly, repeatably, with per-frame input
and readable memory, picture and sound, the rest proceeds with the measured answers (the
Kickstart, the drive arrangement, the frame length, the boot time to the title). If they do not
boot, or a run is not repeatable and no configuration fixes it, the plan stops and the owner
decides what changes. The spike also measures the frame length and the boot-to-title path that
T07, T08 and T09 consume. It is throwaway code, deleted afterwards; only its answers (dated
FINDINGS rows) stay.

## Phase 1 — Disks and boot

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-ofs-reader.md) | ofs-reader | — |
| [T02](tasks/T02-fd-import.md) | fd-import | T01 |
| [T03](tasks/T03-fd-boot-chain.md) | fd-boot-chain | T00, T02 |

At the end: the toolkit reads OFS floppies behind its own interface, every file of both disks
sits in `build/` with a committed inventory, and the boot chain is documented and cross-checked
against the inventory.

## Phase 2 — The packer

| # | Task | Depends on |
|---|---|---|
| [T04](tasks/T04-fd-pack-check.md) | fd-pack-check | T02 |

At the end: every file is classified packed or not, with evidence, and unpacked where it is
packed.

## Phase 3 — The file formats

| # | Task | Depends on |
|---|---|---|
| [T05](tasks/T05-fd-file-formats.md) | fd-file-formats | T02, T04 |

At the end: the level and graphics formats are documented and proven by rendering, and a person
and a session point at the same spot with `<level-id>:<col>,<row>`.

## Phase 4 — The executable map

| # | Task | Depends on |
|---|---|---|
| [T06](tasks/T06-fd-exe-map.md) | fd-exe-map | T02, T03 |

At the end: one shared map of the main program exists, and the variable the first case models is
chosen with evidence. T06 may amend T08's and T09's docs if its variable choice demands it; that
is an amendment to this plan, made with the owner.

## Phase 5 — The measuring machine

| # | Task | Depends on |
|---|---|---|
| [T07](tasks/T07-frame-parameterise.md) | frame-parameterise | T00 |
| [T08](tasks/T08-fd-core.md) | fd-core | T00, T06, T07 |
| [T09](tasks/T09-fd-first-case.md) | fd-first-case | T00, T02, T03, T08 |

At the end: the toolkit's frame length is a value a game supplies, Forest Dumb's stub models one
title-screen variable, and the first conformance case compares it with the original in the
emulator and passes in the game's test suite.

---

## Critical path

```
T01 → T02 → T03 → T06 → T08 → T09
```

Off it: T00 (the parallel gate), T04, T05, T07, which slot in wherever convenient.

Leaves: T09 is the final deliverable. T05 (file formats) is terminal within this plan because its
consumer is the next plan's porting work; the formats are proven by rendering here, and the Rust
readers land where they are consumed.

## Parallel width

10 tasks, longest dependency chain 6, up to 4 can run at once (T00 and T01 at the start; T03,
T04 and T07 once T02 is done, then T06 once T03 is). The width is modest on purpose: the plan is a discovery
chain, and each phase's answers feed the next.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T05 file-formats, T06 exe-map |
| **Medium** | T00 oracle-spike, T01 ofs-reader, T02 fd-import, T07 frame-parameterise, T08 fd-core, T09 fd-first-case |
| **Light** | T03 fd-boot-chain, T04 fd-pack-check |

The places this will overrun: T05 (the formats are the biggest unknown; the renderer must cover
every level and sheet), T06 (the map's depth is open-ended and the variable choice gates T08 and
T09), T01 (the third-party evaluation may surprise, which is why the fallback order is written
in), and T09 (a first case run through a real boot can be flaky until the sync point is settled).

## Decisions still open

Nothing blocks the first task. Three things are deliberately left for their tasks to settle, each
with its rule written in DESIGN §2: the exact modelled variable and its address (T06, with the
criteria of §2.8; the owner is asked only if no candidate qualifies), the exact sync frame and
input of the first case (T00's measurement), and which packer decoder applies, if any (T04, under
the dependency policy of DESIGN §5).
