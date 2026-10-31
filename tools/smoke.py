"""The shell of smoke (smoke-tests DESIGN §2.6, §2.8, §3.1): git, files, processes, the clock, the history.
What a label means and which commands it gives is tools/smoke_core.py.

Each entry runs in a process group of its own, so that a hung entry and whatever it spawned can be killed
without killing the runner. That has a price: pir ends a test line with SIGKILL to the line's group, which
cannot be caught and does not reach the entries. So a watchdog in a session of its own outlives the runner:
the runner tells it each entry's group over a pipe, and when the pipe closes with no `end` line, the runner
is gone and the watchdog kills what is left.
"""
import argparse
import datetime
import os
import shlex
import signal
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field

from smoke_core import Entry, Project, check, run_list

WARN_SECONDS = 15 * 60
LIMIT_SECONDS = 20 * 60

_POLL_SECONDS = 0.02
_SLOWEST = 5
_DEFAULT_HISTORY = '~/.cache/superfrog-re/smoke-times.tsv'

# Unlabelled and always run: they take seconds and guard the machinery itself (DESIGN §2.2).
ROOT_ENTRIES = [
    Entry('root', 'python', '.', ['venv/bin/python', '-m', 'unittest', 'discover', '-q', '-s', 'tools',
                                  '-p', 'test_*.py']),
    Entry('root', 'python', '.', ['venv/bin/python', 'tools/check_tier_names.py']),
]


def _say(line: str) -> None:
    print(line, flush=True)


def _label(entry: Entry) -> str:
    return f'{entry.project}: {shlex.join(entry.argv)}'


def _print_output(output: str) -> None:
    if output:
        print(output, end='' if output.endswith('\n') else '\n', flush=True)


# --- Running the entries ---------------------------------------------------------------------------

@dataclass
class Result:
    status: str            # "ok" | "failed" | "limit"
    seconds: float
    times: list[tuple[Entry, float]] = field(default_factory=list)
    failed: list[Entry] = field(default_factory=list)
    killed: Entry | None = None
    unrun: list[Entry] = field(default_factory=list)


class _Stopped(BaseException):
    """A catchable signal reached the runner. Not an Exception: nothing on the way up may swallow it."""


def run_entries(entries, *, warn_s, limit_s, now, spawn) -> Result:
    """Run the entries in order. `now()` is seconds since smoke's clock started, and `spawn(entry)` returns
    a handle with poll(), kill() and output(). Both are parameters so tests run a limit in a blink."""
    result = Result('ok', 0.0)
    running = None
    project_failed = False
    try:
        for i, entry in enumerate(entries):
            if now() >= limit_s:
                result.unrun = entries[i:]
                break
            started = now()
            running = spawn(entry)
            code = running.poll()
            while code is None and now() < limit_s:
                time.sleep(_POLL_SECONDS)
                code = running.poll()
            if code is None:
                result.killed = entry
                result.unrun = entries[i + 1:]
                # A hang is a failure, and failures print in full (DESIGN §5): this is what says where it hung.
                running.kill()
                _print_output(running.output())
                running = None
                break
            # An entry that exited may have left a child behind, and no entry survives (DESIGN §2.8).
            running.kill()
            output = running.output()
            running = None
            result.times.append((entry, now() - started))
            if code != 0:
                project_failed = True
                result.failed.append(entry)
                _say(f'smoke: FAILED, exit {code}: {_label(entry)}')
                _print_output(output)
            if i + 1 == len(entries) or entries[i + 1].project != entry.project:
                _say(f'smoke: {entry.project} {"FAILED" if project_failed else "ok"} at {now():.0f} s')
                project_failed = False
    finally:
        if running is not None:
            running.kill()
    result.seconds = now()
    if result.killed or result.unrun:
        result.status = 'limit'
    elif result.failed:
        result.status = 'failed'
    _summarise(result, warn_s, limit_s)
    return result


def _summarise(result: Result, warn_s, limit_s) -> None:
    seconds = f'{result.seconds:.0f} s'
    if result.status == 'limit':
        _say(f'smoke: FAILED, over the limit of {limit_s:.0f} s, at {seconds}')
        if result.killed:
            _say(f'smoke: killed: {_label(result.killed)}')
        for entry in result.unrun:
            _say(f'smoke: did not run: {_label(entry)}')
    elif result.status == 'failed':
        _say(f'smoke: FAILED in {seconds}, {len(result.failed)} of {len(result.times)} entries')
    else:
        _say(f'smoke: ok in {seconds}')
    if result.status != 'limit' and result.seconds > warn_s:
        _say(f'smoke: WARNING: took {seconds}, over the {warn_s:.0f} s budget; it fails at {limit_s:.0f} s')
    for entry, took in sorted(result.times, key=lambda t: -t[1])[:_SLOWEST]:
        _say(f'smoke:   {took:6.1f} s  {_label(entry)}')


# --- Processes -------------------------------------------------------------------------------------

def _kill_group(pgid: int) -> None:
    try:
        os.killpg(pgid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass  # gone already; macOS says EPERM for a group holding only zombies


class _Running:
    def __init__(self, proc, capture, watchdog):
        self.proc, self.capture, self.watchdog = proc, capture, watchdog
        self.text = ''

    def poll(self):
        return self.proc.poll()

    def kill(self):
        _kill_group(self.proc.pid)
        self.proc.wait()
        if self.watchdog:
            self.watchdog.tell(f'-{self.proc.pid}')
        # Once only: a signal can land between this and the runner forgetting the entry, and it kills again.
        capture, self.capture = self.capture, None
        if capture:
            capture.seek(0)
            self.text = capture.read().decode(errors='replace')
            capture.close()

    def output(self) -> str:
        return self.text


# Holds the entry until the watchdog knows its group: a kill landing just after the spawn then either closes
# the gate, and the entry exits unstarted, or finds the group already told. Its stdin is the gate, so the
# entry gets /dev/null, as nothing in a test run may wait on input.
_GATE = 'read _ && exec "$@" </dev/null'


def make_spawn(watchdog=None, verbose=False):
    env = {**os.environ, 'NO_COLOR': '1', 'CARGO_TERM_COLOR': 'never'}

    def spawn(entry: Entry) -> _Running:
        argv = [a for a in entry.argv if a != '-q'] if verbose else entry.argv
        # An unnamed file: on a pipe a chatty entry would block, and it leaves nothing behind however we die.
        capture = None if verbose else tempfile.TemporaryFile()
        proc = subprocess.Popen(['sh', '-c', _GATE, 'sh', *argv], cwd=entry.cwd, env=env,
                                stdin=subprocess.PIPE, stdout=capture, stderr=capture and subprocess.STDOUT,
                                process_group=0)
        if watchdog:
            watchdog.tell(f'+{proc.pid}')
        proc.stdin.write(b'\n')
        proc.stdin.close()
        return _Running(proc, capture, watchdog)

    return spawn


class _Watchdog:
    def __init__(self, history: str, commit: str, started: float):
        self.proc = subprocess.Popen(
            [sys.executable, os.path.abspath(__file__), '--watchdog', history, commit, str(started)],
            stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            start_new_session=True, text=True)

    def tell(self, line: str) -> None:
        try:
            self.proc.stdin.write(line + '\n')
            self.proc.stdin.flush()
        except OSError:
            pass  # it died; the run itself is still good

    def finish(self) -> None:
        self.tell('end')
        try:
            self.proc.stdin.close()
        except OSError:
            pass  # it died, and close flushes what tell could not; a green run must not read as a crash
        self.proc.wait()  # it exits on the closed pipe; the run leaves nothing behind


def _watchdog_main(history: str, commit: str, started: str) -> int:
    """Read the runner's pipe to its end. Started by _Watchdog; DESIGN §2.8 is the reason it exists."""
    signal.signal(signal.SIGINT, signal.SIG_IGN)
    groups = set()
    ended = False
    for line in sys.stdin:
        line = line.strip()
        if line == 'end':
            ended = True
        elif line.startswith('+'):
            groups.add(int(line[1:]))
        elif line.startswith('-'):
            groups.discard(int(line[1:]))
    for pgid in groups:
        _kill_group(pgid)
    if not ended:
        try:
            _append_history(history, commit, time.time() - float(started), 'failed')
        except OSError:
            pass  # nowhere to say so, and the history is not evidence about the code
    return 0


# --- The history -----------------------------------------------------------------------------------

def _append_history(path: str, commit: str, seconds: float, status: str) -> None:
    date = datetime.datetime.now().astimezone().isoformat(timespec='seconds')
    row = f'{date}\t{commit}\t{seconds:.0f}\t{status}\n'
    folder = os.path.dirname(path)
    if folder:
        os.makedirs(folder, exist_ok=True)
    # One write in append mode, so that two runs at once cannot garble each other's lines.
    fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
    try:
        os.write(fd, row.encode())
    finally:
        os.close(fd)


def _history_path() -> str:
    return os.environ.get('SMOKE_TIMES_FILE') or os.path.expanduser(_DEFAULT_HISTORY)


def _commit() -> str:
    run = subprocess.run(['git', 'rev-parse', '--short', 'HEAD'], capture_output=True, text=True)
    return run.stdout.strip() if run.returncode == 0 else 'unknown'


def _record(path: str, commit: str, seconds: float, status: str) -> None:
    try:
        _append_history(path, commit, seconds, status)
    except OSError as e:
        _say(f'smoke: could not write the history to {path}: {e}')


# --- Gathering the projects ------------------------------------------------------------------------

def _is_source(path: str) -> bool:
    return path.endswith('.rs') and ('/src/' in path or '/tests/' in path)


def _project(root: str) -> Project:
    listed = subprocess.run(['git', 'ls-files', '-z', '--', root], capture_output=True, check=True)
    prefix = root + '/'
    # A tracked file deleted in this worktree is gone as far as the run can tell.
    files = [p.decode()[len(prefix):] for p in listed.stdout.split(b'\0')
             if p and os.path.isfile(p.decode())]
    sources = {}
    for path in files:
        if _is_source(path):
            with open(prefix + path, encoding='utf-8', errors='replace') as f:
                sources[path] = f.read()
    manifest = None
    if os.path.isfile(prefix + 'smoke.toml'):
        with open(prefix + 'smoke.toml', encoding='utf-8') as f:
            manifest = f.read()
    return Project(root, manifest, files, sources)


def gather_projects() -> list[Project]:
    # Every folder under games/, as tools/test.sh finds them, so that a new game cannot be missed.
    games = sorted(d for d in os.listdir('games') if os.path.isdir(os.path.join('games', d)))
    return [_project(root) for root in ['toolkit', *(f'games/{g}' for g in games)]]


def _problems(projects) -> list[str]:
    return [problem for project in projects for problem in check(project)]


def _report_problems(problems) -> None:
    for problem in problems:
        _say(f'smoke: {problem}')
    _say(f'smoke: FAILED, the label check found {len(problems)} problems; no test ran')


def _entries(projects) -> list[Entry]:
    return [*ROOT_ENTRIES, *(e for project in projects for e in run_list(project))]


# --- The command -----------------------------------------------------------------------------------

def _raise_stopped(signum, _frame):
    raise _Stopped(signum)


def _run(started_mono: float, started_wall: float) -> int:
    history, commit = _history_path(), _commit()
    now = lambda: time.monotonic() - started_mono  # noqa: E731
    watchdog = _Watchdog(history, commit, started_wall)
    status = 'failed'
    try:
        projects = gather_projects()
        problems = _problems(projects)
        if problems:
            _report_problems(problems)
            return 1
        spawn = make_spawn(watchdog, verbose=os.environ.get('SMOKE_VERBOSE') == '1')
        result = run_entries(_entries(projects), warn_s=WARN_SECONDS, limit_s=LIMIT_SECONDS, now=now,
                             spawn=spawn)
        status = result.status
        return 0 if status == 'ok' else 1
    except _Stopped as stop:
        _say(f'smoke: FAILED, stopped by signal {stop.args[0]}')
        return 1
    finally:
        watchdog.finish()
        _record(history, commit, now(), status)


def main(argv=None) -> int:
    started_mono, started_wall = time.monotonic(), time.time()
    args = sys.argv[1:] if argv is None else argv
    if args[:1] == ['--watchdog']:
        return _watchdog_main(*args[1:])
    parser = argparse.ArgumentParser(prog='tools/smoke.sh')
    parser.add_argument('--check-only', action='store_true', help='run the label check alone')
    parser.add_argument('--list', action='store_true', help='print the entries and run nothing')
    options = parser.parse_args(args)

    os.chdir(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    if options.check_only or options.list:
        projects = gather_projects()
        problems = _problems(projects)
        if problems:
            _report_problems(problems)
            return 1
        if options.list:
            for entry in _entries(projects):
                print(f'{entry.project}\t{entry.cwd}\t{shlex.join(entry.argv)}')
        else:
            _say('smoke: the label check is clean')
        return 0

    for signum in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(signum, _raise_stopped)
    return _run(started_mono, started_wall)


if __name__ == '__main__':
    sys.exit(main())
