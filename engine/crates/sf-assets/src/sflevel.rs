//! `.sflevel`: one level in a file of its own, the map editor's target (DESIGN §2.10, docs/14).
//!
//! JSON, with the level's files in their original formats, base64:
//!
//! ```text
//! {
//!   "format": "sflevel",
//!   "version": 1,
//!   "replaces": "W1-1",
//!   "chunk": {"disk": 1, "index": 21, "sector": 1012},
//!   "sections": {"t7mp": "VDdNUAAA…"}
//! }
//! ```
//!
//! The T7MP file is the one loader's input for disk levels and override files alike, so an
//! unchanged export plays as the disk level does by construction. `chunk` says where the level
//! came from and is optional; the engine does not read it. Sections other than `t7mp` are kept as
//! [`LevelData::extra`]. No JSON or base64 crate: the pure crates take no dependency for this.

use std::collections::BTreeMap;
use std::fmt;

use sf_disk::DiskSlot;

use crate::gamedata::{ChunkId, LevelData, LevelId, World};
use crate::t7mp::{t7mp_decode, t7mp_encode};

/// The `format` every `.sflevel` names.
pub const SFLEVEL_FORMAT: &str = "sflevel";
/// The version this build writes and the only one it reads.
pub const SFLEVEL_VERSION: u32 = 1;

/// Why a `.sflevel` cannot be played. [`fmt::Display`] is the reason the player is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SflevelError {
    /// Not JSON at all.
    NotJson,
    /// JSON, but not a level file: not an object, `format` is not `"sflevel"`, or a field is
    /// missing or of the wrong kind.
    WrongFormat,
    /// A version other than [`SFLEVEL_VERSION`].
    Version(u32),
    /// `replaces` names no level.
    UnknownLevel(String),
    /// A section the level needs is not there.
    MissingSection(String),
    /// The `t7mp` section is not a map the game can load.
    BadT7mp(String),
}

impl fmt::Display for SflevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SflevelError::NotJson => write!(f, "it is not a level file (not JSON)"),
            SflevelError::WrongFormat => {
                write!(f, "it is not a Superfrog level file (no \"format\": \"sflevel\" with its fields)")
            }
            SflevelError::Version(v) => write!(
                f,
                "it is a version {v} level file; this Superfrog reads version {SFLEVEL_VERSION}"
            ),
            SflevelError::UnknownLevel(s) => write!(f, "it replaces \"{s}\", which is not a level"),
            SflevelError::MissingSection(s) => write!(f, "its \"{s}\" section is missing"),
            SflevelError::BadT7mp(why) => write!(f, "its map is damaged: {why}"),
        }
    }
}

impl std::error::Error for SflevelError {}

/// A level id as `.sflevel` writes it: [`LevelId`]'s `Display` (`W1-1`, `W6-2alt`, `WB-1`). Every
/// id [`LevelId`]'s `FromStr` takes, and `W6-1alt`, W6's second set's first map, which the disks
/// hold too (the same file as W6-1).
pub fn level_id_named(s: &str) -> Option<LevelId> {
    if s == "W6-1alt" {
        return Some(LevelId {
            world: World::W6,
            level: 1,
            alt: true,
        });
    }
    s.parse().ok()
}

/// The file for `level`: [`SFLEVEL_VERSION`], every section, keys in a fixed order.
pub fn sflevel_encode(level: &LevelData) -> String {
    let c = level.chunk;
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"format\": \"{SFLEVEL_FORMAT}\",\n"));
    s.push_str(&format!("  \"version\": {SFLEVEL_VERSION},\n"));
    s.push_str(&format!("  \"replaces\": \"{}\",\n", level.id));
    s.push_str(&format!(
        "  \"chunk\": {{\"disk\": {}, \"index\": {}, \"sector\": {}}},\n",
        c.disk.number(),
        c.index,
        c.sector
    ));
    s.push_str("  \"sections\": {\n");
    let mut sections = vec![("t7mp".to_string(), t7mp_encode(&level.t7mp))];
    sections.extend(level.extra.iter().map(|(k, v)| (k.clone(), v.clone())));
    let last = sections.len() - 1;
    for (i, (name, bytes)) in sections.iter().enumerate() {
        s.push_str(&format!("    {}: \"{}\"", quote(name), base64_encode(bytes)));
        s.push_str(if i == last { "\n" } else { ",\n" });
    }
    s.push_str("  }\n}\n");
    s
}

/// Read a `.sflevel`. The checks go in the order of [`SflevelError`]'s variants, so a file wrong
/// in two ways gives the first.
pub fn sflevel_decode(s: &str) -> Result<LevelData, SflevelError> {
    let v = Json::parse(s).ok_or(SflevelError::NotJson)?;
    let top = v.object().ok_or(SflevelError::WrongFormat)?;
    if get(top, "format").and_then(Json::str) != Some(SFLEVEL_FORMAT) {
        return Err(SflevelError::WrongFormat);
    }
    let version = get(top, "version")
        .and_then(Json::int)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or(SflevelError::WrongFormat)?;
    if version != SFLEVEL_VERSION {
        return Err(SflevelError::Version(version));
    }
    let replaces = get(top, "replaces").and_then(Json::str).ok_or(SflevelError::WrongFormat)?;
    let id = level_id_named(replaces).ok_or_else(|| SflevelError::UnknownLevel(replaces.into()))?;
    let chunk = match get(top, "chunk") {
        None => ChunkId::new(DiskSlot::D1, 0, 0),
        Some(c) => read_chunk(c).ok_or(SflevelError::WrongFormat)?,
    };
    let sections = match get(top, "sections") {
        None => &[][..],
        Some(s) => s.object().ok_or(SflevelError::WrongFormat)?,
    };
    let mut t7mp = None;
    let mut extra = BTreeMap::new();
    for (name, value) in sections {
        let text = value.str().ok_or(SflevelError::WrongFormat)?;
        let bytes = base64_decode(text);
        if name == "t7mp" {
            t7mp = Some(bytes.ok_or_else(|| SflevelError::BadT7mp("the section is not base64".into()))?);
        } else {
            extra.insert(name.clone(), bytes.ok_or(SflevelError::WrongFormat)?);
        }
    }
    let bytes = t7mp.ok_or_else(|| SflevelError::MissingSection("t7mp".into()))?;
    let t7mp = t7mp_decode(&bytes, chunk, "the level file's map")
        .map_err(|e| SflevelError::BadT7mp(e.to_string()))?;
    // The game unpacks 84 rows of 300 cells whatever the file says (`map_unpack`, h0+$1647a).
    if (t7mp.width, t7mp.height) != (300, 84) {
        return Err(SflevelError::BadT7mp(format!(
            "the map is {}×{} blocks; the game plays only 300×84",
            t7mp.width, t7mp.height
        )));
    }
    Ok(LevelData {
        id,
        chunk,
        t7mp,
        extra,
    })
}

/// The level a file names, when it can be read that far: what the app offers to play from the
/// disks instead of a file it refuses (DESIGN §2.11).
pub fn sflevel_replaces(s: &str) -> Option<LevelId> {
    let v = Json::parse(s)?;
    level_id_named(get(v.object()?, "replaces")?.str()?)
}

fn read_chunk(c: &Json) -> Option<ChunkId> {
    let o = c.object()?;
    let n = |k| get(o, k).and_then(Json::int);
    let disk = DiskSlot::ALL
        .into_iter()
        .find(|d| Some(d.number() as i64) == n("disk"))?;
    Some(ChunkId::new(
        disk,
        u16::try_from(n("index")?).ok()?,
        u16::try_from(n("sector")?).ok()?,
    ))
}

fn get<'a>(o: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    o.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn quote(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            c if (c as u32) < 0x20 => q.push_str(&format!("\\u{:04x}", c as u32)),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

/// The 64 characters of base64 (RFC 4648), by value.
fn base64_char(v: u8) -> u8 {
    match v {
        0..=25 => b'A' + v,
        26..=51 => b'a' + (v - 26),
        52..=61 => b'0' + (v - 52),
        62 => b'+',
        _ => b'/',
    }
}

fn base64_value(c: u8) -> Option<u32> {
    Some(match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    } as u32)
}

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let b = |i: usize| group.get(i).copied().unwrap_or(0) as u32;
        let n = b(0) << 16 | b(1) << 8 | b(2);
        for k in 0..4 {
            if k <= group.len() {
                out.push(base64_char((n >> (18 - 6 * k)) as u8 & 0x3f));
            } else {
                out.push(b'=');
            }
        }
    }
    String::from_utf8(out).expect("ASCII")
}

/// Base64 with its padding; `None` for anything else (whitespace included).
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.as_bytes();
    if s.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let groups = s.len() / 4;
    for (g, quad) in s.chunks(4).enumerate() {
        let pad = quad.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && g + 1 != groups) {
            return None;
        }
        let mut n = 0u32;
        for &c in &quad[..4 - pad] {
            n = n << 6 | base64_value(c)?;
        }
        n <<= 6 * pad as u32;
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..3 - pad]);
    }
    Some(out)
}

/// A JSON value. Numbers keep their integer value when they are one (no floating point in the
/// pure crates); any other number is read and kept as `Num(None)`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Json {
    Null,
    Bool(bool),
    Num(Option<i64>),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn parse(s: &str) -> Option<Json> {
        let mut p = Parser {
            s: s.as_bytes(),
            at: 0,
            depth: 0,
        };
        let v = p.value()?;
        p.space();
        (p.at == p.s.len()).then_some(v)
    }

    fn object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(o) => Some(o),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    fn int(&self) -> Option<i64> {
        match self {
            Json::Num(n) => *n,
            _ => None,
        }
    }
}

/// How deep arrays and objects may nest: a hostile file cannot overflow the stack.
const MAX_DEPTH: u32 = 64;

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
    depth: u32,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.space();
        let hit = self.peek() == Some(c);
        if hit {
            self.at += 1;
        }
        hit
    }

    fn word(&mut self, w: &str, v: Json) -> Option<Json> {
        self.s[self.at..].starts_with(w.as_bytes()).then(|| {
            self.at += w.len();
            v
        })
    }

    fn value(&mut self) -> Option<Json> {
        self.space();
        match self.peek()? {
            b'n' => self.word("null", Json::Null),
            b't' => self.word("true", Json::Bool(true)),
            b'f' => self.word("false", Json::Bool(false)),
            b'"' => self.string().map(Json::Str),
            b'[' | b'{' => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return None;
                }
                let v = if self.peek() == Some(b'[') { self.array() } else { self.obj() };
                self.depth -= 1;
                v
            }
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn array(&mut self) -> Option<Json> {
        self.at += 1;
        let mut items = Vec::new();
        if self.eat(b']') {
            return Some(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            if self.eat(b']') {
                return Some(Json::Arr(items));
            }
            if !self.eat(b',') {
                return None;
            }
        }
    }

    fn obj(&mut self) -> Option<Json> {
        self.at += 1;
        let mut items = Vec::new();
        if self.eat(b'}') {
            return Some(Json::Obj(items));
        }
        loop {
            self.space();
            if self.peek() != Some(b'"') {
                return None;
            }
            let k = self.string()?;
            if !self.eat(b':') {
                return None;
            }
            items.push((k, self.value()?));
            if self.eat(b'}') {
                return Some(Json::Obj(items));
            }
            if !self.eat(b',') {
                return None;
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = self.s.get(self.at..self.at + 4)?;
        self.at += 4;
        u32::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let c = self.peek()?;
            self.at += 1;
            match c {
                b'"' => return Some(out),
                b'\\' => {
                    let e = self.peek()?;
                    self.at += 1;
                    out.push(match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&hi) {
                                if self.s.get(self.at..self.at + 2) != Some(b"\\u") {
                                    return None;
                                }
                                self.at += 2;
                                let lo = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&lo) {
                                    return None;
                                }
                                0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00)
                            } else {
                                hi
                            };
                            char::from_u32(code)?
                        }
                        _ => return None,
                    });
                }
                0..0x20 => return None,
                _ => {
                    // The rest of a UTF-8 character: `s` came from a `&str`, so it is whole.
                    let start = self.at - 1;
                    let len = match c {
                        0xf0.. => 4,
                        0xe0.. => 3,
                        0xc0.. => 2,
                        _ => 1,
                    };
                    self.at = start + len;
                    out.push_str(std::str::from_utf8(&self.s[start..self.at]).ok()?);
                }
            }
        }
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.at;
        let neg = self.peek() == Some(b'-');
        if neg {
            self.at += 1;
        }
        let digits = |p: &mut Self| {
            let from = p.at;
            while matches!(p.peek(), Some(b'0'..=b'9')) {
                p.at += 1;
            }
            p.at - from
        };
        let int_start = self.at;
        let n = digits(self);
        if n == 0 || (n > 1 && self.s[int_start] == b'0') {
            return None;
        }
        let mut whole = true;
        if self.peek() == Some(b'.') {
            self.at += 1;
            if digits(self) == 0 {
                return None;
            }
            whole = false;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if digits(self) == 0 {
                return None;
            }
            whole = false;
        }
        let text = std::str::from_utf8(&self.s[start..self.at]).ok()?;
        Some(Json::Num(if whole { text.parse().ok() } else { None }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_every_length_and_refuses_what_is_not_it() {
        let bytes: Vec<u8> = (0..=255).collect();
        for n in 0..40 {
            let enc = base64_encode(&bytes[..n]);
            assert_eq!(base64_decode(&enc).as_deref(), Some(&bytes[..n]), "{n}");
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
        assert_eq!(base64_encode(&[0xfb, 0xff]), "+/8=");
        for bad in ["TWF", "TW=u", "T===", "TQ==TWFu", "TW u", "TWE\n"] {
            assert_eq!(base64_decode(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn json_reads_what_the_format_needs_and_refuses_what_is_not_json() {
        let v = Json::parse(r#" {"a": [1, -2, 3.5, 1e3, true, null], "b": "x\"é😀é"} "#).unwrap();
        let o = v.object().unwrap();
        assert_eq!(
            get(o, "a"),
            Some(&Json::Arr(vec![
                Json::Num(Some(1)),
                Json::Num(Some(-2)),
                Json::Num(None),
                Json::Num(None),
                Json::Bool(true),
                Json::Null
            ]))
        );
        assert_eq!(get(o, "b").and_then(Json::str), Some("x\"é😀é"));
        for bad in ["", "{", "{\"a\" 1}", "{\"a\":1,}", "[1 2]", "01", "1.", "\"\n\"", "{} x", "nul"] {
            assert_eq!(Json::parse(bad), None, "{bad:?}");
        }
        assert_eq!(Json::parse(&"[".repeat(65)), None, "nested too deep");
        assert!(Json::parse(&format!("{}{}", "[".repeat(64), "]".repeat(64))).is_some());
    }

    #[test]
    fn a_level_id_is_written_and_read_back_including_w6_s_second_first_map() {
        for s in ["W1-1", "W6-4", "W6-1alt", "W6-3alt", "WB-1", "WW-1"] {
            assert_eq!(level_id_named(s).map(|i| i.to_string()).as_deref(), Some(s));
        }
        for bad in ["W7-1", "W1-5", "WB-2", "W5-1alt", "w1-1", ""] {
            assert_eq!(level_id_named(bad), None, "{bad}");
        }
    }
}
