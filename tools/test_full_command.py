"""tools/full.sh, proven on a fake cargo in a scratch folder, because the full set is never run from a
session (smoke-tests DESIGN §2.1, §4). The real test.sh files run; the real suites never do. In this repo
tools/test.sh's root checks would discover this file and recurse without end, hence the scratch folder."""
import os, shutil, stat, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SCRIPTS = ('tools/test.sh', 'tools/full.sh', 'tools/select_scope.py', 'toolkit/test.sh',
           'games/superfrog/test.sh', 'games/forest-dumb/test.sh')
ENGINES = {'toolkit': 'toolkit', 'superfrog': 'games/superfrog/engine', 'forest-dumb': 'games/forest-dumb/engine'}
FAKE_CARGO = '''#!/bin/sh
printf '%s\\n' "$PWD" "$@" "" >> "$FAKE_CARGO_LOG"
case "$PWD" in *"$FAKE_CARGO_FAIL") [ -n "$FAKE_CARGO_FAIL" ] && exit 101 ;; esac
exit 0
'''
SOURCE = '''
// #[ignore = "SF_E2E_OS=1 in a comment"]
fn not_a_test() {}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test a -- --ignored"]
fn a_screen_test() {
    fn inner() {}
}

#[test]
#[ignore = "a tool: writes the dry runs' zips into $SF_T14_ZIPS"]
fn a_tool() {}

#[test]
#[ignore = "slow: about 3 min"]
fn a_slow_test() {}

#[test]
#[ignore]
fn a_bare_ignored_test() {}

#[test]
fn an_ordinary_test() {}
'''


class FullCommand(unittest.TestCase):
    def setUp(self):
        self.repo = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.repo)
        for rel in SCRIPTS:
            os.makedirs(os.path.dirname(self.path(rel)), exist_ok=True)
            shutil.copy(os.path.join(ROOT, rel), self.path(rel))
        # What the real test.sh files need beside cargo: an interpreter, empty test folders, a stub check.
        os.makedirs(self.path('venv/bin'))
        os.symlink(sys.executable, self.path('venv/bin/python'))
        for rel in ('toolkit/tools', 'games/superfrog/tools', 'games/forest-dumb/tools', 'games/forest-dumb/engine'):
            os.makedirs(self.path(rel), exist_ok=True)
        self.write('tools/check_tier_names.py', '')
        self.write('fake/cargo', FAKE_CARGO)
        os.chmod(self.path('fake/cargo'), stat.S_IRWXU)

    def path(self, rel):
        return os.path.join(self.repo, rel)

    def write(self, rel, text):
        os.makedirs(os.path.dirname(self.path(rel)), exist_ok=True)
        with open(self.path(rel), 'w') as f:
            f.write(text)

    def run_it(self, script, **env):
        """(exit status, {project: its cargo arguments}) with a fake cargo first on the PATH."""
        clean = {k: v for k, v in os.environ.items() if k not in ('PIR_TEST_INCLUDE_IGNORED', 'SF_E2E_OS', 'SF_T14_ZIPS')}
        r = subprocess.run(['sh', script], cwd=self.repo, capture_output=True, text=True, env={
            **clean, 'PATH': self.path('fake') + os.pathsep + os.environ['PATH'],
            'FAKE_CARGO_LOG': self.path('log'), 'FAKE_CARGO_FAIL': '', **env})
        calls = {}
        if os.path.exists(self.path('log')):
            with open(self.path('log')) as f:
                log = f.read()
            for call in log.split('\n\n'):
                if call.strip():
                    cwd, *args = call.strip().split('\n')
                    project = next(p for p, e in ENGINES.items() if cwd.endswith(e))
                    self.assertNotIn(project, calls, 'one cargo call per project')
                    calls[project] = args
        return r.returncode, calls

    def seed_superfrog(self):
        self.write('games/superfrog/engine/crates/x/tests/a.rs', SOURCE)

    def test_full_gives_every_project_include_ignored(self):
        self.seed_superfrog()
        status, calls = self.run_it('tools/full.sh')
        self.assertEqual(status, 0)
        self.assertEqual(sorted(calls), sorted(ENGINES))
        for project, args in calls.items():
            self.assertEqual(args[:5], ['test', '-q', '--workspace', '--', '--include-ignored'], project)

    def test_only_superfrog_has_anything_left_out(self):
        self.seed_superfrog()
        _, calls = self.run_it('tools/full.sh')
        self.assertEqual(len(calls['toolkit']), 5)
        self.assertEqual(len(calls['forest-dumb']), 5)

    def test_the_everything_run_without_the_variable_gives_no_flag(self):
        self.seed_superfrog()
        status, calls = self.run_it('tools/test.sh', PIR_TEST_SCOPE='all')
        self.assertEqual(status, 0)
        self.assertEqual(sorted(calls), sorted(ENGINES))
        for project, args in calls.items():
            self.assertEqual(args, ['test', '-q', '--workspace'], project)

    def test_superfrog_skips_a_screen_test_and_the_tool_and_no_other(self):
        self.seed_superfrog()
        _, calls = self.run_it('tools/full.sh')
        # --exact, so a skip cannot also take a test whose name merely contains this one.
        self.assertEqual(calls['superfrog'][5:], ['--exact', '--skip', 'a_screen_test', '--skip', 'a_tool'])

    def test_with_the_screen_variable_only_the_tool_is_skipped(self):
        self.seed_superfrog()
        _, calls = self.run_it('tools/full.sh', SF_E2E_OS='1')
        self.assertEqual(calls['superfrog'][5:], ['--exact', '--skip', 'a_tool'])

    def test_with_the_tool_variable_only_the_screen_test_is_skipped(self):
        self.seed_superfrog()
        _, calls = self.run_it('tools/full.sh', SF_T14_ZIPS='/tmp/somewhere')
        self.assertEqual(calls['superfrog'][5:], ['--exact', '--skip', 'a_screen_test'])

    def test_with_both_variables_superfrog_skips_nothing(self):
        self.seed_superfrog()
        _, calls = self.run_it('tools/full.sh', SF_E2E_OS='1', SF_T14_ZIPS='/tmp/somewhere')
        self.assertEqual(calls['superfrog'], ['test', '-q', '--workspace', '--', '--include-ignored'])

    def test_a_screen_variable_other_than_1_still_skips(self):
        # The tests themselves panic on anything but 1.
        self.seed_superfrog()
        _, calls = self.run_it('tools/full.sh', SF_E2E_OS='0')
        self.assertIn('a_screen_test', calls['superfrog'])

    def test_the_real_source_gives_32_screen_tests(self):
        os.symlink(os.path.join(ROOT, 'games/superfrog/engine'), self.path('games/superfrog/engine'))
        _, calls = self.run_it('tools/full.sh')
        args = calls['superfrog']
        names = [args[i + 1] for i, a in enumerate(args) if a == '--skip']
        # 32 screen tests and the zips tool.
        self.assertEqual(len(names), 33, names)
        self.assertEqual(len(set(names)), 33, names)
        self.assertIn('zips', names)
        self.assertIn('fullscreen_is_remembered_across_a_relaunch', names)
        self.assertIn('os_layer_a_menu_click_reaches_the_app', names)

    def test_a_failing_project_fails_the_run_and_the_others_still_run(self):
        self.seed_superfrog()
        for project, engine in ENGINES.items():
            if os.path.exists(self.path('log')):
                os.remove(self.path('log'))
            status, calls = self.run_it('tools/full.sh', FAKE_CARGO_FAIL=engine)
            self.assertNotEqual(status, 0, project)
            self.assertEqual(sorted(calls), sorted(ENGINES), project)

    def test_full_is_executable(self):
        self.assertTrue(os.stat(os.path.join(HERE, 'full.sh')).st_mode & stat.S_IXUSR)


if __name__ == '__main__':
    unittest.main()
