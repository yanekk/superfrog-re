//! The page: `index.html` (the viewer, with the routes' manifest in it) and `data/<id>.js` per
//! route. Script tags, not fetch, load the data, so the page also works opened from disk; a route
//! loads when it is picked, so two dozen levels' tiles never load at once.

use std::path::Path;

use crate::json::{Json, esc, list, strs};
use crate::verdict::Verdict;

const PAGE: &str = include_str!("page.html");

/// One route on the page.
pub struct Entry {
    /// The switcher's key and the data file's name: `[A-Za-z0-9._-]`.
    pub id: String,
    pub label: String,
    /// The switcher's row: a world (`W1`) or a level.
    pub group: String,
    /// The route file, shown under the title; `None` for a level drawn with no route.
    pub route_file: Option<String>,
    pub verdict: Verdict,
    /// The planner's planning time and route lengths, for the summary.
    pub wall_s: Option<f64>,
    pub ref_frames: Option<u32>,
}

/// `name` as a data file's id: anything but `[A-Za-z0-9._-]` becomes `_`.
pub fn id_of(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect()
}

/// Writes `data/<id>.js` for one route.
pub fn write_data(out: &Path, id: &str, json: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(out.join("data"))?;
    let js = format!("window.ROUTE_DATA=window.ROUTE_DATA||{{}};ROUTE_DATA[\"{}\"]={json};\n", esc(id));
    std::fs::write(out.join("data").join(format!("{id}.js")), js)
}

fn manifest(entries: &[Entry]) -> String {
    list(entries.iter().map(|e| {
        let mut j = Json::object();
        j.str("id", &e.id);
        j.str("label", &e.label);
        j.str("group", &e.group);
        j.str("status", e.verdict.status.word());
        j.raw("faults", &strs(e.verdict.faults.iter()));
        j.raw("info", &strs(e.verdict.info.iter()));
        j.str("data", &format!("data/{}.js", e.id));
        match &e.route_file {
            Some(f) => j.str("route_file", f),
            None => j.raw("route_file", "null"),
        }
        if e.wall_s.is_some() || e.ref_frames.is_some() {
            let mut b = Json::object();
            b.opt("wall_s", e.wall_s);
            b.opt("ref_frames", e.ref_frames);
            j.raw("bench", &b.finish());
        } else {
            j.raw("bench", "null");
        }
        j.finish()
    }))
}

/// Writes `index.html`. `bare` leaves out the `<!doctype>`/`<head>` skeleton, for a host that
/// adds its own (a published artifact); without it the file opens straight from disk.
pub fn write_index(out: &Path, entries: &[Entry], bare: bool) -> std::io::Result<std::path::PathBuf> {
    let mut page = PAGE.replace("__MANIFEST__", &manifest(entries));
    if !bare {
        page = format!(
            "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\">\
             <style>body{{margin:0}}</style></head><body>\n{page}\n</body></html>\n"
        );
    }
    std::fs::create_dir_all(out)?;
    let path = out.join("index.html");
    std::fs::write(&path, page)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::Status;

    #[test]
    fn ids_are_file_safe() {
        assert_eq!(id_of("w64 ordered/s3+"), "w64_ordered_s3_");
        assert_eq!(id_of("W1-4.v2"), "W1-4.v2");
    }

    #[test]
    fn the_manifest_sits_in_the_page_and_cannot_close_its_script() {
        let dir = std::env::temp_dir().join(format!("sf-routeview-site-{}", std::process::id()));
        let e = Entry {
            id: "W1-1".into(),
            label: "W1-1".into(),
            group: "W1".into(),
            route_file: Some("a</script>b.txt".into()),
            verdict: Verdict { status: Status::Faults, faults: vec!["lost 1 heart".into()], info: vec![] },
            wall_s: Some(60.0),
            ref_frames: None,
        };
        let p = write_index(&dir, &[e], false).unwrap();
        let html = std::fs::read_to_string(&p).unwrap();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains(r#""id":"W1-1","label":"W1-1","group":"W1","status":"faults","faults":["lost 1 heart"]"#));
        assert!(html.contains(r#"a<\/script>b.txt"#));
        assert!(!html.contains("__MANIFEST__"));
        let bare = std::fs::read_to_string(write_index(&dir, &[], true).unwrap()).unwrap();
        assert!(bare.starts_with("<title>"));
        write_data(&dir, "W1-1", "{\"a\":1}").unwrap();
        let js = std::fs::read_to_string(dir.join("data/W1-1.js")).unwrap();
        assert_eq!(js, "window.ROUTE_DATA=window.ROUTE_DATA||{};ROUTE_DATA[\"W1-1\"]={\"a\":1};\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
