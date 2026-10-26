"""Tests for tools/handover.py's pure parts (no engine, no disks)."""
import os, sys, tempfile, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import handover as H

ROUTE = '# timing: app\n# seed: 1\n0: -\n3: U L\n5: R\n7: -\n'


class Keys(unittest.TestCase):
    def test_the_owners_mapping_wins(self):
        with tempfile.NamedTemporaryFile('w', suffix='.toml', delete=False) as f:
            f.write('[keys]\nup = ["Space"]\nfire = ["AltLeft"]\nleft = ["ArrowLeft"]\n')
        keys = H.keys_from(f.name)
        os.unlink(f.name)
        self.assertEqual(keys['U'], 'Space')
        self.assertEqual(keys['F'], 'AltLeft')
        self.assertEqual(keys['R'], 'ArrowRight')

    def test_no_settings_is_the_apps_default(self):
        self.assertEqual(H.keys_from('/nonexistent/settings.toml'), H.DEFAULT_KEYS)


class Script(unittest.TestCase):
    def test_route_then_every_key_up_and_pause(self):
        s = H.script(ROUTE, 'W1-1', 1, 6, H.DEFAULT_KEYS, 7).splitlines()
        self.assertIn('fast-until 7', s)
        self.assertIn('seed 1 1', s)
        self.assertIn('1: start-level W1-1 lives=7 record', s)
        # Route frame i is the app's step i + 1.
        self.assertIn('4: down ArrowUp', s)
        self.assertIn('4: down ArrowLeft', s)
        self.assertIn('6: up ArrowUp', s)
        self.assertIn('6: up ArrowLeft', s)
        self.assertIn('6: down ArrowRight', s)
        self.assertEqual(s[-2:], ['7: up ArrowRight', '7: menu pause'])

    def test_first_at_skips_a_dying_frog(self):
        # A frog at cell (10, 5): X = 10*16 - 0x17, Y = 5*16 + 0x10.
        x, y = 10 * 16 - 0x17, 5 * 16 + 0x10
        trail = [f'0 0 0 0 0 0 3', f'1 {x} {y} 0 0 1 3', f'2 {x} {y} 0 0 0 3']
        self.assertEqual(H.first_at(trail, 10, 5), 2)
        self.assertIsNone(H.first_at(trail[:2], 10, 5))


if __name__ == '__main__':
    unittest.main()
