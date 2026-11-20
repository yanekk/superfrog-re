//! The live file both halves agree on: its text (DESIGN §3.3 of the map-page plan), and its
//! writing, replaced whole by rename so a reader beside a thousand writes only ever sees whole
//! frames and no `.tmp` is left.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use amiga_routeview::{LiveFrame, write_live};

fn frame(n: u64) -> LiveFrame {
    LiveFrame {
        t_ms: 1_700_000_000_000 + n,
        playing: true,
        level: Some("W2-1".into()),
        actor: Some((100 + n as i32, -3)),
        cell: Some((7, 12)),
        actors: vec![(40, 50, "enemy".into())],
        // A different length each frame, so a torn read would show as a short or mixed text.
        cells: (0..(n % 37) as u16).map(|i| (i, 3, 0)).collect(),
        counters: vec![("items_left".into(), n as i64)],
    }
}

#[test]
fn the_frame_as_the_page_reads_it() {
    let f = LiveFrame { cells: vec![(3, 4, 0), (5, 6, 32)], ..frame(5) };
    assert_eq!(
        f.to_js(),
        "window.LIVE = {\"t\":1700000000005,\"playing\":true,\"level\":\"W2-1\",\"actor\":[105,-3],\"cell\":[7,12],\
\"actors\":[[40,50,\"enemy\"]],\"cells\":[[3,4,0],[5,6,32]],\"counters\":{\"items_left\":5}};\n"
    );
    let idle = LiveFrame { t_ms: 9, ..LiveFrame::default() };
    assert_eq!(
        idle.to_js(),
        "window.LIVE = {\"t\":9,\"playing\":false,\"level\":null,\"actor\":null,\"cell\":null,\"actors\":[],\"cells\":[],\"counters\":{}};\n"
    );
    let odd = LiveFrame { level: Some("a\"</script>".into()), ..LiveFrame::default() };
    assert!(odd.to_js().contains(r#""level":"a\"<\/script>""#));
}

#[test]
fn a_reader_beside_a_thousand_writes_sees_only_whole_frames() {
    let dir = std::env::temp_dir().join(format!("amiga-routeview-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    write_live(&dir, &frame(0)).unwrap();
    let whole: Vec<String> = (0..1000).map(|n| frame(n).to_js()).collect();
    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let (dir, stop, whole) = (dir.clone(), stop.clone(), whole.clone());
        std::thread::spawn(move || {
            let mut reads = 0u32;
            while !stop.load(Ordering::Relaxed) {
                let text = std::fs::read_to_string(dir.join("live.js")).expect("live.js is always there");
                assert!(whole.contains(&text), "a partial or mixed frame: {text:?}");
                reads += 1;
            }
            reads
        })
    };
    for n in 0..1000 {
        write_live(&dir, &frame(n)).unwrap();
    }
    stop.store(true, Ordering::Relaxed);
    let reads = reader.join().unwrap();
    assert!(reads > 0);
    assert_eq!(std::fs::read_to_string(dir.join("live.js")).unwrap(), whole[999]);
    assert!(!dir.join("live.js.tmp").exists());
    let names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names, ["live.js"]);
    let _ = std::fs::remove_dir_all(&dir);
}
