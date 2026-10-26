#!/usr/bin/env python3
"""A guide for `sf-plan --guide-ordered` from an owner's bug recording (CLAUDE.md, "When the
planner struggles"): the frog's cells through the recording, from the level's start, thinned to
one every 2 cells, as `col,row  # step N` lines.

    python3 tools/recording_guide.py BUG.zip [--out GUIDE.txt]

A hand-over recording holds the route's lead-in and then the owner's play, so it is the whole
path; no other trail is spliced in. The trail stops at the first life lost: the respawn puts the
frog back, and an ordered guide that jumps back sends the planner the wrong way. Dying frames are
skipped. The trail and the cell are tools/handover.py's.
"""
import argparse, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from handover import cell, trail

SPACING = 2


def guide(trail_lines):
    """`(lines, stop)`: the guide's lines, and the step a life was first lost (None if never)."""
    out, last, lives, stop = [], None, None, None
    for l in trail_lines:
        k, x, y, _, _, dying, n = map(int, l.split())
        if lives is not None and n < lives:
            stop = k
            break
        lives = n
        if dying:
            continue
        c, r = cell(x, y)
        if last is None or max(abs(c - last[0]), abs(r - last[1])) >= SPACING:
            out.append(f'{c},{r}  # step {k}')
            last = (c, r)
    return out, stop


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('zip')
    ap.add_argument('--out')
    a = ap.parse_args()
    lines, stop = guide(trail(os.path.abspath(a.zip)))
    if not lines:
        sys.exit(f'{a.zip}: no frog trail')
    head = f'# guide from {os.path.basename(a.zip)}' + (f', cut at step {stop} (a life lost)' if stop else '')
    out = a.out or os.path.splitext(os.path.basename(a.zip))[0] + '-guide.txt'
    open(out, 'w').write(head + '\n' + '\n'.join(lines) + '\n')
    print(f'{out}: {len(lines)} points, last {lines[-1].split()[0]}' + (f', cut at step {stop}' if stop else ''))


if __name__ == '__main__':
    main()
