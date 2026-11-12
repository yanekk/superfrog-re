"""model.py against a fake `claude` on PATH; FakeClaude is also what test_generate and test_reshape use."""
import json
import os
import sys
import tempfile
import textwrap
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model  # noqa: E402

FAKE = textwrap.dedent('''\
    import json, os, sys, time
    d = os.environ['FAKE_CLAUDE_DIR']
    prompt = sys.stdin.read()
    with open(os.path.join(d, 'seen.jsonl'), 'a') as f:
        f.write(json.dumps({'argv': sys.argv[1:], 'prompt': prompt, 'cwd': os.getcwd()}) + '\\n')
    with open(os.path.join(d, 'answers.json')) as f:
        queue = json.load(f)
    a = queue.pop(0) if queue else {'exit': 9, 'stderr': 'the fake has no answer left'}
    with open(os.path.join(d, 'answers.json'), 'w') as f:
        json.dump(queue, f)
    if isinstance(a, str):
        a = {'text': a}
    time.sleep(a.get('sleep', 0))
    if 'exit' in a:
        sys.stderr.write(a.get('stderr', ''))
        sys.exit(a['exit'])
    if 'raw' in a:
        print(a['raw'])
        sys.exit(0)
    print(json.dumps({'type': 'result', 'is_error': a.get('is_error', False), 'result': a['text'],
                      'total_cost_usd': 0.003,
                      'usage': {'input_tokens': 10, 'cache_creation_input_tokens': 900, 'cache_read_input_tokens': 90,
                                'output_tokens': 50}}))
''')


class FakeClaude:
    """A `claude` on PATH answering from a queue: a string is an answer, {'exit': n} a failing run,
    {'sleep': s, ...} a slow one, {'raw': ..} output that is not its JSON. Resets model's call counter."""

    def __init__(self, test):
        tmp = tempfile.TemporaryDirectory()
        test.addCleanup(tmp.cleanup)
        self.dir = tmp.name
        bin_dir = os.path.join(self.dir, 'bin')
        os.makedirs(bin_dir)
        exe = os.path.join(bin_dir, 'claude')
        with open(exe, 'w') as f:
            f.write(f'#!{sys.executable}\n' + FAKE)
        os.chmod(exe, 0o755)
        self.env = {'PATH': bin_dir + os.pathsep + os.environ.get('PATH', ''), 'FAKE_CLAUDE_DIR': self.dir}
        p = mock.patch.dict(os.environ, self.env)
        p.start()
        test.addCleanup(p.stop)
        for name, value in [('calls', 0), ('MAX_CALLS', 40)]:
            p = mock.patch.object(model, name, value)
            p.start()
            test.addCleanup(p.stop)
        self.answer()

    def answer(self, *answers):
        with open(os.path.join(self.dir, 'answers.json'), 'w') as f:
            json.dump(list(answers), f)

    def seen(self):
        path = os.path.join(self.dir, 'seen.jsonl')
        if not os.path.exists(path):
            return []
        with open(path) as f:
            return [json.loads(l) for l in f]

    def prompts(self):
        return [s['prompt'] for s in self.seen()]


class Ask(unittest.TestCase):
    def setUp(self):
        self.fake = FakeClaude(self)
        self.log = os.path.join(self.fake.dir, 'out', 'calls.jsonl')

    def lines(self):
        with open(self.log) as f:
            return [json.loads(l) for l in f]

    def test_an_answer_is_ok_with_its_text_and_a_log_line(self):
        self.fake.answer('hello')
        r = model.ask('Be brief.', 'Say hello.', log=self.log, tag={'service': 'test'})
        self.assertEqual((r['result'], r['text'], r['tokens_in'], r['tokens_out'], r['usd']),
                         ('ok', 'hello', 1000, 50, 0.003))
        seen = self.fake.seen()[0]
        self.assertEqual(seen['prompt'], 'Say hello.')
        self.assertEqual(seen['argv'][:11], ['-p', '--model', 'haiku', '--tools', '', '--output-format', 'json',
                                             '--max-turns', '1', '--system-prompt', 'Be brief.'])
        self.assertFalse(os.path.realpath(seen['cwd']).startswith(os.path.realpath(os.path.dirname(model.__file__))))
        line = self.lines()[0]
        self.assertEqual((line['result'], line['model'], line['service']), ('ok', 'haiku', 'test'))

    def test_a_non_zero_exit_is_no_model(self):
        self.fake.answer({'exit': 1, 'stderr': 'Not logged in'})
        r = model.ask('s', 'p', log=self.log)
        self.assertEqual((r['result'], r['text']), ('no-model', ''))
        self.assertIn('Not logged in', r['error'])
        self.assertIn('Not logged in', self.lines()[0]['error'])

    def test_an_error_result_or_output_that_is_not_json_is_no_model(self):
        self.fake.answer({'text': 'rate limited', 'is_error': True}, {'raw': 'not json'})
        self.assertEqual(model.ask('s', 'p', log=self.log)['result'], 'no-model')
        self.assertEqual(model.ask('s', 'p', log=self.log)['result'], 'no-model')

    def test_no_claude_on_path_is_no_model(self):
        with mock.patch.dict(os.environ, {'PATH': self.fake.dir}):
            self.assertFalse(model.available())
            self.assertEqual(model.ask('s', 'p', log=self.log)['result'], 'no-model')

    def test_a_slow_answer_is_a_timeout(self):
        self.fake.answer({'sleep': 5, 'text': 'late'})
        r = model.ask('s', 'p', log=self.log, timeout_s=1)
        self.assertEqual(r['result'], 'timeout')
        self.assertLess(r['secs'], 4)
        self.assertEqual(self.lines()[0]['result'], 'timeout')

    def test_the_41st_call_is_refused_without_running(self):
        self.fake.answer(*['x'] * 41)
        rs = [model.ask('s', 'p', log=self.log)['result'] for _ in range(41)]
        self.assertEqual(rs, ['ok'] * 40 + ['max-calls'])
        self.assertEqual(len(self.fake.seen()), 40)
        self.assertEqual(len(self.lines()), 41)
        self.assertEqual(model.calls_left(), 0)

    def test_the_cap_is_settable(self):
        model.set_max_calls(1)
        self.fake.answer('a', 'b')
        self.assertEqual([model.ask('s', 'p', log=self.log)['result'] for _ in range(2)], ['ok', 'max-calls'])


class Transcript(unittest.TestCase):
    def test_one_message_is_itself(self):
        self.assertEqual(model.transcript([{'role': 'user', 'content': 'q'}]), 'q')

    def test_an_exchange_ends_with_the_message_to_answer(self):
        t = model.transcript([{'role': 'user', 'content': 'q1'}, {'role': 'assistant', 'content': 'a1'},
                              {'role': 'user', 'content': 'feedback'}])
        self.assertLess(t.index('# You were asked\n\nq1'), t.index('# You answered\n\na1'))
        self.assertTrue(t.endswith('# Now answer this\n\nfeedback'))


class MuseIsGone(unittest.TestCase):
    # T18 deleted Muse's tools (translator DESIGN §2.13): no file here names its provider or its
    # batch tool. The word is built so this file does not hold it: `grep -ri` over tools/ stays empty.
    GONE = ('open' + 'router', 'translate' + '_batch')

    def test_no_tool_names_muse_s_provider_or_batch(self):
        here = os.path.dirname(os.path.abspath(__file__))
        found = []
        for root, dirs, files in os.walk(here):
            dirs[:] = [d for d in dirs if d != '__pycache__']
            for f in files:
                path = os.path.join(root, f)
                try:
                    with open(path, encoding='utf-8') as fh:
                        text = fh.read().lower()
                except (UnicodeDecodeError, OSError):
                    continue
                found += [f'{os.path.relpath(path, here)}: {w}' for w in self.GONE if w in text]
        self.assertEqual(found, [])


if __name__ == '__main__':
    unittest.main()
