"""reshape.py with the real judge (plans/forest-dumb-level-1 T62): canned answers for `rise_step`
(h0_4274), built into the scratch copy of the engine and run on T08's rig test over 10 000 states,
on the main program from the owner's disks. The model is stubbed: a test file never calls the
network. The literal ports are stubbed too, so this needs no listing and no build/translate."""
import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import reshape as r  # noqa: E402
from test_reshape import fake_ports  # noqa: E402


def committed(fn):
    return r.fn_text(r.read(os.path.join(r.ENGINE, r.TASKS['T08']['src'])), fn)


def answer(code):
    return f'```rust\n{code}```\n'


class RealRig(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        fake_ports(self.tmp.name)
        self.sent, self.answers = [], []

        def call_model(messages, key):
            self.sent.append(messages[-1]['content'])
            return self.answers.pop(0), {'in': 1, 'out': 1, 'reasoning': 0, 'cost': 0.0, 'finish': 'stop'}

        for name, value in [('find_key', lambda: ('sk-test', 'nowhere')), ('price', lambda key: (0.0, 0.0)),
                            ('call_model', call_model)]:
            p = mock.patch.object(r.tr, name, value)
            p.start()
            self.addCleanup(p.stop)

    def test_compile_errors_and_the_first_failing_case_go_back_then_the_committed_function_passes(self):
        right = committed('rise_step')
        # A rise a pixel short: `v - 1` where the original moves `v`.
        wrong = right.replace('self.state.char_y.wrapping_sub(v as u32)', 'self.state.char_y.wrapping_sub(v as u32).wrapping_add(1)')
        self.assertNotEqual(wrong, right)
        self.answers = [answer("impl Character<'_> {\n    pub fn rise_step(&mut self) { no_such_thing(); }\n}\n"),
                        answer("impl Character<'_> {\n" + wrong + "}\n"),
                        answer("impl Character<'_> {\n" + right + "}\n")]
        res = r.reshape('h0_4274', 'T08', out=os.path.join(self.tmp.name, 'out'), dirs=[self.tmp.name])
        self.assertIn('It did not compile', self.sent[1])
        self.assertIn('no_such_thing', self.sent[1])
        self.assertIn('differs', self.sent[2])
        self.assertIn('h0_4274', self.sent[2])
        # char_y's low byte: the rig names addresses, the prompt's LAYOUT names them.
        self.assertIn('byte h0+$1e61: original wrote', self.sent[2])
        self.assertEqual((res['result'], res['tries'], res['paths']), ('pass', 3, 6), res)
        self.assertEqual(r.read(os.path.join(r.ENGINE, r.TASKS['T08']['src'])).count(right), 1,
                         'the committed source is never changed')


if __name__ == '__main__':
    unittest.main()
