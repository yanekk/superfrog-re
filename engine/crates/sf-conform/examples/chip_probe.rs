//! T30's probe: replay the whole-game script on the original from power-on (df1 connected, as
//! `whole_game --check`) and, at each of the first frames from `from` that start a channel's
//! sample at `addr` (AUDxLCL written with its low word), print the original frame, the channel
//! and the sample's first 32 bytes in chip RAM. `chip_probe <script.txt> <from> <addr hex,…> <n>`:
//! `n` starts of each address.
use sf_conform::InputScript;
use sf_conform::sync::{Disks, core_memory, step_inserting};
use sf_core::layout::Ram;
use sf_disk::DiskSlot;
use sf_oracle::{Amiga, OracleConfig};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let s = InputScript::load(std::path::Path::new(&args[0])).expect("script");
    let from: u32 = args[1].parse().unwrap();
    let addrs: Vec<u32> = args[2].split(',').map(|a| u32::from_str_radix(a, 16).unwrap()).collect();
    let n: u32 = args[3].parse().unwrap();
    let cfg = OracleConfig::locate(400_000).unwrap().with_boot(DiskSlot::D1).with_drives(2);
    let mut a = Amiga::power_on(&cfg).unwrap();
    let mut disks: Option<Disks> = None;
    let mut seen = vec![0; addrs.len()];
    for f in 0..400_000 {
        let c = step_inserting(&mut a, s.input_at(f)).unwrap();
        let m = core_memory(a.memory());
        disks.get_or_insert_with(|| Disks::at(&m)).serve(&mut a);
        if f < from {
            continue;
        }
        for w in &c.paula {
            for (i, &addr) in addrs.iter().enumerate() {
                if (0xa2..0xe0).contains(&w.reg)
                    && (w.reg - 0xa2) % 0x10 == 0
                    && w.value == (addr & 0xffff) as u16
                    && seen[i] < n
                {
                    let bytes: Vec<String> = (addr..addr + 32).map(|x| format!("{:02x}", m.byte(x))).collect();
                    println!("original frame {f} ch {} ${addr:06x}: {}", (w.reg - 0xa0) / 0x10, bytes.join(" "));
                    seen[i] += 1;
                }
            }
        }
        if seen.iter().all(|&s| s >= n) {
            return;
        }
    }
}
