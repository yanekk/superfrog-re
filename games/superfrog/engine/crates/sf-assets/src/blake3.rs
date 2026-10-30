//! BLAKE3 (unkeyed, 32-byte output), after the specification's reference implementation. Written
//! here rather than taken from the `blake3` crate because that crate builds with `cc`, which the
//! purity test's allowlist cannot take (DESIGN §3.1). Checked against the crate in the cache tests.

const BLOCK_LEN: usize = 64;
const CHUNK_LEN: usize = 1024;

const CHUNK_START: u32 = 1;
const CHUNK_END: u32 = 2;
const PARENT: u32 = 4;
const ROOT: u32 = 8;

const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

const MSG_PERMUTATION: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

fn g(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    s[a] = s[a].wrapping_add(s[b]).wrapping_add(mx);
    s[d] = (s[d] ^ s[a]).rotate_right(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_right(12);
    s[a] = s[a].wrapping_add(s[b]).wrapping_add(my);
    s[d] = (s[d] ^ s[a]).rotate_right(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_right(7);
}

fn round(s: &mut [u32; 16], m: &[u32; 16]) {
    g(s, 0, 4, 8, 12, m[0], m[1]);
    g(s, 1, 5, 9, 13, m[2], m[3]);
    g(s, 2, 6, 10, 14, m[4], m[5]);
    g(s, 3, 7, 11, 15, m[6], m[7]);
    g(s, 0, 5, 10, 15, m[8], m[9]);
    g(s, 1, 6, 11, 12, m[10], m[11]);
    g(s, 2, 7, 8, 13, m[12], m[13]);
    g(s, 3, 4, 9, 14, m[14], m[15]);
}

fn compress(
    cv: &[u32; 8],
    block: &[u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    let mut s = [0u32; 16];
    s[..8].copy_from_slice(cv);
    s[8..12].copy_from_slice(&IV[..4]);
    s[12] = counter as u32;
    s[13] = (counter >> 32) as u32;
    s[14] = block_len;
    s[15] = flags;
    let mut m = *block;
    for r in 0..7 {
        round(&mut s, &m);
        if r < 6 {
            let mut p = [0u32; 16];
            for (k, w) in p.iter_mut().enumerate() {
                *w = m[MSG_PERMUTATION[k]];
            }
            m = p;
        }
    }
    for k in 0..8 {
        s[k] ^= s[k + 8];
        s[k + 8] ^= cv[k];
    }
    s
}

fn words(block: &[u8; BLOCK_LEN]) -> [u32; 16] {
    let mut w = [0u32; 16];
    for (k, x) in w.iter_mut().enumerate() {
        *x = u32::from_le_bytes(block[4 * k..4 * k + 4].try_into().unwrap());
    }
    w
}

fn first8(s: [u32; 16]) -> [u32; 8] {
    s[..8].try_into().unwrap()
}

/// What a compression would be given, held back until it is known whether it is the root.
struct Output {
    cv: [u32; 8],
    block: [u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
}

impl Output {
    fn chaining_value(&self) -> [u32; 8] {
        first8(compress(
            &self.cv,
            &self.block,
            self.counter,
            self.block_len,
            self.flags,
        ))
    }

    fn root_hash(&self) -> [u8; 32] {
        let s = compress(&self.cv, &self.block, 0, self.block_len, self.flags | ROOT);
        let mut out = [0u8; 32];
        for (k, w) in s[..8].iter().enumerate() {
            out[4 * k..4 * k + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }
}

fn parent(left: [u32; 8], right: [u32; 8]) -> Output {
    let mut block = [0u32; 16];
    block[..8].copy_from_slice(&left);
    block[8..].copy_from_slice(&right);
    Output {
        cv: IV,
        block,
        counter: 0,
        block_len: BLOCK_LEN as u32,
        flags: PARENT,
    }
}

/// One 1 KiB chunk (the last may be shorter, or empty for empty input).
fn chunk_output(chunk: &[u8], counter: u64) -> Output {
    let mut cv = IV;
    let mut blocks = chunk.chunks(BLOCK_LEN).peekable();
    let mut start = CHUNK_START;
    loop {
        let b = blocks.next().unwrap_or(&[]);
        let mut block = [0u8; BLOCK_LEN];
        block[..b.len()].copy_from_slice(b);
        if blocks.peek().is_none() {
            return Output {
                cv,
                block: words(&block),
                counter,
                block_len: b.len() as u32,
                flags: start | CHUNK_END,
            };
        }
        cv = first8(compress(
            &cv,
            &words(&block),
            counter,
            BLOCK_LEN as u32,
            start,
        ));
        start = 0;
    }
}

/// The BLAKE3 hash of `data`.
pub fn blake3(data: &[u8]) -> [u8; 32] {
    let mut chunks = data.chunks(CHUNK_LEN).peekable();
    let mut stack: Vec<[u32; 8]> = Vec::new();
    let mut counter = 0u64;
    loop {
        let c = chunks.next().unwrap_or(&[]);
        let out = chunk_output(c, counter);
        counter += 1;
        if chunks.peek().is_none() {
            // Fold the stack from the right into the last chunk; the final fold is the root.
            let mut out = out;
            while let Some(left) = stack.pop() {
                out = parent(left, out.chaining_value());
            }
            return out.root_hash();
        }
        // Merge completed subtrees: one per trailing zero bit of the chunks so far.
        let mut cv = out.chaining_value();
        let mut total = counter;
        while total & 1 == 0 {
            cv = parent(stack.pop().unwrap(), cv).chaining_value();
            total >>= 1;
        }
        stack.push(cv);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(h: [u8; 32]) -> String {
        h.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The specification's published hashes of the empty input and of `abc`.
    #[test]
    fn known_hashes() {
        assert_eq!(
            hex(blake3(b"")),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
        assert_eq!(
            hex(blake3(b"abc")),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    /// Every length around the block, chunk and tree boundaries agrees with the `blake3` crate.
    #[test]
    fn agrees_with_the_blake3_crate() {
        let data: Vec<u8> = (0..70_000u32).map(|k| (k % 251) as u8).collect();
        let mut lens = vec![
            0, 1, 63, 64, 65, 1023, 1024, 1025, 2048, 2049, 3072, 3073, 4096, 4097,
        ];
        lens.extend([5 * 1024 + 7, 8 * 1024, 31 * 1024 + 1, 65 * 1024, 70_000]);
        for n in lens {
            assert_eq!(
                blake3(&data[..n]),
                *::blake3::hash(&data[..n]).as_bytes(),
                "length {n}"
            );
        }
    }
}
