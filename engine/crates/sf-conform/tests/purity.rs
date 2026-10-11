//! The purity test: the guard on the pure/shell boundary (DESIGN §3.1) and on embedded tables (§2.3).
//!
//! The pure crates must not read a clock, touch a file, socket, thread, environment or process, use
//! floating point or a randomly-seeded hash map, or carry an integer table of more than 16 elements
//! (a table belongs on the player's disks). Their dependencies must be on `ALLOWED`, and the shipped
//! app must not depend on the test tooling (the shipped game may not contain an emulator).
//!
//! If this test fails, move the code to the shell side or read the data from the disks. Never relax
//! the test. The one exemption, `sf-paula/src/resample.rs` from the float and array rules, is the
//! owner's decision of 2026-10-10 and covers that file only.

use std::path::{Path, PathBuf};
use std::process::Command;

const PURE: [&str; 4] = ["sf-disk", "sf-assets", "sf-core", "sf-paula"];

/// Crates a pure crate may depend on (normal and build edges, transitively). Each entry beyond the
/// pure crates themselves needs one line saying why it is pure (DESIGN §5, Dependencies).
const ALLOWED: &[&str] = &["sf-disk", "sf-assets", "sf-core", "sf-paula"];

/// Shell crates that must never be in the shipped app's dependency tree (DESIGN §3.2).
const TEST_TOOLING: [&str; 3] = ["sf-oracle", "sf-conform", "sf-e2e"];

/// Exempt from the float and array rules only (DESIGN §3.1).
const RESAMPLER: &str = "sf-paula/src/resample.rs";

const FORBIDDEN_STD: [&str; 6] = ["fs", "time", "net", "thread", "env", "process"];
const FORBIDDEN_IDENTS: [&str; 4] = ["Instant", "SystemTime", "HashMap", "HashSet"];
const MAX_TABLE: usize = 16;
const INT_SUFFIXES: [&str; 12] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

const FIX: &str = "move the code, do not relax the test";

// ---------------------------------------------------------------------------------------------
// Lexer: just enough Rust to see identifiers, numbers and brackets outside comments and strings.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Int,
    Float,
    ByteStr(usize),
    Punct(char),
}

fn lex(src: &str) -> Vec<(Tok, usize)> {
    let s: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let (mut i, mut line) = (0, 1);
    let at = |i: usize| s.get(i).copied().unwrap_or('\0');
    while i < s.len() {
        let c = s[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && at(i + 1) == '/' {
            while i < s.len() && s[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && at(i + 1) == '*' {
            let mut depth = 0;
            while i < s.len() {
                if s[i] == '/' && at(i + 1) == '*' {
                    depth += 1;
                    i += 2;
                } else if s[i] == '*' && at(i + 1) == '/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    line += (s[i] == '\n') as usize;
                    i += 1;
                }
            }
        } else if let Some((len, end, nl)) = string_at(&s, i) {
            if c == 'b' {
                out.push((Tok::ByteStr(len), line));
            }
            line += nl;
            i = end;
        } else if c == '\'' {
            // a char literal ('x', '\n', '\u{..}') or a lifetime ('a)
            if at(i + 1) == '\\' {
                i += 3;
                while i < s.len() && s[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if at(i + 2) == '\'' {
                i += 3;
            } else {
                i += 1;
            }
        } else if c.is_ascii_digit() {
            let start = i;
            let after_dot = start > 0 && s[start - 1] == '.';
            let mut float = false;
            while i < s.len() && (s[i].is_ascii_alphanumeric() || s[i] == '_') {
                i += 1;
            }
            // `1.5`, but not `0..5`, `x.0.1` or `1.max(2)`
            if !after_dot && at(i) == '.' && at(i + 1).is_ascii_digit() {
                float = true;
                i += 1;
                while i < s.len() && (s[i].is_ascii_alphanumeric() || s[i] == '_') {
                    i += 1;
                }
            }
            let text: String = s[start..i].iter().collect();
            let hex = text.starts_with("0x") || text.starts_with("0b") || text.starts_with("0o");
            // an integer suffix may hold an `e` (`5usize`), an exponent never follows one
            let digits = INT_SUFFIXES
                .iter()
                .find_map(|x| text.strip_suffix(x))
                .unwrap_or(&text);
            if !hex && (digits.contains('e') || digits.contains('E')) {
                float = true; // 1e3
            }
            if text.ends_with("f32") || text.ends_with("f64") {
                float = true;
            }
            out.push((if float { Tok::Float } else { Tok::Int }, line));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                i += 1;
            }
            out.push((Tok::Ident(s[start..i].iter().collect()), line));
        } else {
            out.push((Tok::Punct(c), line));
            i += 1;
        }
    }
    out
}

/// A string literal starting at `i` ("..", b"..", r#".."#, br".."): (byte length, end, newlines).
fn string_at(s: &[char], i: usize) -> Option<(usize, usize, usize)> {
    let at = |j: usize| s.get(j).copied().unwrap_or('\0');
    let mut j = i;
    if i > 0 && (s[i - 1].is_alphanumeric() || s[i - 1] == '_') {
        return None; // inside an identifier
    }
    if at(j) == 'b' || at(j) == 'c' {
        j += 1;
    }
    let raw = at(j) == 'r';
    if raw {
        j += 1;
    }
    let mut hashes = 0;
    while raw && at(j) == '#' {
        hashes += 1;
        j += 1;
    }
    if at(j) != '"' {
        return None;
    }
    j += 1;
    let (mut len, mut nl) = (0, 0);
    loop {
        if j >= s.len() {
            return Some((len, j, nl));
        }
        let c = s[j];
        if c == '"' && (0..hashes).all(|k| at(j + 1 + k) == '#') {
            return Some((len, j + 1 + hashes, nl));
        }
        if c == '\\' && !raw {
            // \xNN, \n, \u{..}, or a line continuation; each escape is one unit
            j += 2;
            if at(j - 1) == 'x' {
                j += 2;
            } else if at(j - 1) == 'u' {
                while j < s.len() && s[j] != '}' {
                    j += 1;
                }
                j += 1;
            } else if at(j - 1) == '\n' {
                nl += 1;
                continue;
            }
            len += 1;
            continue;
        }
        nl += (c == '\n') as usize;
        len += c.len_utf8();
        j += 1;
    }
}

// ---------------------------------------------------------------------------------------------
// The rules
// ---------------------------------------------------------------------------------------------

/// Every violation in one source file, as `path:line: what`. `rel` is relative to `engine/crates`.
fn scan(rel: &str, src: &str) -> Vec<String> {
    let toks = lex(src);
    let exempt = rel == RESAMPLER;
    let mut found = Vec::new();
    let mut flag = |line: usize, what: String| found.push(format!("{rel}:{line}: {what}"));
    let is = |k: usize, p: char| matches!(toks.get(k), Some((Tok::Punct(c), _)) if *c == p);
    let ident = |k: usize| match toks.get(k) {
        Some((Tok::Ident(n), _)) => Some(n.as_str()),
        _ => None,
    };

    let mut k = 0;
    while k < toks.len() {
        let (tok, line) = (&toks[k].0, toks[k].1);
        match tok {
            Tok::Ident(n) if n == "std" && is(k + 1, ':') && is(k + 2, ':') => {
                if let Some(m) = ident(k + 3).filter(|m| FORBIDDEN_STD.contains(m)) {
                    flag(line, format!("`std::{m}`"));
                } else if is(k + 3, '{') {
                    // use std::{io, fs::File, ...}: the first segment of each path in the group
                    let mut depth = 0;
                    let mut j = k + 3;
                    while j < toks.len() {
                        if is(j, '{') {
                            depth += 1;
                        } else if is(j, '}') {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        } else if let Some(m) = ident(j) {
                            let first = !(is(j - 1, ':') && j >= 2 && is(j - 2, ':'));
                            if first && FORBIDDEN_STD.contains(&m) {
                                flag(toks[j].1, format!("`std::{m}`"));
                            }
                        }
                        j += 1;
                    }
                }
            }
            Tok::Ident(n) if FORBIDDEN_IDENTS.contains(&n.as_str()) => flag(line, format!("`{n}`")),
            Tok::Ident(n) if (n == "f32" || n == "f64") && !exempt => {
                flag(line, format!("floating point (`{n}`)"))
            }
            Tok::Float if !exempt => flag(line, "floating point (a float literal)".to_string()),
            Tok::ByteStr(len) if *len > MAX_TABLE && !exempt => flag(
                line,
                format!(
                    "a {len}-byte byte-string literal (tables over {MAX_TABLE} elements come from the disks)"
                ),
            ),
            Tok::Punct('[') if !exempt => {
                if let Some((count, end)) = int_table(&toks, k)
                    && count > MAX_TABLE
                {
                    flag(
                        line,
                        format!(
                            "a {count}-element integer array literal (tables over {MAX_TABLE} elements come from the disks)"
                        ),
                    );
                    k = end;
                }
            }
            _ => {}
        }
        k += 1;
    }
    found
}

/// If the bracket at `open` holds only integer literals (with `-`, `,`, `(`, `)` and nested
/// brackets), the number of integers and the index of the matching `]`.
fn int_table(toks: &[(Tok, usize)], open: usize) -> Option<(usize, usize)> {
    let (mut depth, mut count) = (0, 0);
    for (j, (t, _)) in toks.iter().enumerate().skip(open) {
        match t {
            Tok::Punct('[') => depth += 1,
            Tok::Punct(']') => {
                depth -= 1;
                if depth == 0 {
                    return (count > 0).then_some((count, j));
                }
            }
            Tok::Int => count += 1,
            Tok::Punct(',' | '-' | '(' | ')') => {}
            _ => return None,
        }
    }
    None
}

fn report(violations: &[String]) -> String {
    format!(
        "{} purity violation(s) in the pure crates (DESIGN §3.1, §2.3):\n  {}\n\
         Fix: {FIX}. Put clocks, files, threads, floats and hash maps on the shell side; read tables \
         from the player's disks.",
        violations.len(),
        violations.join("\n  ")
    )
}

/// The packages a crate's dependency tree names (normal and build edges), from `cargo tree` output.
fn tree_names(tree: &str) -> Vec<String> {
    let mut names: Vec<String> = tree
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| !n.is_empty() && !n.starts_with('['))
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Dependencies of pure crate `krate` that are not on `ALLOWED`.
fn deps_off_allowlist(krate: &str, tree: &str, allowed: &[&str]) -> Vec<String> {
    tree_names(tree)
        .into_iter()
        .filter(|n| n != krate && !allowed.contains(&n.as_str()))
        .map(|n| {
            format!(
                "{krate} depends on `{n}`, which is not on ALLOWED in sf-conform/tests/purity.rs: \
                 {FIX}, or if `{n}` is pure, add it to ALLOWED with one line saying why"
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Running it on the real workspace
// ---------------------------------------------------------------------------------------------

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let p = e.unwrap().path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n != "target") {
                rust_files(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn cargo_tree(krate: &str) -> String {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .current_dir(crates_dir().parent().unwrap())
        .args([
            "tree",
            "--offline",
            "--quiet",
            "-p",
            krate,
            "-e",
            "normal,build",
            "--prefix",
            "none",
        ])
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("run cargo tree");
    assert!(
        out.status.success(),
        "cargo tree -p {krate}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_real_pure_crates_pass() {
    let mut files = Vec::new();
    for k in PURE {
        rust_files(&crates_dir().join(k), &mut files);
    }
    assert!(
        files.len() >= PURE.len(),
        "found only {} source files",
        files.len()
    );
    let mut violations = Vec::new();
    for f in &files {
        let rel = f
            .strip_prefix(crates_dir())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        violations.extend(scan(&rel, &std::fs::read_to_string(f).unwrap()));
    }
    assert!(violations.is_empty(), "{}", report(&violations));
}

#[test]
fn the_real_pure_crates_depend_only_on_the_allowlist() {
    let mut violations = Vec::new();
    for k in PURE {
        violations.extend(deps_off_allowlist(k, &cargo_tree(k), ALLOWED));
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn the_app_does_not_depend_on_test_tooling() {
    let names = tree_names(&cargo_tree("sf-app"));
    assert!(
        names.contains(&"sf-app".to_string()),
        "cargo tree output not understood: {names:?}"
    );
    let bad: Vec<_> = names
        .iter()
        .filter(|n| TEST_TOOLING.contains(&n.as_str()))
        .collect();
    assert!(
        bad.is_empty(),
        "sf-app's dependency tree contains {bad:?}; the shipped game may not contain an emulator or \
         test tooling (DESIGN §3.2): {FIX}"
    );
}

// ---------------------------------------------------------------------------------------------
// The rules, on fixtures
// ---------------------------------------------------------------------------------------------

fn flagged(rel: &str, src: &str) -> bool {
    !scan(rel, src).is_empty()
}

const SIXTEEN: &str = "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]";
const SEVENTEEN: &str = "[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]";

#[test]
fn each_forbidden_pattern_is_flagged() {
    let cases = [
        "use std::fs;",
        "fn f() { let _ = std::fs::read(\"x\"); }",
        "use std::time::Duration;",
        "use std::net::TcpStream;",
        "use std::thread;",
        "fn f() { std::env::var(\"X\"); }",
        "use std::process::Command;",
        "use std::{io, fs::File};",
        "use std::{collections::BTreeMap, time};",
        "fn f(t: Instant) {}",
        "fn f() -> SystemTime { todo!() }",
        "use std::collections::HashMap;",
        "fn f(s: HashSet<u8>) {}",
        "fn f(x: f32) {}",
        "fn f() -> f64 { 0 }",
        "const X: u32 = 1.5 as u32;",
        "const X: u32 = 1e3 as u32;",
        "const X: u32 = 2f32 as u32;",
        "const X: u32 = 1E3 as u32;",
        "const X: u32 = 2e-3 as u32;",
        &format!("const T: [u8; 17] = {SEVENTEEN};"),
        "const T: [i16; 17] = [-1, 0x2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 0b1_0001];",
        "const T: [[u8; 3]; 6] = [[1, 2, 3], [1, 2, 3], [1, 2, 3], [1, 2, 3], [1, 2, 3], [1, 2, 3]];",
        "const T: &[u8] = &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];",
        "const T: &[u8] = b\"0123456789abcdefg\";",
    ];
    for src in cases {
        assert!(
            flagged("sf-core/src/x.rs", src),
            "not flagged in a pure crate: {src}"
        );
    }
}

#[test]
fn ordinary_pure_code_is_not_flagged() {
    let src = r##"
        // HashMap, std::fs and 1.5 in a comment are fine
        /* so is f32 /* nested */ here */
        use std::collections::BTreeMap;
        use std::{io, collections::BTreeSet};
        const MAGIC: &[u8; 4] = b"ATN!";
        const MSG: &str = "uses no HashMap, f64 or std::fs";
        const RAW: &str = r#"Instant"#;
        const ZEROES: [u8; 1024] = [0; 1024];
        const SIX: [u16; 16] = SIXTEEN_HERE;
        fn f<'a>(t: &'a (u8, (u8, u8)), c: char) -> u32 {
            let _ = (0..10).len() + t.1.0 as usize + 1.max(2);
            let _ = ['a', 'b', '\n'];
            let _ = vec![1u8; 300];
            let _ = 5usize + 3isize as usize + 1_000_usize;
            0x1e3 + 0b1e0_u32.count_ones() + c as u32
        }
    "##
    .replace("SIXTEEN_HERE", SIXTEEN);
    let found = scan("sf-core/src/x.rs", &src);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn only_the_resampler_is_exempt_from_the_float_and_array_rules() {
    let float = "fn tap(x: f32) -> f64 { x as f64 * 0.5 }";
    let table = format!("const FIR: [i32; 17] = {SEVENTEEN};");
    let long_bytes = "const FIR: &[u8] = b\"0123456789abcdefg\";";
    for src in [float, &table, long_bytes] {
        assert!(!flagged(RESAMPLER, src), "the resampler is exempt: {src}");
        for other in [
            "sf-paula/src/lib.rs",
            "sf-paula/src/resample/mod.rs",
            "sf-core/src/resample.rs",
        ] {
            assert!(
                flagged(other, src),
                "only the resampler is exempt, not {other}: {src}"
            );
        }
    }
    // The exemption covers floats and tables only, never clocks, files or hash maps.
    for src in [
        "use std::time::Instant;",
        "use std::fs;",
        "use std::collections::HashMap;",
    ] {
        assert!(
            flagged(RESAMPLER, src),
            "the resampler is not exempt from: {src}"
        );
    }
}

#[test]
fn a_violation_names_the_place_and_the_fix() {
    let found = scan(
        "sf-core/src/frog.rs",
        "fn f() {}\nuse std::collections::HashMap;\n",
    );
    assert_eq!(found, vec!["sf-core/src/frog.rs:2: `HashMap`".to_string()]);
    assert!(report(&found).contains("move the code, do not relax the test"));
    assert!(found.len() == 1 && report(&found).contains("sf-core/src/frog.rs:2"));
}

#[test]
fn a_pure_crate_depending_off_the_allowlist_is_flagged() {
    let tree = "sf-core v0.1.0 (/x/engine/crates/sf-core)\n\
                sf-assets v0.1.0 (/x/engine/crates/sf-assets)\n\
                sf-disk v0.1.0 (/x/engine/crates/sf-disk)\n\
                rand v0.9.0\n\
                rand_core v0.9.0 (*)\n";
    let found = deps_off_allowlist("sf-core", tree, ALLOWED);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].contains("`rand`") && found[0].contains(FIX),
        "{found:?}"
    );
    assert!(found[1].contains("`rand_core`"), "{found:?}");
    let clean = "sf-assets v0.1.0 (/x)\nsf-disk v0.1.0 (/y)\n";
    assert!(deps_off_allowlist("sf-assets", clean, ALLOWED).is_empty());
}

#[test]
fn test_tooling_in_the_app_tree_would_be_seen() {
    let tree = "sf-app v0.1.0 (/x)\nsf-oracle v0.1.0 (/y)\ncc v1.2.0\n";
    let names = tree_names(tree);
    assert!(names.iter().any(|n| TEST_TOOLING.contains(&n.as_str())));
}
