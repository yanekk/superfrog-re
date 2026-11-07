"""pack_check.py on a hand-made conversation log: no run needed."""
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pack_check as pc  # noqa: E402


def step(mid, ctx, *acts):
    content = [{'type': 'tool_use', 'name': name, 'input': {'command': text} if name == 'Bash' else {'file_path': text}}
               for name, text in acts]
    return {'event': {'type': 'assistant', 'message': {'id': mid, 'content': content,
                                                       'usage': {'cache_read_input_tokens': ctx}}}}


class Check(unittest.TestCase):
    def test_pack_steps_first_edit_and_rereads(self):
        lines = [
            step('m0', 50_000, ('Bash', 'git status')),
            step('m1', 52_000, ('Bash', 'python3 -I games/forest-dumb/tools/task_pack.py T22')),
            step('m2', 53_000, ('Read', '/w/games/forest-dumb/build/pack/T22.md')),
            step('m3', 80_000, ('Bash', 'sed -n 1,40p plans/forest-dumb-level-1/DESIGN.md')),
            step('m4', 81_000, ('Write', '/w/games/forest-dumb/engine/crates/fd-core/src/objects/mod.rs')),
            {'event': {'type': 'result', 'model_usage_since_last_result': {'claude-opus-5-5': {'costUSD': 1.25}}}},
        ]
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'T22-implement-1.ndjson')
            with open(p, 'w') as f:
                f.write('\n'.join(json.dumps(l) for l in lines) + '\nnot json\n')
            r = pc.check(p)
        self.assertEqual((r['steps'], r['pack_built'], r['pack_read'], r['first_edit'], r['ctx_at_first_edit']),
                         (5, 1, 2, 4, 81_000))
        self.assertEqual(r['reread'], ['DESIGN.md'])
        self.assertEqual(r['usd'], 1.25)


if __name__ == '__main__':
    unittest.main()
