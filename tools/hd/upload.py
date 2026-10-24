"""PUT local images to the presigned URLs a Higgsfield media_upload call returned.

usage: python tools/hd/upload.py <media_upload result .json> <src dir> <media_ids.json>

The model is reached through the Higgsfield MCP tools, which a session calls; this script only
does the byte transfer those tools hand back, and records filename -> media_id.
Upload filenames are '<prefix>_<name>.png'; the local file is <src dir>/<name>.png.
"""
import json
import os
import subprocess
import sys


def main(result, src, ids_path):
    with open(result) as fh:
        data = json.load(fh)
    ups = data['uploads'] if isinstance(data, dict) else data
    ids = {}
    if os.path.exists(ids_path):
        with open(ids_path) as fh:
            ids = json.load(fh)
    for u in ups:
        fn = u['upload_url'].split('?')[0]
        name = u.get('filename') or ''
        # The result does not always echo the filename; fall back to the instructions text.
        if not name:
            name = u['instructions'].split('--data-binary @')[1].split(' ')[0]
        local = os.path.join(src, name.split('_', 1)[1])
        code = subprocess.run(['curl', '-s', '-o', '/dev/null', '-w', '%{http_code}', '-X', 'PUT',
                               '-H', 'Content-Type: image/png', '--data-binary', '@' + local,
                               u['upload_url']], capture_output=True, text=True).stdout
        print(name, code, u['media_id'])
        if code == '200':
            ids[os.path.splitext(name.split('_', 1)[1])[0]] = u['media_id']
    with open(ids_path, 'w') as fh:
        json.dump(ids, fh, indent=1)


if __name__ == '__main__':
    main(*sys.argv[1:])
