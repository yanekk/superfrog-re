"""Where the owner's disks are (monorepo-and-knowledge DESIGN §2.5): SUPERFROG_DISKS, else
games/superfrog/ in the main checkout, else the main checkout root. The main checkout is the git
common dir's parent, so the lookup works from any worktree."""
import glob, os, subprocess

HERE = os.path.dirname(os.path.abspath(__file__))


def main_checkout():
    common = subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            cwd=HERE, capture_output=True, text=True).stdout.strip()
    return os.path.dirname(common.rstrip('/')) if common else None


def checkout_dirs(main):
    """The two default places in a main checkout, in lookup order."""
    return [os.path.join(main, 'games', 'superfrog'), main]


def holds_disks(d):
    return any('disk' in os.path.basename(p).lower() for p in
               glob.glob(os.path.join(d, '*.zip')) + glob.glob(os.path.join(d, '*.adf')))


def disks_dir():
    """The folder to read: the env var as given, else the first default that holds a disk."""
    env = os.environ.get('SUPERFROG_DISKS')
    if env:
        return env
    main = main_checkout()
    if not main:
        return os.path.dirname(HERE)
    return next((d for d in checkout_dirs(main) if holds_disks(d)), main)
