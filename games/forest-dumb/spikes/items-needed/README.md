# Spike: where items_left's start value comes from (map-page T00, 2026-11-20)

Answer in [docs/10-object-system.md](../../docs/10-object-system.md), "Items": every `load_list_world_N`
calls `h0+$7a9e` (counts bytes `$20`-`$27` over all 38400 bytes of the object map at `h0+$cb38`),
subtracts `$32` and packs the result as BCD (`h4+$1c910`) into `items_left`. A pure function of
`_MapaBobowN`.

The code was throwaway and is deleted; it is in commit `34b06ade` (`git show 34b06ade`). To rerun:

1. Extract the disks (`cargo run -p fd-tools -- extract` in `engine/`).
2. From the maps: `from_maps.py` unpacks each `_MapaBobowN` with `tools/unpack.py` and prints the
   coin count, the count minus 50 and its BCD word.
3. On the oracle: copy `oracle_read.rs` to `engine/crates/fd-conform/examples/items_needed_spike.rs` and
   `cargo run --release -p fd-conform --example items_needed_spike` in `engine/` (about two minutes,
   needs Kickstart). It plays `cases/world-2-start.toml`'s input from power-on, pokes `world_index` =
   N-1 as `load_world` (`h0+$578`) is entered (armed at the sync frame 4990: the boot runs that
   address earlier), and prints `items_left` after `load_world` returns (`h0+$232`) and at `main_loop`'s
   first run (`h0+$252`). Delete the example afterwards.

Reading 2026-11-20, both ways equal: W1-1 282, W2-1 549, W3-1 676, W4-1 396, W5-1 547, W6-1 751.
