#!/usr/bin/env python3
"""A guide for `sf-plan --guide-ordered` from an owner's bug recording (CLAUDE.md, "When the
planner struggles"): the frog's cells through the recording, from the level's start, thinned to
one every 2 cells, as `col,row  # step N` lines.

    python3 tools/recording_guide.py BUG.zip [--out GUIDE.txt]

A hand-over recording holds the route's lead-in and then the owner's play, so it is the whole
path; no other trail is spliced in. The trail stops at the first life lost: the respawn puts the
frog back, and an ordered guide that jumps back sends the planner the wrong way. Dying frames are
skipped. The frog comes from the engine example `bug_trail`, run with cargo from engine/.
"""
import argparse, os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SPACING = 2


def cell(x, y):
    """The frog's cell, as `plan_exit` and tools/handover.py count it."""
    return (x + 0x17) // 16, (y - 0x10) // 16


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
    trail = subprocess.run(['cargo', 'run', '-q', '--release', '-p', 'sf-conform', '--example',
                            'bug_trail', '--', os.path.abspath(a.zip)],
                           cwd=os.path.join(ROOT, 'engine'), capture_output=True, text=True, check=True)
    lines, stop = guide(trail.stdout.splitlines())
    if not lines:
        sys.exit(f'{a.zip}: no frog trail')
    head = f'# guide from {os.path.basename(a.zip)}' + (f', cut at step {stop} (a life lost)' if stop else '')
    out = a.out or os.path.splitext(os.path.basename(a.zip))[0] + '-guide.txt'
    open(out, 'w').write(head + '\n' + '\n'.join(lines) + '\n')
    print(f'{out}: {len(lines)} points, last {lines[-1].split()[0]}' + (f', cut at step {stop}' if stop else ''))


if __name__ == '__main__':
    main()
