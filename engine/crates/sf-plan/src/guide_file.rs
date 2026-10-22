//! Shell: a waypoint guide file parsed (DESIGN §2.11). One `col,row` or `col,row,coins_left` per
//! line, `#` comments, blank lines ignored; a point without coins matches any band.

use crate::core::GuidePoint;

/// The file's points in order; a bad line is an error naming its number (1-based).
pub fn parse_waypoints(text: &str) -> Result<Vec<GuidePoint>, String> {
    let mut points = Vec::new();
    for (k, line) in text.lines().enumerate() {
        let body = line.split('#').next().unwrap_or("").trim();
        if body.is_empty() {
            continue;
        }
        let bad = |why: &str| format!("line {}: {why}: {line:?}", k + 1);
        let fields: Vec<&str> = body.split(',').map(str::trim).collect();
        if !(2..=3).contains(&fields.len()) {
            return Err(bad("want col,row or col,row,coins_left"));
        }
        let num = |s: &str| s.parse::<u16>().map_err(|_| bad("not a number"));
        points.push(GuidePoint {
            cell: (num(fields[0])?, num(fields[1])?),
            coins_left: fields.get(2).map(|s| num(s)).transpose()?,
        });
    }
    Ok(points)
}
