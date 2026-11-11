// T81's measurement of the original around mechanic-vanish's pass 330: each step's routine hits, the
// interrupted PC and registers, the cut read off them, the slots and the creature's cell; LINES=1
// adds every line sample. Not built: copy it to fd-conform/tests/ and run
// `[SCR=case] [FROM=irq] [TO=irq] [CELL=col,row] [LINES=1] cargo test -p fd-conform --test probe -- --nocapture`.
use std::path::Path;

use fd_conform::{Case, Inputs, Live, PASS_END, edges_cut, interrupted_pc, reach_case};
use fd_core::{HUNK_BASES, Mem, layout_var};

#[test]
fn probe() {
    let name = std::env::var("SCR").unwrap_or("mechanic-vanish".into());
    let from: u32 = std::env::var("FROM").map(|v| v.parse().unwrap()).unwrap_or(326);
    let to: u32 = std::env::var("TO").map(|v| v.parse().unwrap()).unwrap_or(333);
    let cellv: Vec<u32> = std::env::var("CELL")
        .unwrap_or("66,72".into())
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    let case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    let inputs = Inputs::load(&case).unwrap();
    let mut am = reach_case(&case, 0).unwrap();
    let irq = layout_var("irq_count").unwrap();
    let h0 = HUNK_BASES[0];
    let pcs: Vec<u32> = [
        0x844u32, 0x98a, 0x9a8, 0xabc, 0xb10, 0x681c, 0x6852, 0x6836, 0x2e0e, 0x2da, 0x302, 0x32a,
        0x352, 0x2b2, 0x1fec, 0x1efa,
    ]
    .iter()
    .map(|p| h0 + p)
    .collect();
    am.watch_pcs(&pcs);
    am.watch_write(Some(PASS_END));
    am.set_profile(true);
    for _ in 0..case.frames {
        let f = am.frame();
        let started = Live(&am).value(irq);
        am.step(inputs.at(f)).unwrap();
        let now = Live(&am).value(irq);
        if now < from || now > to {
            continue;
        }
        println!("=== frame {} irq {started}->{now}", am.frame());
        let hits: Vec<String> = am
            .pc_hits()
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|(p, n)| format!("{:x}:{n}", p - h0))
            .collect();
        println!("  hits {}", hits.join(" "));
        println!("  interrupted_pc {:x?} edges_cut {:?}", interrupted_pc(&am), edges_cut(&am));
        let samples = am.line_samples();
        let mut last = String::new();
        for s in &samples {
            let sup = s.sr & 0x2000 != 0;
            let rets: Vec<String> = s
                .stack
                .iter()
                .take(10)
                .map(|l| l.wrapping_sub(h0))
                .filter(|l| *l < 0xa000)
                .map(|l| format!("{l:x}"))
                .collect();
            let line = format!(
                "{} pc {:x} sp {:x} [{}]",
                if sup { "S" } else { "U" },
                s.pc.wrapping_sub(h0),
                s.sp,
                rets.join(",")
            );
            if line != last && std::env::var("LINES").is_ok() {
                println!("    v{:3} h{:3} {line}", s.vpos, s.hpos);
            }
            last = line;
        }
        // The interrupt frame's registers.
        let ret = h0 + 0x1efa;
        if let Some(s) = samples
            .iter()
            .filter(|s| s.vpos >= 233)
            .find(|s| s.stack.iter().any(|&l| l == ret))
        {
            let i = s.stack.iter().position(|&l| l == ret).unwrap();
            let regs: Vec<String> = s.stack[i + 1..i + 16].iter().map(|r| format!("{r:x}")).collect();
            println!("  regs at v{} d0-d7/a0-a6: {}", s.vpos, regs.join(" "));
        }
        let show = |m: &dyn Mem, what: &str| {
            let v = |n: &str| m.value(layout_var(n).unwrap());
            let act: Vec<u32> = (0..5).map(|i| v(&format!("creature_active+${i:x}"))).collect();
            let sl: Vec<String> = (0..10).map(|i| format!("{:x}", v(&format!("creature_slots+${:x}", i * 4)))).collect();
            println!("  {what}: active {act:?} slots {}", sl.join(" "));
        };
        show(&Live(&am), "step end");
        if let Some(mem) = am.watched_memory() {
            let m = fd_conform::OracleMem(&mem);
            println!("  pass end irq {}", m.value(irq));
            show(&m, "pass end");
        }
        let cell = h0 + 0xcb38 + cellv[0] * 96 + cellv[1];
        println!(
            "  cell {:?} {:x?} row {:x?} latches+flag {:x?} pos {:x?} edge_at {:x?}",
            cellv,
            am.peek(cell, 1),
            (0..24).map(|c| am.peek(h0 + 0xcb38 + (cellv[0] - 14 + c) * 96 + cellv[1], 1)[0]).collect::<Vec<_>>(),
            am.peek(h0 + 0x5196, 10),
            am.peek(h0 + 0x1e66, 4),
            am.peek(h0 + 0x5196 - 16, 16),
        );
    }
}
