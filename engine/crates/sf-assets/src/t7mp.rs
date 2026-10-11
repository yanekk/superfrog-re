//! `T7MP` levels (docs/04): a flat run of `[4-byte tag][u32 length][payload]`, lossless both ways.
//!
//! The four tags that carry what [`T7mp`] types (`T7MP` = the file size, `XBLK`, `YBLK`, `BODY`)
//! keep their place in [`T7mp::tags`] with an empty payload; [`t7mp_encode`] writes them from the
//! typed fields, so an edited width, height or body is what gets written. A file whose four tags
//! are not in that canonical form is refused rather than half-kept, so `encode(decode(x)) == x`.

use crate::error::{AssetError, Reader, Why};
use crate::gamedata::ChunkId;

/// A tag's four bytes as stored (some shipped levels have a tag name zeroed out; docs/04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    pub const T7MP: Tag = Tag(*b"T7MP");
    pub const XBLK: Tag = Tag(*b"XBLK");
    pub const YBLK: Tag = Tag(*b"YBLK");
    pub const BODY: Tag = Tag(*b"BODY");

    fn typed(self) -> bool {
        matches!(self, Tag::T7MP | Tag::XBLK | Tag::YBLK | Tag::BODY)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct T7mp {
    /// Map width in blocks (`XBLK`).
    pub width: u16,
    /// Map height in blocks (`YBLK`).
    pub height: u16,
    /// `BODY`: `width × height` cells, row-major, each `tile (low 10 bits) | marker (high 6)`.
    pub body: Vec<u16>,
    /// Every tag in file order; the four typed ones with an empty payload.
    pub tags: Vec<(Tag, Vec<u8>)>,
}

impl std::fmt::Debug for T7mp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<String> = self
            .tags
            .iter()
            .map(|(t, _)| String::from_utf8_lossy(&t.0).into_owned())
            .collect();
        write!(f, "T7mp({}x{}, {names:?})", self.width, self.height)
    }
}

impl T7mp {
    /// The payload of the first `tag`.
    pub fn tag(&self, tag: Tag) -> Option<&[u8]> {
        self.tags
            .iter()
            .find(|(t, _)| *t == tag)
            .map(|(_, p)| &p[..])
    }

    pub fn cell(&self, col: u16, row: u16) -> u16 {
        self.body[row as usize * self.width as usize + col as usize]
    }
}

/// Decode a T7MP file. `chunk` and `what` name it in an error.
pub fn t7mp_decode(bytes: &[u8], chunk: ChunkId, what: &str) -> Result<T7mp, AssetError> {
    let r = Reader::new(bytes, chunk, what);
    let bad = |m: String| r.err(Why::Malformed(m));
    if bytes.get(..4) != Some(b"T7MP") {
        return Err(bad("does not start with T7MP".into()));
    }
    let mut tags = Vec::new();
    let (mut width, mut height, mut body) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        let tag = Tag(r.bytes(at, 4)?.try_into().unwrap());
        let len = r.u32(at + 4)? as usize;
        let payload = r.bytes(at + 8, len)?;
        let long = || (len == 4).then(|| u32::from_be_bytes(payload.try_into().unwrap()));
        let name = String::from_utf8_lossy(&tag.0).into_owned();
        let dup = tags.iter().any(|(t, _): &(Tag, Vec<u8>)| *t == tag);
        if tag.typed() && dup {
            return Err(bad(format!("a second {name} tag at byte {at}")));
        }
        match tag {
            Tag::T7MP if at != 0 || long() != Some(bytes.len() as u32) => {
                return Err(bad(format!(
                    "T7MP tag does not give the file size {}",
                    bytes.len()
                )));
            }
            Tag::XBLK | Tag::YBLK => {
                let v = long()
                    .and_then(|v| u16::try_from(v).ok())
                    .ok_or_else(|| bad(format!("{name} is not a 4-byte size below 65536")))?;
                if tag == Tag::XBLK {
                    width = Some(v);
                } else {
                    height = Some(v);
                }
            }
            Tag::BODY => body = Some((at, payload)),
            _ => {}
        }
        tags.push((
            tag,
            if tag.typed() {
                Vec::new()
            } else {
                payload.to_vec()
            },
        ));
        at += 8 + len;
    }
    let (width, height) = width
        .zip(height)
        .ok_or_else(|| bad("no XBLK or no YBLK".into()))?;
    let (body_at, body) = body.ok_or_else(|| bad("no BODY".into()))?;
    let cells = width as usize * height as usize;
    if body.len() != cells * 2 {
        return Err(bad(format!(
            "BODY at byte {body_at} has {} bytes, {width} x {height} cells need {}",
            body.len(),
            cells * 2
        )));
    }
    Ok(T7mp {
        width,
        height,
        body: body
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect(),
        tags,
    })
}

/// The T7MP file for `t`: lossless, `t7mp_encode(&t7mp_decode(x)?) == x` (for T28's `.sflevel`).
/// The size tag, `XBLK`, `YBLK` and `BODY` are written from the typed fields.
pub fn t7mp_encode(t: &T7mp) -> Vec<u8> {
    let payload = |tag: Tag, p: &[u8], total: u32| -> Vec<u8> {
        match tag {
            Tag::T7MP => total.to_be_bytes().to_vec(),
            Tag::XBLK => (t.width as u32).to_be_bytes().to_vec(),
            Tag::YBLK => (t.height as u32).to_be_bytes().to_vec(),
            Tag::BODY => t.body.iter().flat_map(|c| c.to_be_bytes()).collect(),
            _ => p.to_vec(),
        }
    };
    let total: usize = t
        .tags
        .iter()
        .map(|(tag, p)| 8 + payload(*tag, p, 0).len())
        .sum();
    let mut out = Vec::with_capacity(total);
    for (tag, p) in &t.tags {
        let body = payload(*tag, p, total as u32);
        out.extend_from_slice(&tag.0);
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
    }
    out
}
