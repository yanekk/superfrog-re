//! Translator T26: every row of the measured OS surface (`re/os-calls.toml`, T01) is answered by the
//! machine (`pcode-machine`'s `os`) or is an explicit stop with its reason, and the machine's README
//! table says so. No disks: the files read are committed.

use std::fs;
use std::path::PathBuf;

use pcode_machine::os::{ANSWERED, Library};

fn read(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The `[[kind]]` tables of the generated TOML, each as its `key = value` lines.
fn tables(toml: &str, kind: &str) -> Vec<Vec<(String, String)>> {
    let head = format!("[[{kind}]]");
    let mut out = Vec::new();
    let mut cur: Option<Vec<(String, String)>> = None;
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with("[[") || line.starts_with('[') {
            out.extend(cur.take());
            if line == head {
                cur = Some(Vec::new());
            }
        } else if let (Some(t), Some((k, v))) = (cur.as_mut(), line.split_once(" = ")) {
            t.push((k.to_string(), v.trim_matches('"').to_string()));
        }
    }
    out.extend(cur);
    out
}

fn get<'a>(t: &'a [(String, String)], k: &str) -> &'a str {
    &t.iter()
        .find(|(key, _)| key == k)
        .unwrap_or_else(|| panic!("{k} in {t:?}"))
        .1
}

fn library(name: &str) -> Library {
    Library::ALL
        .into_iter()
        .find(|l| l.name() == name)
        .unwrap_or_else(|| panic!("{name}: a library the machine does not answer"))
}

#[test]
fn every_measured_call_is_answered_and_in_the_readme_table() {
    let calls = tables(&read("../../../re/os-calls.toml"), "call");
    assert_eq!(calls.len(), 8, "the committed surface's calls");
    let readme = read("../pcode-machine/README.md");
    for c in &calls {
        let (lib, name) = (library(get(c, "library")), get(c, "name"));
        let lvo: i32 = get(c, "lvo").parse().unwrap();
        assert!(
            ANSWERED.contains(&(lib, lvo, name)),
            "{} {name} ({lvo}) is not answered",
            lib.name()
        );
        let row = format!(
            "| {} `{name}` ({lvo}) |",
            lib.name().trim_end_matches(".library")
        );
        assert!(readme.contains(&row), "README lacks the row {row}");
    }
    // And the README lists nothing the machine does not answer.
    let rows = readme.lines().filter(|l| l.contains("` (-")).count();
    assert_eq!(rows, ANSWERED.len(), "README rows against ANSWERED");
}

#[test]
fn every_measured_os_memory_access_irq_and_cia_register_has_its_answer_written_down() {
    let surface = read("../../../re/os-calls.toml");
    // Prose wraps: compare with each line break a space.
    let readme = read("../pcode-machine/README.md").replace('\n', " ");
    for kind in ["read", "write"] {
        for t in tables(&surface, kind) {
            let what = get(&t, "what");
            let said = match what {
                "ExecBase" => "`$4` holds ExecBase",
                "graphics.library base" => "GfxBase's `copinit` (+$26) and `LOFlist` (+$32)",
                "graphics.library OpenFont result" => "`tf_CharData` (+$22) and `tf_Modulo` (+$26)",
                "level 2 autovector" | "level 3 autovector" => "the autovectors `$64..$7C`",
                other => panic!("{other}: no answer written down"),
            };
            assert!(readme.contains(said), "{what}: README lacks {said:?}");
        }
    }
    // The OS's interrupts and handlers are a stop with its reason, never run.
    for kind in ["irq", "vector"] {
        for t in tables(&surface, kind) {
            if get(&t, "owner") == "os" {
                assert!(
                    readme.contains("interrupt handler, not modelled`)"),
                    "the OS's interrupts: README lacks their stop"
                );
            }
        }
    }
    for t in tables(&surface, "cia") {
        let reg = get(&t, "reg");
        let name = reg[5..].to_uppercase();
        assert!(readme.contains(&name), "{reg}: README lacks {name}");
    }
}
