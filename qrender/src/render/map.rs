//! The one piece of arithmetic in a renderer: the zoom of a map and the
//! place of its markers depend on the frame this renderer draws, which the
//! composer cannot know. Other renderers draw other frames.

use crate::primitives::map::{Map, Marker};
use crate::schema::ValueShown;

/// The frame of the static map image, in pixels.
const WIDTH: f64 = 800.0;
const HEIGHT: f64 = 500.0;
/// The zoom when the item has no area.
const DEFAULT_ZOOM: i32 = 11;
/// Metres per pixel at zoom 0 on the equator.
const METRES_PER_PIXEL: f64 = 156_543.0;

/// The share of the frame that the box of a shape takes at most.
const FILL: f64 = 0.9;

pub struct Frame<'b> {
    pub tile: String,
    /// The coordinate of the item (its caption).
    pub lat: f64,
    pub lon: f64,
    /// The place of the coordinate in the frame, in percent; None outside.
    pub pin: Option<(f64, f64)>,
    /// Markers inside the frame, with their place in percent.
    pub markers: Vec<(f64, f64, &'b Marker)>,
    /// The outlines of the shapes, as SVG paths in percent of the frame;
    /// true for the first (the preferred) shape.
    pub outlines: Vec<(String, bool)>,
}

pub fn frame(map: &Map) -> Option<Frame<'_>> {
    let ValueShown::Coordinate { lat, lon, .. } = map.statement.value else {
        return None;
    };
    // With a shape: the zoom at which its box fits, centred on the box.
    // Without: the item's extent takes a third of the frame width.
    let shape = map.shapes.first();
    let unwrap = |x: f64| match shape {
        Some(s) if s.bbox[2] > 180.0 && x < s.bbox[0] => x + 360.0,
        _ => x,
    };
    let (zoom, cx, cy) = match shape {
        Some(s) => {
            let [west, south, east, north] = s.bbox;
            let fits = |z: i32| {
                let (x0, y0) = mercator(north, west, z);
                let (x1, y1) = mercator(south, east, z);
                x1 - x0 <= WIDTH * FILL && y1 - y0 <= HEIGHT * FILL
            };
            let zoom = (1..=14).rev().find(|z| fits(*z)).unwrap_or(1);
            let (x0, y0) = mercator(north, west, zoom);
            let (x1, y1) = mercator(south, east, zoom);
            (zoom, (x0 + x1) / 2.0, (y0 + y1) / 2.0)
        }
        None => {
            let zoom = map.extent_m.map_or(DEFAULT_ZOOM, |extent| {
                let z = (WIDTH * METRES_PER_PIXEL * lat.to_radians().cos() / (3.0 * extent as f64))
                    .log2();
                (z.round_ties_even() as i32).clamp(3, 14)
            });
            let (x, y) = mercator(lat, lon, zoom);
            (zoom, x, y)
        }
    };
    let place = |lat: f64, lon: f64| {
        let (x, y) = mercator(lat, unwrap(lon), zoom);
        (
            50.0 + (x - cx) / WIDTH * 100.0,
            50.0 + (y - cy) / HEIGHT * 100.0,
        )
    };
    let inside = |(left, top): (f64, f64)| {
        ((0.0..=100.0).contains(&left) && (0.0..=100.0).contains(&top)).then_some((left, top))
    };
    let markers = map
        .markers
        .iter()
        .filter_map(|m| {
            let ValueShown::Coordinate { lat, lon, .. } = m.value else {
                return None;
            };
            let (left, top) = inside(place(lat, lon))?;
            Some((left, top, m))
        })
        .collect();
    let outlines = map
        .shapes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let path: String = s
                .lines
                .iter()
                .map(|line| {
                    let points: Vec<String> = line
                        .iter()
                        .map(|p| {
                            let (x, y) = place(p[1], p[0]);
                            format!("{x:.2},{y:.2}")
                        })
                        .collect();
                    let closed = line.first() == line.last();
                    format!("M{}{}", points.join("L"), if closed { "Z" } else { "" })
                })
                .collect();
            (path, i == 0)
        })
        .collect();
    let (centre_lat, centre_lon) = unmercator(cx, cy, zoom);
    Some(Frame {
        tile: format!(
            "https://maps.wikimedia.org/img/osm-intl,{zoom},{centre_lat:.5},{centre_lon:.5},{WIDTH}x{HEIGHT}@2x.png"
        ),
        lat,
        lon,
        pin: inside(place(lat, lon)),
        markers,
        outlines,
    })
}

/// The zoom of the map of a coordinate pin: the point with its region (the
/// roads and the places around it), to say where it is.
const POINT_ZOOM: i32 = 9;

/// The map of one point, centred on it: the map of a coordinate pin.
pub fn point_tile(lat: f64, lon: f64) -> String {
    format!(
        "https://maps.wikimedia.org/img/osm-intl,{POINT_ZOOM},{lat},{lon},{WIDTH}x{HEIGHT}@2x.png"
    )
}

/// The latitude and the longitude (from -180 to 180) of Web Mercator pixel
/// coordinates at a zoom.
fn unmercator(x: f64, y: f64, zoom: i32) -> (f64, f64) {
    let n = 256.0 * 2f64.powi(zoom);
    let lon = x / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * y / n))
        .sinh()
        .atan()
        .to_degrees();
    (lat, (lon + 180.0).rem_euclid(360.0) - 180.0)
}

/// Web Mercator pixel coordinates at a zoom.
fn mercator(lat: f64, lon: f64, zoom: i32) -> (f64, f64) {
    let n = 256.0 * 2f64.powi(zoom);
    let s = lat.to_radians().sin();
    (
        (lon + 180.0) / 360.0 * n,
        (0.5 - ((1.0 + s) / (1.0 - s)).ln() / (4.0 * std::f64::consts::PI)) * n,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercator_centre() {
        let (x, y) = mercator(0.0, 0.0, 0);
        assert_eq!((x, y), (128.0, 128.0));
    }

    #[test]
    fn unmercator_inverts_mercator() {
        let (x, y) = mercator(22.8, 83.0, 5);
        let (lat, lon) = unmercator(x, y, 5);
        assert!((lat - 22.8).abs() < 1e-9 && (lon - 83.0).abs() < 1e-9);
        // A longitude east of 180 (across the antimeridian) comes back west.
        let (x, y) = mercator(52.0, 190.0, 3);
        assert!((unmercator(x, y, 3).1 + 170.0).abs() < 1e-9);
    }
}
