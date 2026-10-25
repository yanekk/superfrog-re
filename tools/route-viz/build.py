#!/usr/bin/env python3
"""Inline a route_viz JSON dump into template.html: one self-contained page per route.

    cd engine && cargo run --release -p sf-plan --example route_viz -- \
        W1-4 ../plans/smart-planner/bench/routes/app/W1-4.txt ../tools/route-viz/W1-4.json
    python3 tools/route-viz/build.py tools/route-viz/W1-4.json   # -> tools/route-viz/W1-4.html

The JSON and HTML hold the level's tiles, which come from the copyrighted disks: they are
git-ignored, like build/.
"""
import os, sys

here = os.path.dirname(os.path.abspath(__file__))
src = sys.argv[1]
out = sys.argv[2] if len(sys.argv) > 2 else os.path.splitext(src)[0] + '.html'
data = open(src).read().replace('</', '<\\/')
page = open(os.path.join(here, 'template.html')).read().replace('__DATA__', data)
open(out, 'w').write(page)
print(out, len(page), 'bytes')
