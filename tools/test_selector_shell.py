"""tools/test.sh itself, in a scratch repo with stub suites: what the diff selects and what it runs."""
import os, shutil, stat, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ENV = {'GIT_AUTHOR_NAME': 'x', 'GIT_AUTHOR_EMAIL': 'x@x', 'GIT_COMMITTER_NAME': 'x', 'GIT_COMMITTER_EMAIL': 'x@x'}


class Shell(unittest.TestCase):
    def setUp(self):
        self.repo = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.repo)
        os.makedirs(os.path.join(self.repo, 'tools'))
        for name in ('test.sh', 'select_scope.py'):
            shutil.copy(os.path.join(HERE, name), os.path.join(self.repo, 'tools'))
        os.makedirs(os.path.join(self.repo, 'venv/bin'))
        os.symlink(sys.executable, os.path.join(self.repo, 'venv/bin/python'))
        self.write('.gitignore', 'venv/\nran\n__pycache__/\n')
        self.write('tools/test_root.py', "open('ran', 'a').write('root\\n')\n")
        self.suite('toolkit/test.sh', 'toolkit')
        self.suite('games/g/test.sh', 'g')
        self.write('toolkit/moved.txt', ''.join(f'line {i}\n' for i in range(50)))
        self.git('init', '-q', '-b', 'main')
        self.commit('base')
        self.git('checkout', '-qb', 'feature')

    def write(self, rel, text):
        path = os.path.join(self.repo, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w') as f:
            f.write(text)

    def suite(self, rel, name, status=0):
        self.write(rel, f'#!/bin/sh\necho {name} >> "$(git rev-parse --show-toplevel)/ran"\nexit {status}\n')
        os.chmod(os.path.join(self.repo, rel), stat.S_IRWXU)

    def git(self, *args):
        subprocess.run(['git', *args], cwd=self.repo, check=True, capture_output=True, env={**os.environ, **ENV})

    def commit(self, message):
        self.git('add', '-A')
        self.git('commit', '-qm', message)

    def run_it(self, **env):
        r = subprocess.run(['sh', 'tools/test.sh'], cwd=self.repo, capture_output=True, text=True,
                           env={**os.environ, 'PIR_TEST_SCOPE': '', **env})
        ran = os.path.join(self.repo, 'ran')
        suites = open(ran).read().split() if os.path.exists(ran) else []
        return r.returncode, suites

    def test_a_game_change_runs_that_game(self):
        self.write('games/g/x', '')
        self.commit('x')
        self.assertEqual(self.run_it(), (0, ['g']))

    def test_a_toolkit_change_runs_the_toolkit_then_the_game(self):
        self.write('toolkit/x', '')
        self.commit('x')
        self.assertEqual(self.run_it(), (0, ['toolkit', 'g']))

    def test_plans_only_runs_nothing(self):
        self.write('plans/p/x', '')
        self.commit('x')
        self.assertEqual(self.run_it(), (0, []))

    def test_a_file_moved_out_of_a_project_runs_that_project(self):
        self.git('mv', 'toolkit/moved.txt', 'games/g/moved.txt')
        self.commit('move')
        self.assertEqual(self.run_it(), (0, ['toolkit', 'g']))

    def test_no_main_runs_everything(self):
        self.git('branch', '-M', 'feature', 'other')
        self.git('branch', '-D', 'main')
        self.assertEqual(self.run_it()[1], ['toolkit', 'g', 'root'])

    def test_a_failing_suite_fails_the_run_and_the_rest_still_run(self):
        self.suite('toolkit/test.sh', 'toolkit', status=3)
        self.commit('x')
        self.assertEqual(self.run_it(), (1, ['toolkit', 'g']))

    def test_a_game_with_no_test_sh_fails_loudly(self):
        os.remove(os.path.join(self.repo, 'games/g/test.sh'))
        self.commit('x')
        self.assertEqual(self.run_it(), (1, []))

    def test_the_all_override_runs_everything(self):
        self.write('plans/p/x', '')
        self.commit('x')
        self.assertEqual(self.run_it(PIR_TEST_SCOPE='all')[1], ['toolkit', 'g', 'root'])


if __name__ == '__main__':
    unittest.main()
