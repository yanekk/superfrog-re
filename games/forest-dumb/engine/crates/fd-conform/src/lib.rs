//! Shell, test tooling: Forest Dumb Forever's conformance runner. A case boots the original from
//! power-on on the oracle to a sync point, starts the engine there, feeds both the same input frame
//! by frame and compares the modelled variables (plans/forest-dumb-start DESIGN §2.8, §3.4). The
//! format is in `engine/conform/README.md`.

pub mod disks;
pub mod sync;

use std::fmt;
use std::path::{Path, PathBuf};

use amiga_conform::script;
use amiga_oracle::{Amiga, OracleConfig, kickstart_path};
use fd_core::{LAYOUT, LAYOUT_HUNK, TitleScene};

/// The main program's hunks where the boot of `boot-to-title` loads them, and `boot-to-level1`'s too
/// (it shares the boot up to the title)
/// (`docs/07-executable-map.md`). AmigaDOS picks them when the program loads, so another boot may
/// differ; there a wrong base shows as a comparison failing on its first change.
pub const HUNK_BASES: [u32; 5] = [0xc0_77d8, 0x1_0178, 0x1_1400, 0x1_2c38, 0x4_d668];

/// `engine/conform/`, wherever this is run from.
pub fn conform_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conform")
}

/// A disk change on the way to the sync point: `disk` (1 or 2) into df0, before `frame` runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swap {
    pub frame: u32,
    pub disk: u32,
}

/// One case file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub name: String,
    pub script: String,
    /// How many oracle frames run before the first comparison, and where the engine starts.
    pub sync_frame: u32,
    pub swap: Option<Swap>,
    /// Frames run and compared after the sync point, which is compared too.
    pub frames: u32,
    pub vars: Vec<String>,
}

impl Case {
    /// Read a case file; a relative path that is not there is tried under [`conform_dir`], so the
    /// runner takes `cases/x.toml` from any folder.
    pub fn load(path: &Path) -> Result<Case, String> {
        let found = if path.is_relative() && !path.exists() {
            conform_dir().join(path)
        } else {
            path.to_path_buf()
        };
        let text = std::fs::read_to_string(&found)
            .map_err(|e| format!("case {}: {e}", found.display()))?;
        let name = found.file_stem().unwrap_or_default().to_string_lossy();
        Case::parse(&name, &text).map_err(|e| format!("case {}: {e}", found.display()))
    }

    /// The whole format and nothing else: a key or value this does not know is an error, since a
    /// comparison quietly narrower than its file says would pass for the wrong reason.
    pub fn parse(name: &str, text: &str) -> Result<Case, String> {
        let (mut sync, mut swap, mut frames, mut compare, mut vars) =
            (None, None, None, None, None);
        for (i, raw) in text.lines().enumerate() {
            let at = |e: String| format!("line {}: {e}", i + 1);
            let body = raw.split('#').next().unwrap_or("").trim();
            if body.is_empty() {
                continue;
            }
            let (key, value) = body
                .split_once('=')
                .ok_or_else(|| at(format!("`{body}`: expected `key = value`")))?;
            let (key, value) = (key.trim(), value.trim());
            // A second `frames` would otherwise win silently over the one a reader saw first.
            let once = |taken: bool| {
                if taken {
                    Err(at(format!("`{key}` is given twice")))
                } else {
                    Ok(())
                }
            };
            match key {
                "sync" => {
                    once(sync.is_some())?;
                    sync = Some(sync_table(value).map_err(at)?);
                }
                "swap" => {
                    once(swap.is_some())?;
                    swap = Some(swap_table(value).map_err(at)?);
                }
                "frames" => {
                    once(frames.is_some())?;
                    frames = Some(int(value).map_err(at)?);
                }
                "compare" => {
                    once(compare.is_some())?;
                    compare = Some(strings(value).map_err(at)?);
                }
                "vars" => {
                    once(vars.is_some())?;
                    vars = Some(strings(value).map_err(at)?);
                }
                other => return Err(at(format!("`{other}` is not a case key"))),
            }
        }
        let missing = |k: &str| format!("no `{k}`");
        let (script, sync_frame): (String, u32) = sync.ok_or_else(|| missing("sync"))?;
        // `run` swaps only on the way to the sync point, so a later swap would never happen.
        if let Some(Swap { frame, .. }) = swap.filter(|s: &Swap| s.frame >= sync_frame) {
            return Err(format!(
                "swap at frame {frame}: a swap must come before the sync frame {sync_frame}"
            ));
        }
        let compare = compare.ok_or_else(|| missing("compare"))?;
        if compare != ["vars"] {
            return Err(format!(
                "compare = {compare:?}: this runner compares only [\"vars\"]"
            ));
        }
        let vars: Vec<String> = vars.ok_or_else(|| missing("vars"))?;
        if vars.is_empty() {
            return Err("`vars` is empty: the case would compare nothing".to_string());
        }
        for v in &vars {
            address(v)?;
        }
        let frames: u32 = frames.ok_or_else(|| missing("frames"))?;
        if sync_frame.checked_add(frames).is_none() {
            return Err(format!(
                "sync frame {sync_frame} plus frames {frames} is past any frame cap"
            ));
        }
        Ok(Case {
            name: name.to_string(),
            script,
            sync_frame,
            swap,
            frames,
            vars,
        })
    }
}

fn int(value: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("`{value}` is not a whole number"))
}

fn quoted(value: &str) -> Result<String, String> {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .filter(|v| !v.contains('"'))
        .map(str::to_string)
        .ok_or_else(|| format!("`{value}` is not a quoted string"))
}

fn strings(value: &str) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| format!("`{value}` is not a list"))?;
    inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(quoted)
        .collect()
}

/// The `key = value` pairs of `{ a = 1, b = 2 }`.
fn table(value: &str) -> Result<Vec<(&str, &str)>, String> {
    let inner = value
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .ok_or_else(|| format!("`{value}` is not a `{{ key = value }}` table"))?;
    inner
        .split(',')
        .map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .ok_or_else(|| format!("`{}`: expected `key = value`", pair.trim()))
        })
        .collect()
}

fn sync_table(value: &str) -> Result<(String, u32), String> {
    let (mut script, mut frame) = (None, None);
    for (k, v) in table(value)? {
        match k {
            "script" => script = Some(quoted(v)?),
            "frame" => frame = Some(int(v)?),
            other => return Err(format!("`{other}` is not a sync key")),
        }
    }
    Ok((
        script.ok_or("sync has no `script`")?,
        frame.ok_or("sync has no `frame`")?,
    ))
}

fn swap_table(value: &str) -> Result<Swap, String> {
    let (mut frame, mut disk) = (None, None);
    for (k, v) in table(value)? {
        match k {
            "frame" => frame = Some(int(v)?),
            "disk" => disk = Some(int(v)?),
            other => return Err(format!("`{other}` is not a swap key")),
        }
    }
    let disk = disk.ok_or("swap has no `disk`")?;
    if !(1..=2).contains(&disk) {
        return Err(format!("swap disk {disk}: the set has disks 1 and 2"));
    }
    Ok(Swap {
        frame: frame.ok_or("swap has no `frame`")?,
        disk,
    })
}

/// Where variable `name` is in the original's memory, on the boot the case runs.
pub fn address(name: &str) -> Result<u32, String> {
    let (_, offset) = LAYOUT
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or_else(|| format!("`{name}` is not in fd-core's LAYOUT"))?;
    Ok(HUNK_BASES[LAYOUT_HUNK as usize] + offset)
}

/// The engine's value of variable `name`. Each variable of `LAYOUT` needs its arm here.
fn engine_value(scene: &TitleScene, name: &str) -> u8 {
    match name {
        "mt_counter" => scene.variable(),
        other => panic!("{other} is in LAYOUT but the runner has no engine side for it"),
    }
}

/// How a case came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every variable equal on every pass; the sync point counts as a pass.
    Same { passes: u32 },
    /// The first difference. `pass` 0 is the sync point; `frame` is the oracle's frames run.
    Differs {
        pass: u32,
        frame: u32,
        var: String,
        engine: u8,
        oracle: u8,
    },
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Same { passes } => write!(f, "same on all {passes} passes"),
            Verdict::Differs {
                pass,
                frame,
                var,
                engine,
                oracle,
            } => write!(
                f,
                "differs at pass {pass} (oracle frame {frame}): {var} is {engine} in the engine, {oracle} in the original"
            ),
        }
    }
}

/// Run `case` on the real disks and Kickstart. Every missing input is an error naming its path, and
/// the cheap checks come first so a wrong setup fails before any emulation.
pub fn run(case: &Case) -> Result<Verdict, String> {
    let script_path = conform_dir()
        .join("scripts")
        .join(format!("{}.txt", case.script));
    let text = std::fs::read_to_string(&script_path)
        .map_err(|e| format!("script {}: {e}", script_path.display()))?;
    let script =
        script::parse(&text).map_err(|e| format!("script {}: {e}", script_path.display()))?;
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disks = disks::load()?;
    let vars: Vec<(&str, u32)> = case
        .vars
        .iter()
        .map(|v| Ok((v.as_str(), address(v)?)))
        .collect::<Result<_, String>>()?;

    let cap = case.sync_frame + case.frames;
    let cfg = OracleConfig::new(kickstart, disks, cap);
    let mut amiga = Amiga::power_on(&cfg).map_err(|e| e.to_string())?;
    let step = |amiga: &mut Amiga, frame: u32| {
        let input = script::input_at(&script.lines, frame);
        amiga.step(input).map_err(|e| e.to_string())?;
        Ok::<_, String>(input)
    };
    for frame in 0..case.sync_frame {
        if let Some(swap) = case.swap.filter(|s| s.frame == frame) {
            amiga.insert_disk(0, swap.disk as usize - 1);
        }
        step(&mut amiga, frame)?;
    }
    let mut scene = TitleScene::new();
    for pass in 0..=case.frames {
        let frame = case.sync_frame + pass;
        if pass > 0 {
            let input = step(&mut amiga, frame - 1)?;
            scene.frame(input);
        }
        for &(var, addr) in &vars {
            let (engine, oracle) = (engine_value(&scene, var), amiga.peek(addr, 1)[0]);
            if engine != oracle {
                return Ok(Verdict::Differs {
                    pass,
                    frame,
                    var: var.to_string(),
                    engine,
                    oracle,
                });
            }
        }
    }
    Ok(Verdict::Same {
        passes: case.frames + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "# c\nsync = { script = \"s\", frame = 7 }\nswap = { frame = 3, disk = 2 }\n\
                        frames = 9 # c\ncompare = [\"vars\"]\nvars = [\"mt_counter\"]\n";

    #[test]
    fn a_whole_case_parses() {
        assert_eq!(
            Case::parse("n", GOOD).unwrap(),
            Case {
                name: "n".into(),
                script: "s".into(),
                sync_frame: 7,
                swap: Some(Swap { frame: 3, disk: 2 }),
                frames: 9,
                vars: vec!["mt_counter".into()],
            }
        );
    }

    #[test]
    fn what_the_format_does_not_say_is_refused() {
        for (from, to, says) in [
            ("frames = 9", "frames = 9\nresync = true", "resync"),
            ("frames = 9", "frames = many", "many"),
            ("frames = 9", "", "frames"),
            ("[\"vars\"]", "[\"vars\", \"picture\"]", "compare"),
            ("[\"mt_counter\"]", "[\"lives\"]", "lives"),
            ("[\"mt_counter\"]", "[]", "compare nothing"),
            ("disk = 2", "disk = 3", "disks 1 and 2"),
            ("frame = 7", "frame = 7, df1 = true", "df1"),
            ("frames = 9", "frames = 9\nframes = 8", "twice"),
            ("frame = 3,", "frame = 7,", "before the sync frame"),
            ("frames = 9", "frames = 4294967295", "frame cap"),
        ] {
            let text = GOOD.replace(from, to);
            assert_ne!(text, GOOD, "{from}");
            let err = Case::parse("n", &text).unwrap_err();
            assert!(err.contains(says), "{to}: {err}");
        }
    }

    #[test]
    fn the_variable_resolves_to_the_address_the_map_records() {
        // docs/07-executable-map.md: "one byte, `$66c31` on the oracle".
        assert_eq!(address("mt_counter"), Ok(0x66c31));
    }

    #[test]
    fn a_difference_names_the_pass_and_the_two_values() {
        let text = Verdict::Differs {
            pass: 3,
            frame: 4925,
            var: "mt_counter".into(),
            engine: 3,
            oracle: 2,
        }
        .to_string();
        for part in [
            "pass 3",
            "4925",
            "mt_counter",
            "3 in the engine",
            "2 in the original",
        ] {
            assert!(text.contains(part), "{text}");
        }
    }
}
