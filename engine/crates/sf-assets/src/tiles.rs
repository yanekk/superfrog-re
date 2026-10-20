//! World graphics: the `LnBM` tiles, the `LnMS` masks and the level palette (docs/06, docs/10).

use crate::error::{AssetError, Reader};
use crate::gamedata::World;
use crate::palette::Palette;
use crate::sprites::SpriteBank;

/// Bytes per tile in `LnBM`: 16 × 16 pixels, 5 planes, per-tile plane-major (5 × 16 rows × 2).
pub const TILE_BYTES: usize = 160;

/// A world's tiles: 16 × 16 colour indices each (0..31), row-major. 840 for W1..W6 and the final
/// level, 640 for the bonus level (whose `LBBM` is 102 400 bytes).
#[derive(Clone, PartialEq, Eq)]
pub struct Tileset {
    pub tiles: Vec<[u8; 256]>,
}

impl std::fmt::Debug for Tileset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tileset({} tiles)", self.tiles.len())
    }
}

impl Tileset {
    pub(crate) fn read(r: &Reader, tiles: usize) -> Result<Tileset, AssetError> {
        r.size(tiles * TILE_BYTES)?;
        let tiles = r
            .data
            .chunks_exact(TILE_BYTES)
            .map(|t| {
                let mut px = [0u8; 256];
                for p in 0..5 {
                    for y in 0..16 {
                        let w = u16::from_be_bytes([t[(p * 16 + y) * 2], t[(p * 16 + y) * 2 + 1]]);
                        for x in 0..16 {
                            if w & (0x8000 >> x) != 0 {
                                px[y * 16 + x] |= 1 << p;
                            }
                        }
                    }
                }
                px
            })
            .collect();
        Ok(Tileset { tiles })
    }

    /// The `LnBM` bytes again.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.tiles.len() * TILE_BYTES];
        for (t, px) in self.tiles.iter().enumerate() {
            for p in 0..5 {
                for y in 0..16 {
                    let mut w = 0u16;
                    for x in 0..16 {
                        if px[y * 16 + x] >> p & 1 != 0 {
                            w |= 0x8000 >> x;
                        }
                    }
                    let o = t * TILE_BYTES + (p * 16 + y) * 2;
                    out[o..o + 2].copy_from_slice(&w.to_be_bytes());
                }
            }
        }
        out
    }
}

/// `LnMS`: one 16 × 16 one-plane mask per tile, 32 bytes each (a row per big-endian word, bit 15
/// leftmost). The game rewrites it as it loads it (docs/08: "via `h3+$22f30`, transformed"); what
/// a set bit means is open (inferred: the tile's opaque pixels), for T13 and T14.
#[derive(Clone, PartialEq, Eq)]
pub struct TileMasks {
    pub masks: Vec<[u16; 16]>,
}

impl std::fmt::Debug for TileMasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TileMasks({})", self.masks.len())
    }
}

impl TileMasks {
    pub(crate) fn read(r: &Reader) -> Result<TileMasks, AssetError> {
        r.size(840 * 32)?;
        let tiles = 840;
        let masks = (0..tiles)
            .map(|t| {
                let mut m = [0u16; 16];
                for (y, w) in m.iter_mut().enumerate() {
                    *w = r.u16(t * 32 + y * 2)?;
                }
                Ok(m)
            })
            .collect::<Result<_, AssetError>>()?;
        Ok(TileMasks { masks })
    }
}

/// Everything a world's levels are drawn with (docs/08 "Files, disks and level composition").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldGfx {
    pub world: World,
    /// `LnBM`.
    pub tiles: Tileset,
    /// `LnMS`; the bonus level has none.
    pub masks: Option<TileMasks>,
    /// `LnBO`: the world's bobs.
    pub bobs: SpriteBank,
    /// The level palette: the 32 colours after the world's `LnLP` loading picture, which the
    /// game fades in from `h3+$21900` (= the picture's `+$11300`) and keeps for the level (oracle:
    /// W1-1's copper list holds them). The final level loads no picture and keeps W6's (inferred).
    pub palette: Palette,
}
