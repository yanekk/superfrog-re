//! 10 000 random mutations of a real chunk: `decompress` never panics, and allocates nothing but
//! the output, at most the `decomp_size` the (mutated) header claims. Its own test binary, because
//! the counting allocator is process-wide.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use sf_disk::{AtnError, DiskSlot, Offered, carve, decompress, identify};

struct Counting;

thread_local! {
    /// Bytes allocated on this thread while counting, or `None` when not counting.
    static COUNTED: Cell<Option<usize>> = const { Cell::new(None) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = COUNTED.try_with(|c| c.set(c.get().map(|n| n + layout.size())));
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let _ = COUNTED.try_with(|c| c.set(c.get().map(|n| n + layout.size())));
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = COUNTED.try_with(|c| c.set(c.get().map(|n| n + new_size)));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

fn counted<T>(f: impl FnOnce() -> T) -> (T, usize) {
    COUNTED.with(|c| c.set(Some(0)));
    let out = f();
    let n = COUNTED.with(|c| c.replace(None)).unwrap();
    (out, n)
}

#[test]
fn ten_thousand_mutations_never_panic_or_overallocate() {
    let files: Vec<Offered> = sf_conform::disks::read_disk_files()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|(name, bytes)| Offered { name, bytes })
        .collect();
    let set = identify(&files).unwrap_or_else(|e| panic!("{e:?}"));
    // A small chunk that decodes as carved, so each run is cheap and most mutations reach the stream.
    let chunk = DiskSlot::ALL
        .iter()
        .flat_map(|&s| carve(set.disk(s)))
        .filter(|c| c.raw.len() >= c.comp_size as usize + 46 && c.decomp_size >= 2000)
        .min_by_key(|c| c.decomp_size)
        .unwrap();
    let (good, n) = counted(|| decompress(&chunk.raw));
    assert_eq!(good.as_ref().map(Vec::len), Ok(chunk.decomp_size as usize));
    assert_eq!(n, chunk.decomp_size as usize);

    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut kinds = std::collections::BTreeMap::<&str, u32>::new();
    for i in 0..10_000 {
        let mut b = chunk.raw.clone();
        match next() % 4 {
            // a few random bytes anywhere, header included
            0 | 1 => {
                for _ in 0..1 + next() % 8 {
                    let at = (next() as usize) % b.len();
                    b[at] = next() as u8;
                }
            }
            // one bit in the stream or trailer
            2 => {
                let at = 12 + (next() as usize) % (b.len() - 12);
                b[at] ^= 1 << (next() % 8);
            }
            // cut short
            _ => b.truncate((next() as usize) % b.len()),
        }
        let claimed = if b.len() >= 8 {
            u32::from_be_bytes(b[4..8].try_into().unwrap()) as usize
        } else {
            0
        };
        let (r, n) = counted(|| std::panic::catch_unwind(|| decompress(&b)));
        let r = r.unwrap_or_else(|_| panic!("mutation {i} panicked"));
        assert!(
            n <= claimed,
            "mutation {i}: allocated {n}, header claims {claimed}"
        );
        if let Ok(out) = &r {
            assert_eq!(out.len(), claimed, "mutation {i}");
        }
        *kinds
            .entry(match r {
                Ok(_) => "ok",
                Err(AtnError::BadMagic) => "bad magic",
                Err(AtnError::Truncated) => "truncated",
                Err(AtnError::Overrun { .. }) => "overrun",
                Err(AtnError::LengthMismatch { .. }) => "length",
                Err(AtnError::TooLarge { .. }) => "too large",
            })
            .or_default() += 1;
    }
    eprintln!("{kinds:?}");
    // the mutations reach the stream, not only the header checks
    assert!(
        kinds.contains_key("overrun") && kinds.contains_key("length"),
        "{kinds:?}"
    );
}
