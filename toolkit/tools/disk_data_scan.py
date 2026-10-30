"""Finds runs of source bytes in other files: the scan engine of a game's no-disk-data check.

A game's driver supplies the sources ((label, bytes) pairs, whatever it decodes from its disks) and
the files to check. Every source is indexed at a 64-byte stride, keeping every place a block occurs;
each file is scanned at a 1-byte stride, so any copied run of 127 bytes or more contains one indexed
block. A hit is extended byte by byte against each place and reported if the longest shared run is
128 bytes or longer. Blocks with fewer than 4 distinct byte values (zero fill, solid colour) are not
indexed: they are not data anyone could recover.
"""
import os, subprocess

BLOCK = 64
MIN_RUN = 128
MIN_DISTINCT = 4


def build_index(sources):
    index = {}
    for si, (_label, data) in enumerate(sources):
        for off in range(0, len(data) - BLOCK + 1, BLOCK):
            block = data[off:off + BLOCK]
            if len(set(block)) >= MIN_DISTINCT:
                index.setdefault(block, []).append((si, off))
    return index


def scan(data, index, sources):
    """[(file offset, run length, source label, source offset)] for every shared run >= MIN_RUN."""
    hits = []
    pos = 0
    n = len(data)
    while pos + BLOCK <= n:
        refs = index.get(data[pos:pos + BLOCK])
        if refs is None:
            pos += 1
            continue
        # The block may occur at several places on the disks; measure the run against each.
        run, back, fwd, si, soff = 0, 0, 0, 0, 0
        for rsi, rsoff in refs:
            src = sources[rsi][1]
            b = 0
            while pos - b > 0 and rsoff - b > 0 and data[pos - b - 1] == src[rsoff - b - 1]:
                b += 1
            f = BLOCK
            while pos + f < n and rsoff + f < len(src) and data[pos + f] == src[rsoff + f]:
                f += 1
            if b + f > run:
                run, back, fwd, si, soff = b + f, b, f, rsi, rsoff
        if run >= MIN_RUN:
            hits.append((pos - back, run, sources[si][0], soff - back))
            pos += fwd
        else:
            pos += 1
    return hits


def repo_root(start):
    return subprocess.run(['git', 'rev-parse', '--show-toplevel'], cwd=start,
                          capture_output=True, text=True, check=True).stdout.strip()


def repo_files(root):
    """Every file git would commit: tracked, plus untracked files that are not ignored."""
    out = subprocess.run(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'],
                         cwd=root, capture_output=True, check=True).stdout
    return [os.path.join(root, p) for p in out.decode().split('\0') if p]
