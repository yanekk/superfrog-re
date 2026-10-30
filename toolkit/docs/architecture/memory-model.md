+++
id = "memory-model"
title = "The memory model"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 100, game = 0, data = 0, shell = 0 }
reuse = "as-is"
owns = [
  "toolkit/crates/amiga-chips/src/memory.rs",
]
+++

# The memory model

## 1. What it is

The method: every original variable an engine models is kept as the original's bytes at the original's
address, and ported 68000 routines read and write it by address, big-endian, as the 68000 does. The
toolkit holds only the machine's memory picture, in `toolkit/crates/amiga-chips/src/memory.rs`: the
`Ram` trait, `AmigaMemory` (the A500's chip RAM and slow RAM) with `byte_at`, `word` and `long`, and
`SLOW_BASE` (`$C00000`). A read off the map is none. The state store, its variable table and the bus
are the game's.

## 2. Kind mix

`amiga` 100.

## 3. Why byte-exact

68000 code treats memory as memory. Its reads overlay widths (a long where it wrote two words, a byte
of a word it wrote whole). Its variables hold absolute addresses into its own tables, so those tables
have to sit at their original addresses. It may read and write through null pointers, so low memory
and vectors matter. It walks runs of records by stride. Writing all that in native types means finding
every such use first. Keeping the bytes makes a port line for line and makes starting from the
original's memory (transplant) and reading values back (probe) a copy.

## 4. The coupling a native rewrite meets

A check against the original needs, at each compared moment, the engine's value for every compared
variable as the original would hold it, and a way to start the engine from the original's memory. A
rewrite that keeps values in native types breaks both. Its routes, which can be mixed per subsystem:

| Route | Cost |
|---|---|
| Keep a mapping layer: the byte store behind typed accessors | Least change to the checks; the engine stays byte-shaped underneath |
| A probe adapter: native state, projected to original addresses, with the inverse for transplant | The checks keep their power. Every field needs a reader and a writer, including pointers and overlaid fields; a mistake shows as a divergence, so the checks also test the adapter |
| Change the checks: compare in native terms | No adapter, but state and routine comparison lose address-level precision, so a divergence is found later and named less exactly |

## 5. What must be preserved

| Behaviour to keep | Check that proves it |
|---|---|
| Reads big-endian at chip and slow addresses, none off the map | `amiga_memory_reads_0_or_none_off_the_map` (`toolkit/crates/amiga-chips/src/memory.rs`), `memory_reads_big_endian_at_chip_and_slow_addresses` (`toolkit/crates/amiga-oracle/src/amiga.rs`) |

## 6. Reuse

**`as-is`** for this file. The approach (byte store, table-driven transplant and probe, a bus over the
loaded image) carries to any Amiga game whose port is checked against the original by memory; the
table and the hunk bases are per game.

## 7. Sources

`toolkit/crates/amiga-chips/src/memory.rs`.
