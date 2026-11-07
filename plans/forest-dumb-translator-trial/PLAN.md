# Implementation plan

9 tasks in 3 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- Level 1's work is brought in first (T00), because the judge, the generators and the inventory the trial measures with exist only on its branch.
- The parts are built and tested on made-up programs before any game routine is judged (phase 1), so the trial's failures are about the game and Ghidra's P-code, not about untested plumbing.
- The trial's real run and its fixes (T06, T07) are separate from the report (T08), so the session that fixes the translator is not also the one that reads the verdict to the owner.

```
Phase 0  ▸  T00                    bring level 1 in
Phase 1  ▸  T01 T02 T03 T04 T05    export, runtime, translator, judge mode, code-writes check
Phase 2  ▸  T06 T07 T08            trial run, general fixes, report and the owner's go
```

## Phase 0 — Ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-bring-in-level-1.md) | bring-in-level-1 | — |

At the end: the plan branch holds level 1's work as of that day, and the judge still passes one of Muse's ports.

## Phase 1 — The parts

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-pcode-export.md) | pcode-export | T00 |
| [T02](tasks/T02-pcode-runtime.md) | pcode-runtime | T00 |
| [T03](tasks/T03-pcode-translator.md) | pcode-translator | T02 |
| [T04](tasks/T04-judge-translated.md) | judge-translated | T00, T02 |
| [T05](tasks/T05-code-writes-check.md) | code-writes-check | T00 |

At the end: the game's tree exports from Ghidra, made-up programs translate and run correctly, the judge can judge a translated tree, and level 1's path has been checked for writes into code.

## Phase 2 — The trial

| # | Task | Depends on |
|---|---|---|
| [T06](tasks/T06-trial-tool.md) | trial-tool | T01, T03, T04 |
| [T07](tasks/T07-trial-fixes.md) | trial-fixes | T06 |
| [T08](tasks/T08-trial-report.md) | trial-report | T05, T07 |

At the end: the report is committed and the owner's go or no-go is recorded.

---

## Critical path

```
T00 → T02 → T03 → T06 → T07 → T08
```

T01, T04 and T05 are off it. T02 waits on T00 although it uses none of level 1's code, because it registers its crate in `purity.rs` and `smoke.toml`, which level 1 changed; built beside T00 it would conflict at merge. Leaves: T08 only.

## Parallel width

9 tasks · longest dependency chain 6 · up to 4 could run at once (T01, T03, T04, T05).

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T03, T07 |
| **Medium** | T01, T02, T04, T06 |
| **Light** | T00, T05, T08 |

T07 is the unknown: its size is however many kinds of failure the real routines show. Its worker asks the owner to add a task rather than run one session past about 200k context (level 1 DESIGN §4.4); each such follow-up needs the owner's own yes, not the coordinator agent's (DESIGN §7). T01 carries the address-mapping risk (DESIGN §3.4): relocations come from the program file, since Ghidra records none; if they do not map cleanly onto operands, T01 asks the owner.

## Decisions still open

None block the build. The second plan's questions (who may run the result, what is rewritten by hand, the table bar for generated code) wait for the go.
