"""PUT local images to the presigned URLs a Higgsfield media_upload call returned.

usage: python tools/hd/upload.py <media_upload result .json> [...]

The model is reached through the Higgsfield MCP tools, which a session calls; this script only
does the byte transfer those tools hand back. Upload filenames are 'w<N>_<name>.png'; the local
file is build/hd2/w<N>/src/<name>.png and the media id is recorded in build/hd2/w<N>/media_ids.json.
Names with no local file (padding a request so its result is written to a file) are skipped.
"""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def main(results):
    for result in results:
        with open(result) as fh:
            data = json.load(fh)
        for u in data['uploads'] if isinstance(data, dict) else data:
            name = u.get('filename') or u['instructions'].split('--data-binary @')[1].split(' ')[0]
            world, rest = name.split('_', 1)
            d = os.path.join(ROOT, 'build', 'hd2', world)
            local = os.path.join(d, 'src', rest)
            if not os.path.exists(local):
                continue
            code = subprocess.run(['curl', '-s', '-o', '/dev/null', '-w', '%{http_code}', '-X', 'PUT',
                                   '-H', 'Content-Type: image/png', '--data-binary', '@' + local,
                                   u['upload_url']], capture_output=True, text=True).stdout
            print(name, code, u['media_id'])
            if code == '200':
                ids_path = os.path.join(d, 'media_ids.json')
                ids = json.load(open(ids_path)) if os.path.exists(ids_path) else {}
                ids[os.path.splitext(rest)[0]] = u['media_id']
                with open(ids_path, 'w') as fh:
                    json.dump(ids, fh, indent=1)


if __name__ == '__main__':
    main(sys.argv[1:])
