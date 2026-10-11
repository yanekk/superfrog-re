//! The Python tools as yardsticks for the Rust decoders (DESIGN §7: kept as references), run in
//! the worktree's venv (the test block's setup makes it) on bytes piped in, never on files.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const PY_RENDER: &str = r#"
import sys, hashlib, struct
sys.path.insert(0, sys.argv[1])
import render_levels as rl
data = sys.stdin.buffer.read()
at, tiles = 0, {}
def blob():
    global at
    n = struct.unpack_from('>I', data, at)[0]
    at += 4 + n
    return data[at - n:at]
while at < len(data):
    level, tileset, palette_src = blob(), blob(), blob()
    if tileset not in tiles:
        tiles[tileset] = rl.decode_tiles(tileset)
    pal = rl.base_palette(palette_src)
    if pal is None:
        print('no-palette'); continue
    im = rl.render_level(level, tiles[tileset], pal)
    print(im.width, im.height, hashlib.sha1(im.tobytes()).hexdigest())
"#;

/// One level as `tools/render_levels.py` renders it: `render_level(level, decode_tiles(tileset),
/// base_palette(palette_src))`, as `(width, height, SHA-1 of the RGB bytes)`.
pub type PyRender = (u32, u32, String);

/// [`PyRender`] for each `(level, tileset, palette_src)`, in order, in one Python process.
pub fn python_render_levels(jobs: &[(&[u8], &[u8], &[u8])]) -> Result<Vec<PyRender>, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let python = root.join("venv/bin/python");
    let mut input = Vec::new();
    for (a, b, c) in jobs {
        for blob in [a, b, c] {
            input.extend_from_slice(&(blob.len() as u32).to_be_bytes());
            input.extend_from_slice(blob);
        }
    }
    let mut child = Command::new(&python)
        .arg("-c")
        .arg(PY_RENDER)
        .arg(root.join("tools"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "could not run {} ({e}); run the test block's setup",
                python.display()
            )
        })?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    writer
        .join()
        .expect("writer thread")
        .map_err(|e| format!("writing to the Python renderer: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "the Python renderer failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            match f[..] {
                [w, h, sha] => Ok((
                    w.parse().map_err(|_| l.to_string())?,
                    h.parse().map_err(|_| l.to_string())?,
                    sha.to_string(),
                )),
                _ => Err(format!("unexpected line from the Python renderer: {l}")),
            }
        })
        .collect()
}
