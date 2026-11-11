"""Translator T15 (DESIGN §2.5): `h0_6852` on the real translator and the game's program. Its
generator gives slot d7 values the game never does, and on those states the original writes into its
own code; they are counted as self-modifying and left out, and the routine passes on the rest. Full
set: a Ghidra export, a translation, a fd-judge build and one judge run, about a minute; it holds
judge.lock and writes build/translator/ as the trial does."""
import json
import os
import sys
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import translator_trial as t  # noqa: E402


class CodeWriteRule(unittest.TestCase):
    def test_h0_6852_passes_with_its_self_modifying_states_shown(self):
        out = []
        with mock.patch('builtins.print', lambda *a, file=None, **k: out.append(' '.join(map(str, a)))):
            code = t.main(['--routines', 'h0_6852'])
        self.assertEqual(code, 0, '\n'.join(out))
        s = json.loads(out[-1])
        self.assertEqual((s['judgeable'], s['pass'], s['self_modifying'], s['write_into_code']), (1, 1, 1, 0))
        paths = t.Paths()
        with open(os.path.join(paths.out, 'trial.json')) as f:
            row = json.load(f)['rows'][0]
        p = row['plain']
        self.assertEqual((row['routine'], p['verdict']), ('h0+$6852', 'pass'))
        self.assertGreater(p['self_modifying'], 0)
        self.assertGreater(p['passed'], 0)
        self.assertEqual(p['passed'] + p['self_modifying'], t.jr.CASES)
        with open(os.path.join(paths.out, 'table.md')) as f:
            self.assertIn(f"| pass |  | 0 | 0 | {p['self_modifying']} | 0 |", f.read())


if __name__ == '__main__':
    unittest.main()
