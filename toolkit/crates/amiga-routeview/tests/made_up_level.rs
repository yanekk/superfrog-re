//! A made-up game's level through the whole page writer, with no disk: the site holds its data
//! file and an index whose manifest names it, and the page loads that data by script tag (not
//! fetch), so it works opened from disk.

use std::io::{self, Write};

use amiga_routeview::verdict::{Status, Verdict};
use amiga_routeview::{Category, Entry, LevelDump, PageSpec, write_site};

struct Level {
    id: &'static str,
    faults: Vec<String>,
}

impl LevelDump for Level {
    fn level_id(&self) -> &str {
        self.id
    }

    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        // Two 2×1 cells of a 2×2-pixel map, a track of three frames, one coin.
        write!(
            out,
            r##"{{"level":"{}","cols":2,"rows":1,"seed":1,"timing":"app","palette":["#000000","#ffffff"],"tiles":{{"0":"00010100"}},"body":[0,0],"objects":[[1,0,9,"coin","coin"]],"track":[[0,0],[1,0],[2,0]],"inputs":[0,8,8],"live":[[]],"events":[],"notes":[],"coins_needed":1,"end":null,"inputs_end":2}}"##,
            self.id
        )
    }

    fn entry(&self) -> Entry {
        Entry {
            id: self.id.into(),
            label: format!("Level {}", self.id),
            group: "Made up".into(),
            route_file: Some("routes/a.txt".into()),
            verdict: Verdict { status: Status::Faults, faults: self.faults.clone(), info: vec![] },
            wall_s: None,
            ref_frames: Some(3),
        }
    }
}

/// Uses the trait's default entry.
struct Bare;

impl LevelDump for Bare {
    fn level_id(&self) -> &str {
        "bare"
    }

    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        out.write_all(b"{}")
    }
}

#[test]
fn a_made_up_level_makes_a_site() {
    let root = std::env::temp_dir().join(format!("amiga-routeview-made-up-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let spec = PageSpec {
        title: "Made-up Game".into(),
        cell_size: (2, 2),
        categories: vec![
            Category::new("gems", "Gems", "--accent", true, &["coin"]),
            Category::new("rest", "Everything else", "#ffffff", true, &[]),
        ],
        actor: "hero".into(),
        anchor: (1, -1),
        cell_origin: (1, -2),
        live_box: (0, -2, 2, 2),
    };
    let a = Level { id: "L1", faults: vec!["lost 1 heart".into()] };
    write_site(&root, &spec, &[&a, &Bare]).unwrap();

    let js = std::fs::read_to_string(root.join("data/L1.js")).unwrap();
    assert!(js.starts_with("window.ROUTE_DATA=window.ROUTE_DATA||{};ROUTE_DATA[\"L1\"]={\"level\":\"L1\",\"cols\":2,"));
    assert_eq!(std::fs::read_to_string(root.join("data/bare.js")).unwrap(), "window.ROUTE_DATA=window.ROUTE_DATA||{};ROUTE_DATA[\"bare\"]={};\n");

    let html = std::fs::read_to_string(root.join("index.html")).unwrap();
    let manifest = html.split(r#"<script id="manifest" type="application/json">"#).nth(1).unwrap().split("</script>").next().unwrap();
    assert_eq!(
        manifest,
        concat!(
            r#"[{"id":"L1","label":"Level L1","group":"Made up","status":"faults","faults":["lost 1 heart"],"info":[],"data":"data/L1.js","route_file":"routes/a.txt","bench":{"wall_s":null,"ref_frames":3}},"#,
            r#"{"id":"bare","label":"bare","group":"bare","status":"clean","faults":[],"info":[],"data":"data/bare.js","route_file":null,"bench":null}]"#
        )
    );
    // Each manifest entry's data file exists, and the page loads it with a script tag.
    for f in ["data/L1.js", "data/bare.js"] {
        assert!(root.join(f).is_file(), "{f}");
    }
    assert!(html.contains("document.createElement('script'); s.src = e.data;"));
    assert!(!html.contains("fetch("), "fetch does not work from a page opened from disk");
    assert!(html.contains("<title>Made-up Game Route Viewer</title>") && html.contains("Follow hero"));
    assert!(html.contains(r#""cell":[2,2],"anchor":[1,-1],"cell_origin":[1,-2]"#));
    let _ = std::fs::remove_dir_all(&root);
}
