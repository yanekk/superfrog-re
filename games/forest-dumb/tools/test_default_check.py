"""default_check.py (translator T21): the per-run verdict on recorded fake transcripts, and one run's
worktree handling with a fake `claude`. No model call."""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import default_check as dc  # noqa: E402

TRANSLATE = "python3 -I games/forest-dumb/tools/translate.py char_sprite_position"


def transcript(*turns, num_turns=None):
    """turns: each a list of (tool name, input) the assistant called in that turn -> stream-json lines."""
    lines = [json.dumps({'type': 'system', 'subtype': 'init', 'tools': dc.TOOLS})]
    for i, calls in enumerate(turns):
        content = [{'type': 'text', 'text': 'thinking aloud'}]
        content += [{'type': 'tool_use', 'id': f't{i}{j}', 'name': n, 'input': inp} for j, (n, inp) in enumerate(calls)]
        # stream-json sends one event a content block, with the message's id repeated
        for block in content:
            lines.append(json.dumps({'type': 'assistant', 'message': {'id': f'msg{i}', 'content': [block]}}))
        lines.append(json.dumps({'type': 'user', 'message': {'content': [{'type': 'tool_result', 'content': 'ok'}]}}))
    lines.append(json.dumps({'type': 'result', 'subtype': 'success', 'num_turns': num_turns or len(turns),
                             'total_cost_usd': 0.12}))
    return lines


def bash(cmd):
    return ('Bash', {'command': cmd})


def read(path):
    return ('Read', {'file_path': path})


class Verdict(unittest.TestCase):
    def test_translate_first_passes(self):
        r = dc.judge(transcript([bash(TRANSLATE)]))
        self.assertEqual(r['result'], 'pass')
        self.assertEqual(r['translate_at_turn'], 1)
        self.assertEqual(r['first_tool'], f'Bash {TRANSLATE}')
        self.assertEqual(r['usd'], 0.12)

    def test_writing_rust_by_hand_first_fails(self):
        r = dc.judge(transcript([read('games/forest-dumb/CLAUDE.md')],
                                [('Write', {'file_path': 'games/forest-dumb/engine/crates/fd-core/src/sprite.rs',
                                            'content': 'pub fn char_sprite_position() {}'})],
                                [bash(TRANSLATE)]))
        self.assertEqual(r['result'], 'fail')
        self.assertEqual(r['translate_at_turn'], 3)
        self.assertTrue(r['first_rust'].startswith('Write '))

    def test_reading_the_listing_then_translate_passes(self):
        r = dc.judge(transcript([read('games/forest-dumb/CLAUDE.md'), read('games/forest-dumb/docs/11-translation.md')],
                                [bash("python3 -I games/forest-dumb/tools/re_listing.py char_sprite_position")],
                                [('Grep', {'pattern': 'char_sprite_position'})],
                                [bash(TRANSLATE)]))
        self.assertEqual(r['result'], 'pass')
        self.assertEqual(r['translate_at_turn'], 4)
        self.assertEqual(r['first_tool'], 'Read games/forest-dumb/CLAUDE.md')

    def test_an_edit_of_a_rust_file_first_fails(self):
        r = dc.judge(transcript([('Edit', {'file_path': '/x/fd-core/src/lib.rs', 'old_string': 'a', 'new_string': 'b'})],
                                [bash(TRANSLATE)]))
        self.assertEqual(r['result'], 'fail')

    def test_another_rust_tool_or_a_redirect_first_fails(self):
        for cmd in ("python3 -I games/forest-dumb/tools/reshape.py h0+$5462",
                    "python3 -I games/forest-dumb/tools/pcode_rust.py --entry h0+$5462",
                    "cat > games/forest-dumb/engine/crates/fd-core/src/sprite.rs <<'EOF'"):
            with self.subTest(cmd=cmd):
                r = dc.judge(transcript([bash(cmd)], [bash(TRANSLATE)]))
                self.assertEqual(r['result'], 'fail')

    def test_never_producing_rust_fails(self):
        r = dc.judge(transcript([read('games/forest-dumb/CLAUDE.md')], num_turns=15))
        self.assertEqual(r['result'], 'fail')
        self.assertIsNone(r['translate_at_turn'])
        self.assertEqual(r['turns'], 15)

    def test_the_translate_py_name_alone_is_not_another_tool(self):
        self.assertEqual(dc.rust_kind(*bash('python3 -I games/forest-dumb/tools/translator_trial.py')), 'other')
        self.assertEqual(dc.rust_kind(*bash('python3 -I games/forest-dumb/tools/translate.py --task T08')), 'translate')
        self.assertIsNone(dc.rust_kind(*read('games/forest-dumb/tools/translate.py')))

    def test_an_empty_transcript_fails(self):
        self.assertEqual(dc.judge([])['result'], 'fail')


class Run(unittest.TestCase):
    """One run in a throwaway repo: the scratch worktree exists while `claude` runs and is gone after."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = os.path.join(self.tmp.name, 'repo')
        os.makedirs(self.repo)
        for args in (['init', '-q'], ['-c', 'user.email=t@t', '-c', 'user.name=t', 'commit', '-q',
                                      '--allow-empty', '-m', 'x']):
            subprocess.run(['git', *args], cwd=self.repo, check=True)
        self.args = SimpleNamespace(routine='char_sprite_position', max_turns=15, minutes=1)

    def tearDown(self):
        self.tmp.cleanup()

    def run_with(self, runner):
        return dc.one_run(1, self.args, repo=self.repo, out=os.path.join(self.tmp.name, 'out'), runner=runner)

    def worktrees(self):
        out = subprocess.run(['git', 'worktree', 'list'], cwd=self.repo, capture_output=True, text=True).stdout
        return len(out.splitlines())

    def test_a_passing_run_removes_its_worktree(self):
        seen = {}

        def runner(argv, *, cwd, stdout, env, **_):
            seen.update(cwd=cwd, argv=argv, exists=os.path.isdir(os.path.join(cwd, '.git')) or
                        os.path.isfile(os.path.join(cwd, '.git')), env=env)
            stdout.write('\n'.join(transcript([bash(TRANSLATE)])) + '\n')
            return subprocess.CompletedProcess(argv, 0, stderr='')

        row = self.run_with(runner)
        self.assertEqual(row['result'], 'pass')
        self.assertTrue(seen['exists'])
        self.assertIn('.claude/worktrees/default-check-', seen['cwd'])
        self.assertFalse(os.path.exists(seen['cwd']))
        self.assertEqual(self.worktrees(), 1)
        self.assertNotIn('CLAUDECODE', seen['env'])
        self.assertNotIn('--model', seen['argv'])
        self.assertIn('Bash(python3 -I games/forest-dumb/tools/*)', seen['argv'])
        self.assertTrue(seen['argv'][-1].startswith('Port char_sprite_position of Forest Dumb Forever'))

    def test_a_timed_out_run_fails_and_removes_its_worktree(self):
        def runner(argv, *, cwd, stdout, **_):
            stdout.write('\n'.join(transcript([read('CLAUDE.md')])[:-1]) + '\n')
            raise subprocess.TimeoutExpired(argv, 60)

        row = self.run_with(runner)
        self.assertEqual(row['result'], 'timeout')
        self.assertEqual(self.worktrees(), 1)

    def test_a_crash_fails_and_removes_its_worktree(self):
        def runner(argv, **_):
            raise RuntimeError('boom')

        with self.assertRaises(RuntimeError):
            self.run_with(runner)
        self.assertEqual(self.worktrees(), 1)


class Usage(unittest.TestCase):
    def test_bad_flags_exit_2(self):
        with open(os.devnull, 'w') as null:
            old, sys.stderr = sys.stderr, null
            try:
                self.assertEqual(dc.main(['--runs', '0']), 2)
                self.assertEqual(dc.main(['--nope']), 2)
            finally:
                sys.stderr = old


if __name__ == '__main__':
    unittest.main()
