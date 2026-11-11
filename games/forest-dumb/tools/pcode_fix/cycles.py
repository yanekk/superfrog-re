"""The cycles each instruction adds to the counter (T11 the fixed part, T12 the data-dependent part).

Nothing is counted yet: `cycles(ins)` returns no lines, and the instruction check reports the cycles
column `not-counted` until a counter exists (fd-conform `judge::insn::translated_cycles`).
"""


def cycles(ins):
    return []
