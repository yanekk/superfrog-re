//! The workspace's purity test: the guard on the pure/shell boundary and on embedded tables
//! (plans/forest-dumb-start DESIGN §3.1), written from the first game's and the toolkit's with this
//! workspace's lists. Each workspace runs its own, from that workspace alone; the toolkit's checks
//! the toolkit crates' sources, so this one scans the pure crates' (`PURE`).
//!
//! The pure crates must not read a clock, touch a file, socket, thread, environment or process, use
//! floating point or a randomly-seeded hash map, or carry an integer table of more than 16 elements
//! (a table belongs on the player's disks). Their dependencies must be on `ALLOWED`, and no toolkit
//! crate may depend on a game's crate.
//!
//! The translator's generated code is held to the same rules (translator DESIGN §2.11, T34): the
//! tree `fd-translated` builds in (`FD_TRANSLATED`, else `build/translator/tree/`) when one exists,
//! and every committed file `pcode_rust.py` wrote. The one exemption is in [`code_ranges_end`].
//!
//! If this test fails, move the code to the shell side or read the data from the disks. Never relax
//! the test.

use std::path::{Path, PathBuf};
use std::process::Command;

const PURE: [&str; 5] = [
    "fd-core",
    "fd-assets",
    "pcode-rt",
    "pcode-machine",
    "fd-translated",
];

/// Crates a pure crate may depend on (normal and build edges, transitively). Each entry beyond the
/// pure crates needs one line saying why it is pure. The toolkit crates' sources are scanned by
/// the toolkit's own purity test.
const ALLOWED: &[&str] = &[
    "fd-core",
    "fd-assets",
    "pcode-rt",
    "pcode-machine",
    "fd-translated",
    "amiga-clock",
    "amiga-input",
    // The renderer's chips (T10): the copper run, the bitplane fetch and the sprite composition,
    // functions of the memory and registers handed to them. Pure by the toolkit's own purity test
    // (`toolkit-checks/tests/purity.rs`), which scans its sources.
    "amiga-chips",
    // amiga-chips' sound-side dependency: the mixer's chip-memory trait AmigaMemory implements; pure
    // by the same toolkit test (its resampler's floats are that test's one named exemption).
    "amiga-paula",
    // The recording format (T14): the toolkit's game-free recorder, its text and its zip, pure by the
    // toolkit's own purity test. Its zip is amiga-disk's, bytes in and bytes out.
    "amiga-record",
    "amiga-disk",
    // amiga-disk's DEFLATE decoder: bytes in, bytes out; no I/O, clock, thread or float.
    "miniz_oxide",
    // miniz_oxide's Adler-32 checksum: integer arithmetic over a byte slice.
    "adler2",
    // miniz_oxide's SIMD Adler-32, switched on by png in a whole-workspace build. Integer arithmetic
    // over a byte slice, no I/O.
    "simd-adler32",
    // The OFS reader behind amiga-disk's `ofs` module, built without its `std` feature: blocks in,
    // parsed volume out; no I/O, clock, thread or float, and no dependencies of its own.
    "amiga-ffs",
    // amiga-input's save-state derive: serde's traits and derive. The derive runs at compile time
    // and emits plain field-by-field code; the traits read and write values handed to them.
    "serde",
    "serde_core",
    "serde_derive",
    // serde_derive's compile-time parser and code generator: they run in the compiler, never in the game.
    "proc-macro2",
    "quote",
    "syn",
    "unicode-ident",
];

/// The name prefixes of the games' own crates, which no toolkit crate may depend on.
const GAME_PREFIX: [&str; 2] = ["sf-", "fd-"];

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
            // `0x18f32` is hex digits: a hex, binary or octal literal takes no float suffix
            if !hex && (text.ends_with("f32") || text.ends_with("f64")) {
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

/// Every violation in one hand-written source file, as `path:line: what`.
fn scan(rel: &str, src: &str) -> Vec<String> {
    scan_with(rel, src, false)
}

/// Every violation in one file `pcode_rust.py` generated: the same rules, with `CODE` exempt
/// ([`code_ranges_end`]).
fn scan_generated(rel: &str, src: &str) -> Vec<String> {
    scan_with(rel, src, true)
}

/// The first line of every file `pcode_rust.py` writes as a whole module or a tree's root.
fn is_generated(src: &str) -> bool {
    src.lines()
        .take(2)
        .any(|l| l == "// By pcode_rust.py. Do not edit: regenerate.")
}

/// A file by what wrote it: generated by `pcode_rust.py` or by hand.
fn scan_file(rel: &str, src: &str) -> Vec<String> {
    scan_with(rel, src, is_generated(src))
}

fn scan_with(rel: &str, src: &str, generated: bool) -> Vec<String> {
    let toks = lex(src);
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
        if generated && let Some(end) = code_ranges_end(&toks, k) {
            k = end + 1;
            continue;
        }
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
            Tok::Ident(n) if (n == "f32" || n == "f64") => {
                flag(line, format!("floating point (`{n}`)"))
            }
            Tok::Float => flag(line, "floating point (a float literal)".to_string()),
            Tok::ByteStr(len) if *len > MAX_TABLE => flag(
                line,
                format!(
                    "a {len}-byte byte-string literal (tables over {MAX_TABLE} elements come from the disks)"
                ),
            ),
            Tok::Punct('[') => {
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

/// The generated tree's `CODE`, if `const` at `open` starts it in exactly the shape
/// `pcode_rust.py` writes: `const CODE: &[(u32, u32, u32)] = &[(h, start, end), ..];`. Gives the
/// index of its closing `]`; anything else named `CODE`, or of another shape, is scanned as usual.
///
/// The one exemption from the table bar, owner's decision at T34 (2026-11-12), a deliberate
/// exception to DESIGN §3.1's "never relax": §2.11 bars tables because data in the code would be
/// a second copy of the game's data with nothing checking it against the disk. `CODE` is no game
/// data but the translator's index of the instruction ranges it emitted, beside them in the same
/// output and consistent by construction; `pcode-rt` stops a write into them. Applied to generated
/// files only (`scan_generated`).
fn code_ranges_end(toks: &[(Tok, usize)], open: usize) -> Option<usize> {
    const HEAD: &str = "const CODE : & [ ( u32 , u32 , u32 ) ] = & [";
    let mut k = open;
    for want in HEAD.split(' ') {
        let ok = match toks.get(k)?.0 {
            Tok::Ident(ref n) => n == want,
            Tok::Punct(c) => want.len() == 1 && want.starts_with(c),
            _ => false,
        };
        if !ok {
            return None;
        }
        k += 1;
    }
    let p = |k: usize, c: char| matches!(toks.get(k), Some((Tok::Punct(x), _)) if *x == c);
    let int = |k: usize| matches!(toks.get(k), Some((Tok::Int, _)));
    loop {
        if p(k, ']') {
            return p(k + 1, ';').then_some(k);
        }
        // one `(Int, Int, Int)`, then `,` or the closing `]`
        if !(p(k, '(') && int(k + 1) && p(k + 2, ',') && int(k + 3) && p(k + 4, ',') && int(k + 5))
        {
            return None;
        }
        k += 6;
        if p(k, ',') && p(k + 1, ')') {
            k += 1; // `(h, s, e,)`
        }
        if !p(k, ')') {
            return None;
        }
        k += 1;
        if p(k, ',') {
            k += 1;
        } else if !p(k, ']') {
            return None;
        }
    }
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
                "{krate} depends on `{n}`, which is not on ALLOWED in fd-conform/tests/purity.rs: \
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

fn toolkit_dir() -> PathBuf {
    crates_dir().join("../../../../toolkit")
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

/// `cargo tree` of `krate` in the workspace at `workspace`.
fn cargo_tree(workspace: &Path, krate: &str) -> String {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .current_dir(workspace)
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
        files.len() >= 3,
        "found only {} source files in the pure crates",
        files.len()
    );
    let mut violations = Vec::new();
    for f in &files {
        let rel = f
            .strip_prefix(crates_dir())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        violations.extend(scan_file(&rel, &std::fs::read_to_string(f).unwrap()));
    }
    assert!(violations.is_empty(), "{}", report(&violations));
}

/// The tree `fd-translated` builds in (`buildscripts/fd_translated.rs`): `FD_TRANSLATED` (a
/// directory or one file), else `build/translator/tree/` under `game` when it holds a `mod.rs`.
fn generated_tree(fd_translated: Option<&str>, game: &Path) -> Option<PathBuf> {
    match fd_translated {
        Some(p) if !p.is_empty() => Some(PathBuf::from(p)),
        _ => Some(game.join("build/translator/tree")).filter(|t| t.join("mod.rs").is_file()),
    }
}

#[test]
fn the_generated_tree_passes() {
    let game = crates_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let var = std::env::var("FD_TRANSLATED").ok();
    let Some(tree) = generated_tree(var.as_deref(), &game) else {
        // The stub fd-translated builds in instead holds no table (`buildscripts/fd_translated.rs`).
        println!(
            "scanned nothing: no generated tree (FD_TRANSLATED unset, no {}/build/translator/tree/mod.rs)",
            game.display()
        );
        return;
    };
    let mut files = Vec::new();
    if tree.is_dir() {
        rust_files(&tree, &mut files);
    } else {
        files.push(tree.clone());
    }
    assert!(!files.is_empty(), "no .rs files in {}", tree.display());
    let mut violations = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        violations.extend(scan_generated(&f.display().to_string(), &src));
    }
    println!(
        "scanned {} generated files in {}",
        files.len(),
        tree.display()
    );
    assert!(
        violations.is_empty(),
        "{}\nThe generated tree holds instructions only (DESIGN §2.11): fix pcode_rust.py, never the tree.",
        report(&violations)
    );
}

#[test]
fn the_real_pure_crates_depend_only_on_the_allowlist() {
    let workspace = crates_dir().parent().unwrap().to_path_buf();
    let mut violations = Vec::new();
    for k in PURE {
        violations.extend(deps_off_allowlist(k, &cargo_tree(&workspace, k), ALLOWED));
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Dependencies in `tree` that are a game's own crates.
fn game_crates(tree: &str) -> Vec<String> {
    tree_names(tree)
        .into_iter()
        .filter(|n| GAME_PREFIX.iter().any(|p| n.starts_with(p)))
        .collect()
}

#[test]
fn no_toolkit_crate_depends_on_a_game_crate() {
    // Every member of the toolkit's workspace, not only those this game uses today.
    let toolkit = toolkit_dir();
    let mut members: Vec<String> = std::fs::read_dir(toolkit.join("crates"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("Cargo.toml").is_file())
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    members.sort();
    assert!(
        members.iter().any(|m| m == "amiga-oracle"),
        "toolkit members not understood: {members:?}"
    );
    for k in &members {
        let tree = cargo_tree(&toolkit, k);
        assert!(
            tree_names(&tree).contains(k),
            "cargo tree output not understood: {tree}"
        );
        let bad = game_crates(&tree);
        assert!(
            bad.is_empty(),
            "{k}'s dependency tree contains {bad:?}; the toolkit depends on no game: {FIX}"
        );
    }
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
            flagged("fd-core/src/x.rs", src),
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
    let found = scan("fd-core/src/x.rs", &src);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_pure_crate_depending_off_the_allowlist_is_flagged() {
    let tree = "fd-core v0.1.0 (/x/crates/fd-core)\n\
                                amiga-clock v0.1.0 (/x/toolkit/crates/amiga-clock)\n\
                rand v0.9.0\n\
                rand_core v0.9.0 (*)\n";
    let found = deps_off_allowlist("fd-core", tree, ALLOWED);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].contains("`rand`") && found[0].contains(FIX),
        "{found:?}"
    );
    assert!(found[1].contains("`rand_core`"), "{found:?}");
    let clean = "fd-core v0.1.0 (/x)\namiga-clock v0.1.0 (/y)\n";
    assert!(deps_off_allowlist("fd-core", clean, ALLOWED).is_empty());
}

#[test]
fn a_game_crate_in_a_toolkit_tree_would_be_seen() {
    for prefix in GAME_PREFIX {
        let game = format!("{prefix}core");
        let tree = format!("amiga-chips v0.1.0 (/x)\n{game} v0.1.0 (/y)\ncc v1.2.0\n");
        assert_eq!(game_crates(&tree), [game]);
    }
    assert!(game_crates("amiga-chips v0.1.0 (/x)\nserde v1.0.0\n").is_empty());
}

// ---------------------------------------------------------------------------------------------
// Generated code (T34)
// ---------------------------------------------------------------------------------------------

const HEADER: &str = "// Generated from x.json; never committed when made from a game.\n\
                      // By pcode_rust.py. Do not edit: regenerate.\n";

/// `n` ranges as `pcode_rust.py` writes them.
fn ranges(n: u32) -> String {
    (0..n)
        .map(|i| format!("    (0, 0x{:x}, 0x{:x}),\n", i * 16, i * 16 + 10))
        .collect()
}

/// `CODE` as `pcode_rust.py` writes it, with `n` ranges.
fn code(n: u32) -> String {
    format!("pub const CODE: &[(u32, u32, u32)] = &[\n{}];\n", ranges(n))
}

#[test]
fn a_generated_file_with_a_seventeen_entry_table_fails() {
    let src = format!("{HEADER}{}const T: &[u32] = &{SEVENTEEN};\n", code(3));
    let found = scan_generated("tree/p00.rs", &src);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("17-element"), "{found:?}");
    assert_eq!(scan_file("tree/p00.rs", &src), found);
}

#[test]
fn the_generated_code_ranges_pass_only_in_generated_code() {
    let src = format!("{HEADER}{}", code(40));
    assert!(scan_generated("tree/mod.rs", &src).is_empty());
    assert!(scan_file("tree/mod.rs", &src).is_empty());
    // The same list in a hand-written file is a table.
    let hand = code(40);
    assert!(!is_generated(&hand));
    assert_eq!(scan_file("fd-core/src/x.rs", &hand).len(), 1);
    assert_eq!(scan("fd-core/src/x.rs", &src).len(), 1);
}

#[test]
fn a_second_long_list_beside_the_code_ranges_still_fails() {
    let src = format!(
        "{HEADER}{}pub const HUNK_SIZES: &[u32] = &{SEVENTEEN};\n{}",
        code(40),
        "fn f() { let _ = std::fs::read(\"x\"); let _ = 1.5; }\n"
    );
    let found = scan_generated("tree/mod.rs", &src);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found[0].contains("17-element"), "{found:?}");
}

#[test]
fn code_of_any_other_shape_still_fails() {
    let pairs: String = (0..17).map(|i| format!("({i}, {i}),")).collect();
    let quads: String = (0..17).map(|i| format!("(0, {i}, {i}, 1),")).collect();
    let mixed: String = (0..17)
        .map(|i| format!("(0, {i}, 0x{i:x}),"))
        .collect::<String>()
        + "(0, 1, -2),";
    let cases = [
        format!("pub const CODE: &[u32] = &{SEVENTEEN};"),
        format!("pub const CODE: &[(u32, u32)] = &[{pairs}];"),
        format!("pub const CODE: &[(u32, u32, u32, u32)] = &[{quads}];"),
        format!("pub const CODE: &[(u32, u32, u32)] = &[{quads}];"),
        format!("pub const CODE: &[(u32, u32, u32)] = &[{mixed}];"),
        format!("pub const CODE: [(u32, u32, u32); 17] = [{}];", ranges(17)),
        format!("pub static CODE: &[(u32, u32, u32)] = &[{}];", ranges(17)),
        format!("pub const RANGES: &[(u32, u32, u32)] = &[{}];", ranges(17)),
        format!("pub const CODE: &[(u32, u32, u32)] = &[{}]", ranges(17)),
        // A non-integer in any slot ends the exemption, so the usual scan sees the float.
        format!(
            "pub const CODE: &[(u32, u32, u32)] = &[{}(1.5, 0, 1)];",
            ranges(17)
        ),
        format!(
            "pub const CODE: &[(u32, u32, u32)] = &[{}(0, 1.5, 1)];",
            ranges(17)
        ),
        format!(
            "pub const CODE: &[(u32, u32, u32)] = &[{}(0, 0, 1.5)];",
            ranges(17)
        ),
    ];
    for c in &cases {
        let src = format!("{HEADER}{c}\n");
        assert!(
            !scan_generated("tree/mod.rs", &src).is_empty(),
            "not flagged: {c}"
        );
    }
}

#[test]
fn a_hex_literal_ending_in_f32_or_f64_is_an_integer() {
    // The generated tree's block addresses, `pc = (4, 0x18f32)`: hex digits, no float suffix.
    let src = "fn f() { let pc = (4, 0x18f32); let q = (0, 0x6f64); }\n";
    assert!(scan_generated("tree/p01.rs", src).is_empty());
    assert_eq!(
        scan("fd-core/src/x.rs", "const X: f32 = 1f32 + 2f64;\n").len(),
        3
    );
}

#[test]
fn the_generated_tree_is_the_one_fd_translated_builds_in() {
    let nowhere = Path::new("/nonexistent/game");
    assert_eq!(generated_tree(None, nowhere), None);
    assert_eq!(generated_tree(Some(""), nowhere), None);
    assert_eq!(
        generated_tree(Some("/t/tree.rs"), nowhere),
        Some(PathBuf::from("/t/tree.rs"))
    );
}
