//! The port register's rules (plans/forest-dumb-level-1 DESIGN §4.1): no two entries for one
//! address or one name, and each entry is a `kind = "func"` symbol of `re/symbols.toml` at the same
//! hunk and offset. `re/symbols.toml` is committed, so those need no disk. It is read by a small
//! parser here, since the plan admits no `toml` dependency. T21 adds the spawnable values of W1-1:
//! every value its object map holds is in `re/objects-W1-1.toml` with its porting task, read from
//! disk 2.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use fd_core::{PORTED, Ported};

/// One `[[sym]]` of `re/symbols.toml`, the keys this test reads.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sym {
    name: String,
    hunk: u8,
    offset: u32,
    kind: String,
}

/// Every `[[sym]]` block's name, hunk, offset and kind. Other keys are skipped; a block missing one
/// of the four, or a value that does not read, is an error naming its line.
fn parse_symbols(text: &str) -> Result<Vec<Sym>, String> {
    type Block = (usize, BTreeMap<String, String>);
    let mut blocks: Vec<Block> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[sym]]" {
            blocks.push((i + 1, BTreeMap::new()));
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            return Err(format!("line {}: not `key = value`: {line}", i + 1));
        };
        let Some((_, keys)) = blocks.last_mut() else {
            return Err(format!("line {}: a key before the first [[sym]]", i + 1));
        };
        keys.insert(k.trim().to_string(), v.trim().to_string());
    }
    blocks
        .into_iter()
        .map(|(line, keys)| {
            let get = |k: &str| {
                keys.get(k)
                    .ok_or(format!("the [[sym]] at line {line} has no `{k}`"))
            };
            let string = |k: &str| -> Result<String, String> {
                let v = get(k)?;
                v.strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .map(str::to_string)
                    .ok_or(format!(
                        "the [[sym]] at line {line}: `{k}` is not a string: {v}"
                    ))
            };
            let int = |k: &str| -> Result<u32, String> {
                let v = get(k)?;
                match v.strip_prefix("0x") {
                    Some(hex) => u32::from_str_radix(&hex.replace('_', ""), 16),
                    None => v.replace('_', "").parse(),
                }
                .map_err(|e| format!("the [[sym]] at line {line}: `{k}` = {v}: {e}"))
            };
            Ok(Sym {
                name: string("name")?,
                hunk: int("hunk")? as u8,
                offset: int("offset")?,
                kind: string("kind")?,
            })
        })
        .collect()
}

/// Everything wrong with `entries` against `syms`, one line each.
fn register_problems(entries: &[Ported], syms: &[Sym]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut by_addr: BTreeMap<(u8, u32), Vec<&str>> = BTreeMap::new();
    let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
    for e in entries {
        by_addr.entry((e.hunk, e.offset)).or_default().push(e.name);
        *by_name.entry(e.name).or_default() += 1;
    }
    for ((hunk, offset), names) in &by_addr {
        if names.len() > 1 {
            problems.push(format!(
                "h{hunk}+${offset:x} is registered {} times ({}): a routine is ported once; call the \
                 registered port instead",
                names.len(),
                names.join(", ")
            ));
        }
    }
    for (name, n) in &by_name {
        if *n > 1 {
            problems.push(format!("{name} is registered {n} times"));
        }
    }
    for e in entries {
        match syms.iter().find(|s| s.name == e.name) {
            None => problems.push(format!(
                "{} (h{}+${:x}, {}) has no entry in re/symbols.toml: name the routine there first",
                e.name, e.hunk, e.offset, e.port
            )),
            Some(s) if s.kind != "func" => problems.push(format!(
                "{} is a `{}` in re/symbols.toml, not a func",
                e.name, s.kind
            )),
            Some(s) if (s.hunk, s.offset) != (e.hunk, e.offset) => problems.push(format!(
                "{} is registered at h{}+${:x}, but re/symbols.toml has it at h{}+${:x}",
                e.name, e.hunk, e.offset, s.hunk, s.offset
            )),
            Some(_) => {}
        }
    }
    problems
}

fn symbols() -> Vec<Sym> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re/symbols.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse_symbols(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

const fn entry(name: &'static str, hunk: u8, offset: u32) -> Ported {
    Ported {
        name,
        hunk,
        offset,
        port: "level::x",
    }
}

#[test]
fn the_real_register_is_consistent_with_the_symbols() {
    let problems = register_problems(PORTED, &symbols());
    assert!(
        problems.is_empty(),
        "fd-core/src/ported.rs:\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn the_symbols_file_reads() {
    let syms = symbols();
    let funcs = syms.iter().filter(|s| s.kind == "func").count();
    assert!(funcs > 50, "only {funcs} funcs read from re/symbols.toml");
    let step = syms
        .iter()
        .find(|s| s.name == "char_step")
        .expect("char_step");
    assert_eq!(
        (step.hunk, step.offset, step.kind.as_str()),
        (0, 0x3e30, "func")
    );
    let mt = syms
        .iter()
        .find(|s| s.name == "mt_music")
        .expect("mt_music");
    assert_eq!((mt.hunk, mt.offset), (4, 0x1852a));
}

#[test]
fn two_entries_for_one_address_fail() {
    let syms = symbols();
    let twice = [entry("char_step", 0, 0x3e30), entry("char_step", 0, 0x3e30)];
    let p = register_problems(&twice, &syms);
    assert!(
        p.iter()
            .any(|l| l.contains("h0+$3e30 is registered 2 times")),
        "{p:?}"
    );
    // Two names for one address is the same routine ported twice.
    let other = Sym {
        name: "char_step_alias".into(),
        hunk: 0,
        offset: 0x3e30,
        kind: "func".into(),
    };
    let syms2: Vec<Sym> = syms.iter().cloned().chain([other]).collect();
    let two_names = [
        entry("char_step", 0, 0x3e30),
        entry("char_step_alias", 0, 0x3e30),
    ];
    let p = register_problems(&two_names, &syms2);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(
        p[0].contains("registered 2 times (char_step, char_step_alias)"),
        "{p:?}"
    );
}

#[test]
fn an_entry_must_match_a_func_symbol() {
    let syms = symbols();
    assert!(register_problems(&[entry("char_step", 0, 0x3e30)], &syms).is_empty());
    let p = register_problems(&[entry("char_step", 0, 0x3e32)], &syms);
    assert!(p[0].contains("re/symbols.toml has it at h0+$3e30"), "{p:?}");
    let p = register_problems(&[entry("char_step", 4, 0x3e30)], &syms);
    assert_eq!(p.len(), 1, "{p:?}");
    let p = register_problems(&[entry("no_such_routine", 0, 0x10)], &syms);
    assert!(p[0].contains("has no entry in re/symbols.toml"), "{p:?}");
    let p = register_problems(&[entry("quit_flag", 0, 0x1a8)], &syms);
    assert!(p[0].contains("is a `var`"), "{p:?}");
}

#[test]
fn a_damaged_symbols_file_is_an_error_not_a_pass() {
    assert!(
        parse_symbols("[[sym]]\nname = \"a\"\nhunk = 0\nkind = \"func\"\n")
            .unwrap_err()
            .contains("no `offset`")
    );
    assert!(
        parse_symbols("[[sym]]\nname = \"a\"\nhunk = 0\noffset = 0xzz\nkind = \"func\"\n").is_err()
    );
    assert!(parse_symbols("name = \"a\"\n").is_err());
    let ok = parse_symbols(
        "# c\n[[sym]]\nexe = \"x\"\nname = \"a\"\nhunk = 4\noffset = 0x1_0000\nkind = \"func\"\n",
    )
    .unwrap();
    assert_eq!(
        ok,
        [Sym {
            name: "a".into(),
            hunk: 4,
            offset: 0x10000,
            kind: "func".into()
        }]
    );
}

// ---- The spawnable values of W1-1 (plans/forest-dumb-level-1 T21) ----

/// One `[[value]]` of `re/objects-W1-1.toml`: the keys this test reads.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Listed {
    value: u8,
    port_task: String,
    routines: Vec<String>,
}

/// Every `[[value]]` block of `text`; a block missing `value`, `port_task` or `routines`, or a value
/// that does not read, is an error naming its line.
fn parse_values(text: &str) -> Result<Vec<Listed>, String> {
    let mut blocks: Vec<(usize, BTreeMap<String, String>)> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[value]]" {
            blocks.push((i + 1, BTreeMap::new()));
            continue;
        }
        let (k, v) = line
            .split_once('=')
            .ok_or(format!("line {}: not `key = value`: {line}", i + 1))?;
        let (_, keys) = blocks
            .last_mut()
            .ok_or(format!("line {}: a key before the first [[value]]", i + 1))?;
        keys.insert(k.trim().to_string(), v.trim().to_string());
    }
    let unquote = |v: &str| v.trim().trim_matches('"').to_string();
    blocks
        .into_iter()
        .map(|(line, keys)| {
            let get = |k: &str| {
                keys.get(k)
                    .ok_or(format!("the [[value]] at line {line} has no `{k}`"))
            };
            let v = get("value")?;
            let value = match v.strip_prefix("0x") {
                Some(hex) => u8::from_str_radix(hex, 16),
                None => v.parse(),
            }
            .map_err(|e| format!("the [[value]] at line {line}: value = {v}: {e}"))?;
            let r = get("routines")?;
            let inner = r
                .strip_prefix('[')
                .and_then(|r| r.strip_suffix(']'))
                .ok_or(format!(
                    "the [[value]] at line {line}: routines is not a list"
                ))?;
            Ok(Listed {
                value,
                port_task: unquote(get("port_task")?),
                routines: inner
                    .split(',')
                    .map(unquote)
                    .filter(|s| !s.is_empty())
                    .collect(),
            })
        })
        .collect()
}

/// The `name`s of `re/translate.toml`'s routines.
fn translate_names() -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re/translate.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("name = "))
        .map(|n| n.trim_matches('"').to_string())
        .collect()
}

fn listed_values() -> Vec<Listed> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re/objects-W1-1.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse_values(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The values W1-1's object map holds in its used columns, from disk 2, as shipped.
fn w1_1_values() -> BTreeSet<u8> {
    let dir = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    let map = fd_tools::game_file(&dir, "disk2/_MapaBobow1").unwrap_or_else(|e| panic!("{e}"));
    // The used columns stay inside the collision map's 240 (docs/05-file-formats.md).
    map[..240 * 96]
        .iter()
        .copied()
        .filter(|&v| v != 0)
        .collect()
}

/// The object values fd-core's dispatch (T07's `Dispatch::level_1`) has an `ObjectKind` for.
fn dispatch_kinds() -> BTreeSet<u8> {
    fd_core::Dispatch::level_1().values().into_iter().collect()
}

/// Everything wrong with `listed` for a map holding `values`, one line each.
fn value_problems(
    values: &BTreeSet<u8>,
    listed: &[Listed],
    routines: &BTreeSet<String>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for v in values {
        if !listed.iter().any(|l| l.value == *v) {
            problems.push(format!(
                "${v:02x} is in W1-1's object map but not in re/objects-W1-1.toml: measure it (T21's \
                 object_claims.rs) and list it with its porting task"
            ));
        }
    }
    let mut seen = BTreeSet::new();
    for l in listed {
        if !seen.insert(l.value) {
            problems.push(format!("${:02x} is listed twice", l.value));
        }
        if !["T22", "T24", "T25"].contains(&l.port_task.as_str()) {
            problems.push(format!(
                "${:02x}: port_task {} is not T22, T24 or T25",
                l.value, l.port_task
            ));
        }
        if l.routines.is_empty() {
            problems.push(format!("${:02x} names no routine", l.value));
        }
        for r in &l.routines {
            if !routines.contains(r) {
                problems.push(format!("${:02x}: {r} is not in re/translate.toml", l.value));
            }
        }
    }
    problems
}

#[test]
fn every_value_w1_1_can_spawn_is_listed_with_its_porting_task() {
    let values = w1_1_values();
    let listed = listed_values();
    let problems = value_problems(&values, &listed, &translate_names());
    assert!(
        problems.is_empty(),
        "re/objects-W1-1.toml:\n  {}",
        problems.join("\n  ")
    );
    let kinds = dispatch_kinds();
    let missing: Vec<String> = listed
        .iter()
        .filter(|l| !kinds.contains(&l.value))
        .map(|l| format!("${:02x} ({})", l.value, l.port_task))
        .collect();
    // A report, not a failure: the porting tasks add the kinds (DESIGN §4.1, `Event::Unported`).
    eprintln!(
        "values with no ObjectKind in fd-core's dispatch yet: {}",
        missing.join(", ")
    );
}

#[test]
fn a_value_missing_from_the_list_fails_naming_it() {
    let values = BTreeSet::from([0x01, 0x20, 0x77]);
    let listed = vec![
        Listed {
            value: 0x01,
            port_task: "T24".into(),
            routines: vec!["h0_681c".into()],
        },
        Listed {
            value: 0x20,
            port_task: "T08".into(),
            routines: vec!["no_such".into()],
        },
    ];
    let names = translate_names();
    let p = value_problems(&values, &listed, &names);
    assert!(
        p.iter()
            .any(|l| l.starts_with("$77 is in W1-1's object map")),
        "{p:?}"
    );
    assert!(p.iter().any(|l| l.contains("port_task T08")), "{p:?}");
    assert!(
        p.iter()
            .any(|l| l.contains("no_such is not in re/translate.toml")),
        "{p:?}"
    );
    assert_eq!(p.len(), 3, "{p:?}");
}

#[test]
fn the_values_file_reads_and_a_damaged_one_is_an_error() {
    let listed = listed_values();
    assert!(listed.len() > 20, "{listed:?}");
    assert!(
        parse_values("[[value]]\nvalue = 0x01\nport_task = \"T24\"\n")
            .unwrap_err()
            .contains("routines")
    );
    assert!(parse_values("[[value]]\nvalue = 0xzz\nport_task = \"T24\"\nroutines = []\n").is_err());
    assert!(parse_values("value = 1\n").is_err());
}
