#!/usr/bin/env python3
"""Measure which Rust test files are fast, without running the full set (smoke-tests DESIGN §2.4, §5.2).

Each test file and each crate's inline tests is compiled uncapped, then run alone under a cap of 11 s,
because labelling only needs to know whether it finishes in 10. Prints a TSV on stdout, one row per entry:
project, label key (as in smoke.toml), seconds or "over" or "uncompiled", exit status ("-" when killed).
Python test files are not timed: all of them together take about 33 s and all go in smoke.

A pass may be cut into pieces, because rows already measured are worth keeping:
    smoke_timings.py --stop-after 500 > t.tsv
    smoke_timings.py --stop-after 500 --done t.tsv >> t.tsv
Exit status 3: stopped with entries left. Exit status 4: the disk is nearly full (a cold compile is about
8 GB, and other sessions on this machine share the disk, so filling it would break them too).
"""
import argparse
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

import smoke_core

ROOT = Path(__file__).resolve().parent.parent
MIN_FREE_BYTES = 2 * 1024 ** 3
TAIL_BYTES = 2000


@dataclass(frozen=True)
class Timed:
    project: str
    key: str
    cwd: str               # relative to the repo root
    argv: list[str]


def entries_of(project: smoke_core.Project, workspace: str) -> list[Timed]:
    """Every entry a project's smoke.toml must label. The private helper is used on purpose: the keys
    have to be the ones the check asks for, and a second reading of the file layout could drift."""
    cwd = (project.root + '/' + smoke_core._workspace_prefix(workspace)).rstrip('/')
    entries = []
    for key, target in smoke_core._rust_targets(project, workspace).items():
        argv = ['cargo', 'test', '-q', '-p', target.crate]
        argv += ['--test', target.test] if target.test else list(target.flags)
        entries.append(Timed(project.root, key, cwd, argv))
    return entries


def run_capped(argv: list[str], cwd, cap: float | None, env=None) -> tuple[float | None, int | None, bytes]:
    """(seconds, status, output tail), with both None-able values None when the cap killed it.
    In its own process group so that a child of the entry dies with it. Output goes to a file: a surviving
    child holding a pipe open would block the read past the cap."""
    with tempfile.TemporaryFile() as out:
        start = time.monotonic()
        proc = subprocess.Popen(argv, cwd=cwd, env=env, stdout=out, stderr=subprocess.STDOUT,
                                start_new_session=True)
        seconds, status = None, None
        try:
            status = proc.wait(timeout=cap)
            seconds = time.monotonic() - start
        except subprocess.TimeoutExpired:
            pass
        finally:
            # Also after a normal exit, and on SIGTERM of this script: nothing an entry started may stay.
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            proc.wait()
        out.seek(max(0, out.tell() - TAIL_BYTES))
        return seconds, status, out.read()


def measure(entry: Timed, cap: float, cwd=ROOT, env=None) -> tuple[str, str, bytes]:
    """The row's (seconds, status) as text, and the output worth showing."""
    entry_cwd = Path(cwd) / entry.cwd
    _, status, tail = run_capped(entry.argv + ['--no-run'], entry_cwd, None, env)
    if status != 0:
        return 'uncompiled', str(status), tail
    seconds, status, tail = run_capped(entry.argv, entry_cwd, cap, env)
    if seconds is None:
        return 'over', '-', b''
    return f'{seconds:.1f}', str(status), tail if status else b''


def discover() -> list[Timed]:
    tracked = subprocess.run(['git', 'ls-files'], cwd=ROOT, check=True, capture_output=True,
                             text=True).stdout.splitlines()
    games = sorted({p.split('/')[1] for p in tracked if p.startswith('games/') and p.count('/') >= 2})
    entries = []
    for root, workspace in [('toolkit', '.')] + [(f'games/{g}', 'engine') for g in games]:
        files = [p[len(root) + 1:] for p in tracked if p.startswith(root + '/')]
        sources = {f: (ROOT / root / f).read_text(encoding='utf-8') for f in files
                   if f.endswith('.rs') and '/crates/' in '/' + f}
        entries += entries_of(smoke_core.Project(root, None, files, sources), workspace)
    return entries


def done_keys(path: str | None) -> set[tuple[str, str]]:
    if not path or not os.path.exists(path):
        return set()
    with open(path, encoding='utf-8') as f:
        return {tuple(line.split('\t')[:2]) for line in f}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description='Time each Rust test entry alone, under a cap.')
    parser.add_argument('--cap', type=float, default=11.0, help='seconds an entry may run (default 11)')
    parser.add_argument('--done', help='a TSV of an earlier piece: skip its rows and print no header')
    parser.add_argument('--stop-after', type=float, help='start no new entry after this many seconds')
    args = parser.parse_args(argv)

    # SIGTERM becomes an exception so that run_capped's finally kills the entry's group.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
    env = {**os.environ, 'CARGO_TERM_COLOR': 'never', 'NO_COLOR': '1'}
    done = done_keys(args.done)
    if not args.done:
        print('project\tkey\tseconds\tstatus', flush=True)
    start = time.monotonic()
    todo = [e for e in discover() if (e.project, e.key) not in done]
    for i, entry in enumerate(todo):
        if args.stop_after is not None and time.monotonic() - start > args.stop_after:
            print(f'smoke_timings: stopped, {len(todo) - i} entries left', file=sys.stderr)
            return 3
        if shutil.disk_usage(ROOT).free < MIN_FREE_BYTES:
            print(f'smoke_timings: under 2 GB free, stopped, {len(todo) - i} entries left', file=sys.stderr)
            return 4
        seconds, status, tail = measure(entry, args.cap, env=env)
        print(f'{entry.project}\t{entry.key}\t{seconds}\t{status}', flush=True)
        print(f'smoke_timings: {entry.project} {entry.key}: {seconds} ({status})', file=sys.stderr)
        if tail:
            sys.stderr.write(tail.decode('utf-8', 'replace') + '\n')
    return 0


if __name__ == '__main__':
    sys.exit(main())
