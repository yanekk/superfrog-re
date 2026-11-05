"""translate.py with the real judge (plans/forest-dumb-level-1 T33): canned answers, built into
fd-judge by cargo and run on the main program from the owner's disks. The model is stubbed: a test
file never calls the network."""
import json
import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import translate as t  # noqa: E402

# h0+$774 by hand (fd-conform/tests/judge.rs): a raster wait, then one blit of tile d5 to a3.
RIGHT = """pub fn port(cpu: &mut Cpu) {
    cpu.d[5] = (cpu.d[5] & 0xffff) * 0xa0;
    let a5 = h(3, 0x1_3540).wrapping_add(cpu.d[5]);
    let a3 = cpu.a[3];
    let m = &mut cpu.mem;
    m.wl(0xdff054, a3);
    m.wl(0xdff050, a5);
    m.ww(0xdff066, MODULO);
    m.ww(0xdff064, 0);
    m.wl(0xdff044, 0xffff_ffff);
    m.ww(0xdff040, 0x09f0);
    m.ww(0xdff042, 0);
    m.ww(0xdff058, 0x1401);
}
"""


def answer(code):
    return f'```rust\n{code}```\n'


def port(modulo):
    return RIGHT.replace('MODULO', hex(modulo))


class RealJudge(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.out = self.tmp.name
        self.sent = []
        self.answers = []

        def call_model(messages, key):
            self.sent.append(messages[-1]['content'])
            return self.answers.pop(0), {'in': 1, 'out': 1, 'reasoning': 0, 'cost': 0.0, 'finish': 'stop'}

        for name, value in [('find_key', lambda: ('sk-test', 'nowhere')),
                            ('flow_text', lambda h, o, stops=(): ('listing', 20)),
                            ('price', lambda key: (0.0, 0.0)),
                            ('call_model', call_model)]:
            p = mock.patch.object(t, name, value)
            p.start()
            self.addCleanup(p.stop)

    def test_compile_errors_and_the_first_failing_case_go_back_then_a_right_port_passes(self):
        self.answers = [answer('pub fn port(cpu: &mut Cpu) { cpu.nope = 1; }\n'),
                        answer(port(0x2e)), answer(port(0x2c))]
        with mock.patch.object(t, 'split_at', lambda h, o: []):
            res = t.translate('h0+$774', 'h0+$774', out=self.out)
        self.assertEqual((res['result'], res['tries'], res['paths']), ('pass', 3, 3), res)
        self.assertIn('did not compile', self.sent[1])
        self.assertIn('nope', self.sent[1])
        self.assertIn('differs on 2000 of 2000 states', self.sent[2])
        self.assertIn('$dff066=$002c', self.sent[2])

    def test_a_split_routine_is_judged_with_its_callees_port_built_in(self):
        # The passing h0+$774 port filed as a callee's; the caller only calls it.
        callee = os.path.join(self.out, 'h0-5462')
        os.makedirs(callee)
        with open(os.path.join(callee, 'port.rs'), 'w') as f:
            f.write(port(0x2c))
        with open(os.path.join(callee, 'result.json'), 'w') as f:
            json.dump({'result': 'pass', 'split_at': []}, f)
        self.answers = [answer('pub fn port(cpu: &mut Cpu) {\n    port_h0_5462(cpu);\n}\n')]
        with mock.patch.object(t, 'split_at', lambda h, o: [(0, 0x5462)] if (h, o) == (0, 0x774) else []):
            res = t.translate('h0+$774', 'h0+$774', out=self.out)
        self.assertEqual((res['result'], res['tries']), ('pass', 1), res)


if __name__ == '__main__':
    unittest.main()
