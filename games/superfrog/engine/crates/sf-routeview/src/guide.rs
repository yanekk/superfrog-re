//! An sf-plan waypoint file (`sf-plan --guide`, sf-plan `guide_file.rs`): `col,row` per line, in
//! the tile addressing (`W1-1:94,51` is `94,51`). A comment after it may carry `t <secs>` and
//! `est` (a waypoint traced from a video while the frog was not visible: its place estimated from
//! the camera), which the page shows.

#[derive(Debug, Clone, PartialEq)]
pub struct Waypoint {
    pub c: u16,
    pub r: u16,
    pub t: Option<f64>,
    pub est: bool,
}

/// The waypoints in `text`; lines that are not `col,row` (headers, blank lines) are skipped.
pub fn parse(text: &str) -> Vec<Waypoint> {
    text.lines()
        .filter_map(|line| {
            let (body, note) = line.split_once('#').unwrap_or((line, ""));
            let (c, r) = body.trim().split_once(',')?;
            let (c, r) = (c.trim().parse().ok()?, r.trim().parse().ok()?);
            let words: Vec<&str> = note.split_whitespace().collect();
            let t = words.iter().position(|w| *w == "t").and_then(|i| words.get(i + 1)?.parse().ok());
            Some(Waypoint { c, r, t, est: words.contains(&"est") })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_points_times_and_estimates() {
        let g = parse("# a video trace\n152,74  # t 7008.40 est\n152,67  # t 7008.80\n\n 3 , 4\nnot,a point\n");
        assert_eq!(
            g,
            vec![
                Waypoint { c: 152, r: 74, t: Some(7008.4), est: true },
                Waypoint { c: 152, r: 67, t: Some(7008.8), est: false },
                Waypoint { c: 3, r: 4, t: None, est: false },
            ]
        );
    }
}
