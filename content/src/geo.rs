//! The outline of a geoshape: the lines of a GeoJSON document, simplified
//! for a map on a page. Plane geometry on longitude and latitude: good
//! enough for an outline at the scale of a page.

use serde_json::Value as Json;

/// A point: longitude, latitude.
pub type Point = [f64; 2];

/// The lines of a GeoJSON document: the rings of its polygons and its
/// lines, of every feature and geometry.
pub fn lines(geojson: &Json) -> Vec<Vec<Point>> {
    let mut out = Vec::new();
    collect(geojson, &mut out);
    out
}

fn collect(g: &Json, out: &mut Vec<Vec<Point>>) {
    let line = |c: &Json| -> Vec<Point> {
        c.as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| Some([p[0].as_f64()?, p[1].as_f64()?]))
            .collect()
    };
    let coordinates = &g["coordinates"];
    match g["type"].as_str() {
        Some("FeatureCollection") => g["features"]
            .as_array()
            .into_iter()
            .flatten()
            .for_each(|f| collect(f, out)),
        Some("Feature") => collect(&g["geometry"], out),
        Some("GeometryCollection") => g["geometries"]
            .as_array()
            .into_iter()
            .flatten()
            .for_each(|f| collect(f, out)),
        Some("LineString") => out.push(line(coordinates)),
        Some("Polygon" | "MultiLineString") => {
            out.extend(coordinates.as_array().into_iter().flatten().map(line));
        }
        Some("MultiPolygon") => out.extend(
            coordinates
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|p| p.as_array().into_iter().flatten().map(line)),
        ),
        _ => {}
    }
}

/// The lines, simplified to `max_points` points at most, and their box:
/// west, south, east, north. When the shape crosses the antimeridian (the
/// narrower span of longitudes is across it), the longitudes west of 0 are
/// moved by 360: east is then above 180, and the lines are continuous.
/// None for a shape without points.
pub fn outline(
    mut lines: Vec<Vec<Point>>,
    max_points: usize,
) -> Option<(Vec<Vec<Point>>, [f64; 4])> {
    lines.retain(|l| l.len() >= 2);
    let shifted = |x: f64| if x < 0.0 { x + 360.0 } else { x };
    let (west, east) = span(lines.iter().flatten().map(|p| p[0]));
    if west > east {
        return None;
    }
    let (west2, east2) = span(lines.iter().flatten().map(|p| shifted(p[0])));
    if east2 - west2 < east - west {
        for p in lines.iter_mut().flatten() {
            p[0] = shifted(p[0]);
        }
    }
    let (west, east) = span(lines.iter().flatten().map(|p| p[0]));
    let (south, north) = span(lines.iter().flatten().map(|p| p[1]));
    let diagonal = ((east - west).powi(2) + (north - south).powi(2)).sqrt();
    let mut epsilon = diagonal * 0.0015;
    loop {
        let simplified: Vec<Vec<Point>> = lines
            .iter()
            .filter(|l| extent(l) >= 2.0 * epsilon)
            .map(|l| simplify(l, epsilon))
            .collect();
        let points: usize = simplified.iter().map(Vec::len).sum();
        if points <= max_points || epsilon > diagonal {
            return Some((simplified, [west, south, east, north]));
        }
        epsilon *= 1.5;
    }
}

/// The lowest and the highest of the numbers.
fn span(numbers: impl Iterator<Item = f64>) -> (f64, f64) {
    numbers.fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)))
}

/// The diagonal of the box of a line.
fn extent(line: &[Point]) -> f64 {
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in line {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2)).sqrt()
}

/// Douglas–Peucker: the points of the line that are further than
/// `epsilon` from the line through their neighbours that are kept.
fn simplify(line: &[Point], epsilon: f64) -> Vec<Point> {
    let mut keep = vec![false; line.len()];
    keep[0] = true;
    keep[line.len() - 1] = true;
    let mut stack = vec![(0, line.len() - 1)];
    while let Some((first, last)) = stack.pop() {
        let (mut index, mut far) = (0, 0.0);
        for (i, p) in line.iter().enumerate().take(last).skip(first + 1) {
            let d = distance(*p, line[first], line[last]);
            if d > far {
                (index, far) = (i, d);
            }
        }
        if far > epsilon {
            keep[index] = true;
            stack.push((first, index));
            stack.push((index, last));
        }
    }
    line.iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

/// The distance of `p` from the line through `a` and `b`.
fn distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = (dx * dx + dy * dy).sqrt();
    if length == 0.0 {
        return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
    }
    ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / length
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn lines_of_every_geometry() {
        let doc = json!({"type": "FeatureCollection", "features": [
            {"type": "Feature", "geometry": {"type": "Polygon",
                "coordinates": [[[0, 0], [1, 0], [1, 1], [0, 0]]]}},
            {"type": "Feature", "geometry": {"type": "GeometryCollection", "geometries": [
                {"type": "MultiPolygon", "coordinates": [[[[2, 2], [3, 2], [3, 3], [2, 2]]]]},
                {"type": "LineString", "coordinates": [[5, 5], [6, 6]]}
            ]}}
        ]});
        assert_eq!(lines(&doc).len(), 3);
    }

    #[test]
    fn a_straight_line_keeps_its_ends() {
        let line: Vec<Point> = (0..=10).map(|i| [f64::from(i), 0.0]).collect();
        let (out, bbox) = outline(vec![line], 100).unwrap();
        assert_eq!(out, [vec![[0.0, 0.0], [10.0, 0.0]]]);
        assert_eq!(bbox, [0.0, 0.0, 10.0, 0.0]);
    }

    #[test]
    fn many_points_are_simplified_to_the_limit() {
        let circle: Vec<Point> = (0..=1000)
            .map(|i| {
                let a = f64::from(i) / 1000.0 * std::f64::consts::TAU;
                [a.cos() * 10.0, a.sin() * 10.0]
            })
            .collect();
        let (out, _) = outline(vec![circle], 100).unwrap();
        let points: usize = out.iter().map(Vec::len).sum();
        assert!((4..=100).contains(&points), "{points}");
    }

    #[test]
    fn across_the_antimeridian() {
        // Alaska's Aleutians: west of 180 and east of -180.
        let line = vec![[172.0, 52.0], [179.0, 51.0], [-179.0, 51.0], [-170.0, 52.0]];
        let (out, bbox) = outline(vec![line], 100).unwrap();
        assert_eq!(bbox, [172.0, 51.0, 190.0, 52.0]);
        assert!(out[0].iter().all(|p| p[0] >= 172.0));
    }
}
