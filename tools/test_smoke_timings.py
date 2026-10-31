"""smoke_timings on fake entries: the cap, the row and the key form. Nothing here runs cargo."""
import os
import tempfile
import time
import unittest
from pathlib import Path

import smoke_core
import smoke_timings


def fake(script: str) -> smoke_timings.Timed:
    # measure() appends --no-run for the compile step; under `sh -c` that word arrives as $0.
    body = f'if [ "$0" = "--no-run" ]; then exit 0; fi; {script}'
    return smoke_timings.Timed('p', 'k', '.', ['sh', '-c', body])


class Measure(unittest.TestCase):
    def test_the_cap_kills_a_slow_entry_and_its_child_and_records_over(self):
        with tempfile.TemporaryDirectory() as tmp:
            pid_file = Path(tmp) / 'child'
            entry = fake(f'sleep 30 & echo $! > {pid_file}; wait')
            start = time.monotonic()
            seconds, status, _ = smoke_timings.measure(entry, 0.3, cwd=tmp)
            self.assertLess(time.monotonic() - start, 5)
            self.assertEqual((seconds, status), ('over', '-'))
            child = int(pid_file.read_text())
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                try:
                    os.kill(child, 0)
                except ProcessLookupError:
                    return
                time.sleep(0.02)
            self.fail('the child outlived the cap')

    def test_a_fast_entry_records_its_seconds(self):
        seconds, status, tail = smoke_timings.measure(fake('exit 0'), 5, cwd='.')
        self.assertLess(float(seconds), 5)
        self.assertEqual((status, tail), ('0', b''))

    def test_a_failing_entry_records_its_status_and_its_output(self):
        seconds, status, tail = smoke_timings.measure(fake('echo broke; exit 7'), 5, cwd='.')
        self.assertLess(float(seconds), 5)
        self.assertEqual(status, '7')
        self.assertIn(b'broke', tail)

    def test_a_compile_failure_is_not_a_time(self):
        entry = smoke_timings.Timed('p', 'k', '.', ['sh', '-c', 'exit 101'])
        self.assertEqual(smoke_timings.measure(entry, 5, cwd='.')[:2], ('uncompiled', '101'))


class Keys(unittest.TestCase):
    def project(self, manifest=None):
        return smoke_core.Project('games/g', manifest, [
            'engine/crates/a/tests/quick.rs',
            'engine/crates/a/tests/dir/main.rs',
            'engine/crates/a/src/lib.rs',
            'engine/crates/b/src/main.rs',
        ], {
            'engine/crates/a/tests/quick.rs': '#[test] fn t() {}',
            'engine/crates/a/tests/dir/main.rs': '#[test] fn t() {}',
            'engine/crates/a/src/lib.rs': '#[test] fn t() {}',
            'engine/crates/b/src/main.rs': '#[test] fn t() {}',
        })

    def test_the_keys_are_exactly_what_the_check_asks_for(self):
        entries = smoke_timings.entries_of(self.project(), 'engine')
        labels = ''.join(f'"{e.key}" = "smoke"\n' for e in entries)
        manifest = (f'workspace = "engine"\n[tests]\n{labels}[[area]]\nname = "a"\n'
                    f'covers = ["{entries[0].key}"]\n')
        self.assertEqual(len(entries), 4)
        self.assertEqual(smoke_core.check(self.project(manifest)), [])

    def test_the_command_is_the_one_smoke_will_run(self):
        project = self.project()
        entries = smoke_timings.entries_of(project, 'engine')
        labels = ''.join(f'"{e.key}" = "smoke"\n' for e in entries)
        made = self.project(f'workspace = "engine"\n[tests]\n{labels}')
        self.assertEqual([e.argv for e in entries], [e.argv for e in smoke_core.run_list(made)])
        self.assertEqual({e.cwd for e in entries}, {'games/g/engine'})


if __name__ == '__main__':
    unittest.main()
