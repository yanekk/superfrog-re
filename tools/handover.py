#!/usr/bin/env python3
"""Hand the owner a level a little before where the planner stalls (CLAUDE.md, "When the planner
struggles"): an sf-app script that plays an app-timed route from Game ▸ Start Level… with the
recorder on, fast-forwarded, to the first step the frog is alive within a cell of a spot, then
releases every key and pauses, leaving the keyboard to the owner.

    python3 tools/handover.py ROUTE.txt LEVEL COL,ROW [--settings FILE] [--lives 7] [--out FILE]

ROUTE is `sf-plan --app` output (`# timing: app`, `# seed: N`). `--widescreen` (real-widescreen
T12) hands over a `sf-plan --widescreen` route (`# widescreen: on`): the trail is the wide start's,
and the script turns View ▸ Widescreen on before Start Level if the owner's settings have it off
(the menu item is a toggle, and the app keeps it on afterwards). Without it, 4:3 as before. The script's keys go through the
owner's Controls mapping, so they are written from his settings.toml (`[keys]`; default: the app's
home), not as arrow keys: the owner jumps with Space, and a script pressing ArrowUp would have
scrambled the route (W6-4, 2026-10-25). The frog's trail comes from the engine example
`bug_trail`, run with cargo from engine/.

The owner then runs (printed at the end):

    superfrog --script FILE --max-frames 4000000 --timeout 7200 --bugs-dir <repo>/bugs
"""
import argparse, os, subprocess, sys, tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HOME_SETTINGS = os.path.expanduser('~/Library/Application Support/Superfrog-RE/settings.toml')
# Joystick tokens of a route script (sf_core::input::format_tokens) and the Controls actions.
ACTIONS = {'U': 'up', 'D': 'down', 'L': 'left', 'R': 'right', 'F': 'fire'}
DEFAULT_KEYS = {'U': 'ArrowUp', 'D': 'ArrowDown', 'L': 'ArrowLeft', 'R': 'ArrowRight', 'F': 'Space'}


def keys_from(settings):
    """Each joystick token's first bound host key in settings.toml, else the app's default."""
    bound = tomllib.load(open(settings, 'rb')).get('keys', {}) if settings and os.path.exists(settings) else {}
    return {t: (bound.get(a) or [DEFAULT_KEYS[t]])[0] for t, a in ACTIONS.items()}


def route_lines(text):
    """`(frame, set of joystick tokens)` per line of a route script."""
    out = []
    for raw in text.splitlines():
        body = raw.split('#')[0].strip()
        if body:
            n, toks = body.split(':', 1)
            out.append((int(n), {t for t in toks.split() if t in ACTIONS}))
    return out


def cell(x, y):
    """The frog's cell, as `plan_exit` counts it."""
    return (x + 0x17) // 16, (y - 0x10) // 16


def widescreen_setting(settings):
    """The owner's View ▸ Widescreen setting (settings.toml `widescreen`, default off)."""
    if settings and os.path.exists(settings):
        return bool(tomllib.load(open(settings, 'rb')).get('widescreen', False))
    return False


def trail(*args):
    """The engine example `bug_trail`'s lines for a bug zip, or an app-timed route and level."""
    return subprocess.run(['cargo', 'run', '-q', '--release', '-p', 'sf-conform', '--example',
                           'bug_trail', '--', *args],
                          cwd=os.path.join(ROOT, 'engine'), capture_output=True, text=True,
                          check=True).stdout.splitlines()


def first_at(trail_lines, col, row):
    """The first step whose frog is alive within one cell of (col, row)."""
    for l in trail_lines:
        k, x, y, _, _, dying, _ = map(int, l.split())
        c, r = cell(x, y)
        if not dying and abs(c - col) <= 1 and abs(r - row) <= 1:
            return k
    return None


def script(route_text, level, seed, frame, keys, lives, toggle_wide=False):
    """The app script: the route to `frame` (route frame i is the app's step i + 1, as
    sf-bench's `app_script` maps it), every key up and `menu pause` at step frame + 1."""
    out = [f'# Hand-over for the owner: {level} to step {frame + 1}, then the keyboard is his.',
           f'# Keys as his Controls map them: {keys}',
           'native-ui', f'fast-until {frame + 1}', f'seed {seed} {seed}']
    if toggle_wide:
        # Same-frame events run in file order: widescreen is on when Start Level starts the level.
        out.append('1: menu widescreen')
    out.append(f'1: start-level {level} lives={lives} record')
    lines = route_lines(route_text)
    held, j, cur = set(), 0, set()
    for i in range(frame):
        while j < len(lines) and lines[j][0] <= i:
            cur = lines[j][1]
            j += 1
        for t in ACTIONS:
            if (t in cur) != (t in held):
                out.append(f'{i + 1}: {"down" if t in cur else "up"} {keys[t]}')
        held = set(cur)
    out += [f'{frame + 1}: up {keys[t]}' for t in sorted(held)]
    out.append(f'{frame + 1}: menu pause')
    return '\n'.join(out) + '\n'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('route')
    ap.add_argument('level')
    ap.add_argument('at', help='COL,ROW')
    ap.add_argument('--settings', default=HOME_SETTINGS)
    ap.add_argument('--lives', type=int, default=7, choices=(3, 5, 7))
    ap.add_argument('--out')
    ap.add_argument('--widescreen', action='store_true', help='a sf-plan --widescreen route')
    a = ap.parse_args()
    text = open(a.route).read()
    wide_route = '# widescreen: on' in text.splitlines()
    if a.widescreen != wide_route:
        sys.exit(f'{a.route}: `# widescreen: on` and --widescreen must agree')
    seed = next((int(l.split(':', 1)[1]) for l in text.splitlines() if l.startswith('# seed:')), None)
    if '# timing: app' not in text or seed is None:
        sys.exit(f'{a.route}: not an sf-plan --app route (`# timing: app`, `# seed: N`)')
    col, row = map(int, a.at.split(','))
    extra = ['--widescreen'] if a.widescreen else []
    frame = first_at(trail(os.path.abspath(a.route), a.level, *extra), col, row)
    if frame is None:
        sys.exit(f'the route never reaches {a.level}:{col},{row} with the frog alive')
    keys = keys_from(a.settings)
    out = a.out or f'handover-{a.level}-{col}-{row}.script'
    toggle = a.widescreen and not widescreen_setting(a.settings)
    open(out, 'w').write(script(text, a.level, seed, frame, keys, a.lives, toggle))
    print(f'{out}: {a.level} to step {frame + 1} ({frame / 50:.0f} s of play, fast-forwarded), keys {keys}')
    # The main checkout's bugs/ (git-ignored, so not in a worktree), as docs/BUG-FIX.md finds it.
    common = subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()
    bugs = os.path.join(os.path.dirname(common), 'bugs')
    print(f'owner runs: superfrog --script {os.path.abspath(out)} --max-frames 4000000 --timeout 7200 '
          f'--bugs-dir {bugs}')


if __name__ == '__main__':
    main()
