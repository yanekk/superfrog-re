"""tools/smoke.sh and tools/smoke.py (smoke-tests DESIGN §2.6, §2.8, §3.2): limits, kills and the history, on
fake entries with limits of a fraction of a second.

The command tests run a copy of the scripts in a throwaway repo with fake tests and a fake cargo. They never
run the real root entries: those discover this file, and would recurse.
"""
import contextlib
import io
import os
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import smoke  # noqa: E402
from smoke_core import Entry  # noqa: E402

PASS_TEST = 'import unittest\nclass T(unittest.TestCase):\n    def test_a(self):\n        pass\n'
FAIL_TEST = 'import unittest\nclass T(unittest.TestCase):\n    def test_a(self):\n        self.fail("fake break")\n'
# Records itself and a child it spawned, then hangs; the test kills from outside.
HANG_TEST = '''import os, subprocess, time, unittest
class T(unittest.TestCase):
    def test_a(self):
        child = subprocess.Popen(['sleep', '30'])
        d = os.environ['SMOKE_FAKE_DIR']
        with open(d + '/pids.tmp', 'w') as f:
            f.write(f'{os.getpid()} {child.pid}')
        os.rename(d + '/pids.tmp', d + '/pids')
        time.sleep(30)
'''
FAKE_CARGO = '#!/bin/sh\necho "$PWD $*" >> "$SMOKE_FAKE_DIR/cargo.log"\n'


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


def wait_until(condition, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if condition():
            return True
        time.sleep(0.02)
    return condition()


def sh(script):
    return ['sh', '-c', script]


class FakeHandle:
    def __init__(self, code):
        self.code = code

    def poll(self):
        return self.code

    def kill(self):
        pass

    def output(self):
        return 'fake output\n'


class RunEntries(unittest.TestCase):
    """run_entries with a fake clock and fake processes, where no real process is the point."""

    def run_fake(self, codes, step, warn_s=100, limit_s=1000):
        clock = [0.0]
        spawned = []

        def spawn(entry):
            spawned.append(entry)
            clock[0] += step
            return FakeHandle(codes[len(spawned) - 1])

        entries = [Entry('p', 'python', '.', [f'e{i}']) for i in range(len(codes))]
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            result = smoke.run_entries(entries, warn_s=warn_s, limit_s=limit_s, now=lambda: clock[0], spawn=spawn)
        return result, out.getvalue(), spawned

    def test_all_pass_prints_the_summary(self):
        result, out, _ = self.run_fake([0] * 7, 1)
        self.assertEqual(result.status, 'ok')
        self.assertIn('smoke: p ok', out)
        self.assertIn('smoke: ok in 7 s', out)
        self.assertEqual(sum(line.startswith('smoke:   ') for line in out.splitlines()), 5)
        self.assertNotIn('fake output', out)

    def test_one_failure_runs_the_rest_and_prints_its_output(self):
        result, out, spawned = self.run_fake([0, 3, 0], 1)
        self.assertEqual(result.status, 'failed')
        self.assertEqual(len(spawned), 3)
        self.assertEqual([e.argv for e in result.failed], [['e1']])
        self.assertIn('fake output', out)

    def test_past_the_warn_limit_warns_and_passes(self):
        result, out, _ = self.run_fake([0, 0], 60, warn_s=100)
        self.assertEqual(result.status, 'ok')
        warning = next(line for line in out.splitlines() if line.startswith('smoke: WARNING'))
        self.assertIn('120 s', warning)
        self.assertIn('100 s', warning)

    def test_at_the_warn_limit_exactly_does_not_warn(self):
        _, out, _ = self.run_fake([0, 0], 50, warn_s=100)
        self.assertNotIn('WARNING', out)

    def test_a_limit_reached_between_entries_names_the_unrun(self):
        result, out, spawned = self.run_fake([0, 0, 0], 60, limit_s=100)
        self.assertEqual(result.status, 'limit')
        self.assertEqual(len(spawned), 2)
        self.assertIn('smoke: FAILED, over the limit', out)
        self.assertIn('did not run: p: e2', out)

    def test_the_limit_constants_are_the_design_numbers(self):
        self.assertEqual((smoke.WARN_SECONDS, smoke.LIMIT_SECONDS), (900, 1200))


class RealProcesses(unittest.TestCase):
    """run_entries with real entries: what a kill reaches can only be shown on real processes."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir, True)

    def run_real(self, entries, limit_s, now=None):
        started = time.monotonic()
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            result = smoke.run_entries(entries, warn_s=limit_s, limit_s=limit_s,
                                       now=now or (lambda: time.monotonic() - started), spawn=smoke.make_spawn())
        return result, out.getvalue()

    def test_the_limit_kills_the_entry_and_its_child_and_names_the_unrun(self):
        hang = Entry('p', 'python', self.dir, sh('sleep 30 & echo $! > child; echo $$ > parent.tmp; mv parent.tmp parent; wait'))
        unrun = Entry('p', 'python', self.dir, sh('touch unrun'))
        # The limit falls once the entry has written both pids, so a busy machine cannot trip it before then.
        parent = os.path.join(self.dir, 'parent')
        result, out = self.run_real([hang, unrun], 0.5, now=lambda: 1.0 if os.path.exists(parent) else 0.0)
        self.assertEqual(result.status, 'limit')
        self.assertEqual(result.unrun, [unrun])
        self.assertIn('did not run: p: sh -c', out)
        self.assertFalse(os.path.exists(os.path.join(self.dir, 'unrun')))
        for name in ('parent', 'child'):
            with open(os.path.join(self.dir, name)) as f:
                pid = int(f.read())
            self.assertTrue(wait_until(lambda: not alive(pid)), name)

    def test_a_child_left_behind_by_an_entry_that_exited_is_killed(self):
        leaves = Entry('p', 'python', self.dir, sh('sleep 30 >/dev/null 2>&1 & echo $! > child'))
        result, _ = self.run_real([leaves], 20)
        self.assertEqual(result.status, 'ok')
        with open(os.path.join(self.dir, 'child')) as f:
            pid = int(f.read())
        self.assertTrue(wait_until(lambda: not alive(pid)))

    def test_a_failed_entrys_output_prints_in_full(self):
        result, out = self.run_real([Entry('p', 'python', self.dir, sh('echo the reason; exit 3'))], 20)
        self.assertEqual(result.status, 'failed')
        self.assertIn('exit 3', out)
        self.assertIn('the reason', out)

    def test_a_missing_program_is_a_failed_entry(self):
        result, _ = self.run_real([Entry('p', 'python', self.dir, ['no-such-program-here'])], 20)
        self.assertEqual(result.status, 'failed')

    def test_an_entry_reads_no_stdin(self):
        result, _ = self.run_real([Entry('p', 'python', self.dir, sh('cat'))], 20)
        self.assertEqual(result.status, 'ok')


class Command(unittest.TestCase):
    """tools/smoke.sh in a throwaway repo."""

    def setUp(self):
        self.base = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.base, True)
        self.repo = os.path.join(self.base, 'repo')
        self.fake = os.path.join(self.base, 'fake')
        self.home = os.path.join(self.base, 'home')
        self.history = os.path.join(self.base, 'times.tsv')
        os.makedirs(self.fake)
        os.makedirs(self.home)
        self.build_repo()

    def write(self, path, text):
        full = os.path.join(self.repo, path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, 'w') as f:
            f.write(text)

    def build_repo(self, toolkit_test=PASS_TEST):
        os.makedirs(os.path.join(self.repo, 'tools'))
        for name in ('smoke.sh', 'smoke.py', 'smoke_core.py'):
            shutil.copy(os.path.join(HERE, name), os.path.join(self.repo, 'tools', name))
        os.symlink(os.path.join(os.path.dirname(HERE), 'venv'), os.path.join(self.repo, 'venv'))
        self.write('.gitignore', 'venv\n__pycache__/\n')
        self.write('tools/test_root_fake.py', PASS_TEST)
        self.write('tools/check_tier_names.py', 'print("tier names fake")\n')
        self.write('toolkit/tools/test_kit.py', toolkit_test)
        self.write('toolkit/smoke.toml', '''workspace = "."
[python]
"tools/test_kit.py" = "smoke"
[[area]]
name = "kit"
covers = ["tools/test_kit.py"]
''')
        self.write('games/demo/engine/crates/x/tests/quick.rs', '#[test]\nfn a() {}\n')
        self.write('games/demo/tools/test_demo.py', PASS_TEST)
        self.write('games/demo/smoke.toml', '''workspace = "engine"
[tests]
"crates/x/tests/quick.rs" = "smoke"
[python]
"tools/test_demo.py" = "smoke"
[[area]]
name = "demo"
covers = ["crates/x/tests/quick.rs"]
''')
        cargo = os.path.join(self.fake, 'cargo')
        with open(cargo, 'w') as f:
            f.write(FAKE_CARGO)
        os.chmod(cargo, os.stat(cargo).st_mode | stat.S_IEXEC)
        self.git('init', '-q')
        self.commit()

    def git(self, *args):
        subprocess.run(['git', '-c', 'user.name=t', '-c', 'user.email=t@t', *args], cwd=self.repo, check=True,
                       env=self.env(), capture_output=True)

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-q', '-m', 'fake')

    def env(self, **extra):
        env = {k: v for k, v in os.environ.items()
               if not k.startswith('GIT_') and k not in ('SMOKE_VERBOSE', 'SMOKE_TIMES_FILE')}
        env.update(PATH=self.fake + os.pathsep + env['PATH'], HOME=self.home, SMOKE_TIMES_FILE=self.history,
                   SMOKE_FAKE_DIR=self.fake)
        env.update(extra)
        return env

    def smoke(self, *args, **env):
        return subprocess.run([os.path.join(self.repo, 'tools', 'smoke.sh'), *args], cwd=self.base,
                              env=self.env(**env), capture_output=True, text=True, timeout=300)

    def cargo_log(self):
        path = os.path.join(self.fake, 'cargo.log')
        if not os.path.exists(path):
            return []
        with open(path) as f:
            return f.read().splitlines()

    def rows(self):
        if not os.path.exists(self.history):
            return []
        with open(self.history) as f:
            return [line.split('\t') for line in f.read().splitlines()]

    def test_all_pass(self):
        run = self.smoke()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        lines = run.stdout.splitlines()
        self.assertEqual([line.split()[1] for line in lines[:3]], ['root', 'toolkit', 'games/demo'])
        self.assertTrue(any(line.startswith('smoke: ok in ') for line in lines))
        rows = self.rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual(len(rows[0]), 4)
        self.assertEqual(rows[0][3], 'ok')
        head = subprocess.run(['git', 'rev-parse', '--short', 'HEAD'], cwd=self.repo, env=self.env(),
                              capture_output=True, text=True).stdout.strip()
        self.assertEqual(rows[0][1], head)
        self.assertTrue(rows[0][2].isdigit())

    def test_the_rust_entry_runs_from_the_workspace_and_is_quiet(self):
        self.smoke()
        log = self.cargo_log()
        self.assertEqual(len(log), 1)
        self.assertTrue(log[0].endswith('games/demo/engine test -q -p x --test quick'), log[0])

    def test_verbose_drops_the_quiet_flag(self):
        run = self.smoke(SMOKE_VERBOSE='1')
        self.assertEqual(run.returncode, 0)
        self.assertTrue(self.cargo_log()[0].endswith('test -p x --test quick'))

    def test_a_check_problem_runs_no_entry(self):
        os.remove(os.path.join(self.repo, 'games/demo/smoke.toml'))
        self.commit()
        run = self.smoke()
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('games/demo/smoke.toml: missing', run.stdout)
        self.assertEqual(self.cargo_log(), [])
        self.assertEqual(self.rows()[0][3], 'failed')

    def test_a_new_game_cannot_be_missed(self):
        os.makedirs(os.path.join(self.repo, 'games/newgame'))
        run = self.smoke('--check-only')
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('games/newgame/smoke.toml: missing', run.stdout)

    def test_one_failed_entry_runs_the_rest(self):
        self.write('toolkit/tools/test_kit.py', FAIL_TEST)
        self.commit()
        run = self.smoke()
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('fake break', run.stdout)
        self.assertEqual(len(self.cargo_log()), 1)
        self.assertEqual(self.rows()[0][3], 'failed')

    def test_sigkill_to_the_process_group_leaves_nothing_running(self):
        self.write('toolkit/tools/test_kit.py', HANG_TEST)
        self.commit()
        pids = os.path.join(self.fake, 'pids')
        # A group of its own, killed whole with SIGKILL: exactly how pir ends a test line.
        with open(os.path.join(self.base, 'out'), 'w') as out:
            proc = subprocess.Popen([os.path.join(self.repo, 'tools', 'smoke.sh')], cwd=self.base,
                                    env=self.env(), stdout=out, stderr=out, start_new_session=True)
        self.addCleanup(proc.wait)
        self.assertTrue(wait_until(lambda: os.path.exists(pids), 60))
        with open(pids) as f:
            entry, child = (int(p) for p in f.read().split())
        self.addCleanup(lambda: [os.kill(p, signal.SIGKILL) for p in (entry, child) if alive(p)])
        self.assertTrue(alive(entry))
        os.killpg(proc.pid, signal.SIGKILL)
        self.assertTrue(wait_until(lambda: not alive(entry) and not alive(child)))
        self.assertTrue(wait_until(lambda: len(self.rows()) == 1))
        self.assertEqual(self.rows()[0][3], 'failed')

    def test_the_hard_limit_fails_the_command_with_a_limit_row(self):
        path = os.path.join(self.repo, 'tools/smoke.py')
        with open(path) as f:
            source = f.read()
        self.assertIn('LIMIT_SECONDS = 20 * 60', source)
        self.write('tools/smoke.py', source.replace('LIMIT_SECONDS = 20 * 60', 'LIMIT_SECONDS = 2'))
        self.write('toolkit/tools/test_kit.py', HANG_TEST)
        self.commit()
        run = self.smoke()
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('smoke: FAILED, over the limit', run.stdout)
        self.assertIn('did not run: games/demo', run.stdout)
        self.assertEqual([row[3] for row in self.rows()], ['limit'])

    def test_sigterm_kills_the_entry_and_records_the_run(self):
        self.write('toolkit/tools/test_kit.py', HANG_TEST)
        self.commit()
        pids = os.path.join(self.fake, 'pids')
        with open(os.path.join(self.base, 'out'), 'w') as out:
            proc = subprocess.Popen([os.path.join(self.repo, 'tools', 'smoke.sh')], cwd=self.base,
                                    env=self.env(), stdout=out, stderr=out, start_new_session=True)
        self.addCleanup(proc.wait)
        self.assertTrue(wait_until(lambda: os.path.exists(pids), 60))
        with open(pids) as f:
            entry, child = (int(p) for p in f.read().split())
        proc.terminate()
        self.assertEqual(proc.wait(timeout=20), 1)
        self.assertTrue(wait_until(lambda: not alive(entry) and not alive(child)))
        time.sleep(0.2)  # room for a second row, from the watchdog, had the `end` line gone missing
        self.assertEqual([row[3] for row in self.rows()], ['failed'])

    def test_an_unwritable_history_prints_a_line_and_keeps_the_exit_status(self):
        blocker = os.path.join(self.base, 'blocker')
        with open(blocker, 'w') as f:
            f.write('a file where a folder is needed')
        path = os.path.join(blocker, 'times.tsv')
        run = self.smoke(SMOKE_TIMES_FILE=path)
        self.assertEqual(run.returncode, 0)
        self.assertIn(f'smoke: could not write the history to {path}', run.stdout)

    def test_the_default_history_lands_outside_the_repo(self):
        run = self.smoke(SMOKE_TIMES_FILE='')
        self.assertEqual(run.returncode, 0)
        self.assertTrue(os.path.isfile(os.path.join(self.home, '.cache/superfrog-re/smoke-times.tsv')))

    def test_the_run_leaves_the_repo_clean(self):
        self.smoke()
        status = subprocess.run(['git', 'status', '--porcelain'], cwd=self.repo, env=self.env(),
                                capture_output=True, text=True)
        self.assertEqual(status.stdout, '')

    def test_list_runs_nothing(self):
        run = self.smoke('--list')
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        lines = run.stdout.splitlines()
        self.assertEqual([line.split('\t')[0] for line in lines],
                         ['root', 'root', 'toolkit', 'games/demo', 'games/demo'])
        self.assertEqual(lines[3].split('\t')[1:], ['games/demo/engine', 'cargo test -q -p x --test quick'])
        self.assertEqual(self.cargo_log(), [])
        self.assertEqual(self.rows(), [])

    def test_check_only_runs_no_entry(self):
        run = self.smoke('--check-only')
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(self.cargo_log(), [])
        self.assertEqual(self.rows(), [])


if __name__ == '__main__':
    unittest.main()
