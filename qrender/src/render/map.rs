//! The one piece of arithmetic in a renderer: the zoom of a map and the
//! place of its markers depend on the frame this renderer draws, which the
//! composer cannot know. Other renderers draw other frames.

use content::Pid;
use serde::Serialize;

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
    /// The box of the frame: south-west and north-east, latitude and
    /// longitude. The longitude is not wrapped (east of 180 across the
    /// antimeridian). The map of a dialog shows this box first.
    pub bounds: [[f64; 2]; 2],
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
        bounds: bounds(cx, cy, zoom),
    })
}

/// The zoom of the map of a coordinate pin: the point with its region (the
/// roads and the places around it), to say where it is.
const POINT_ZOOM: i32 = 9;

/// The map of one point, centred on it: the map of a coordinate pin.
pub fn point_frame(lat: f64, lon: f64) -> Frame<'static> {
    let (x, y) = mercator(lat, lon, POINT_ZOOM);
    Frame {
        tile: format!(
            "https://maps.wikimedia.org/img/osm-intl,{POINT_ZOOM},{lat},{lon},{WIDTH}x{HEIGHT}@2x.png"
        ),
        lat,
        lon,
        pin: Some((50.0, 50.0)),
        markers: Vec::new(),
        outlines: Vec::new(),
        bounds: bounds(x, y, POINT_ZOOM),
    }
}

/// The map of a dialog, for map.js and Leaflet (which takes latitude,
/// longitude): the pin, the box to show first, the markers with their
/// labels, and the outlines of the shapes, each as its closed rings and
/// its open lines. Places are rounded to 4 decimals (about 11 m): the
/// outlines are simplified more than that.
#[derive(Serialize)]
struct Live {
    pin: [f64; 2],
    bounds: [[f64; 2]; 2],
    markers: Vec<LiveMarker>,
    outlines: Vec<LiveOutline>,
}

#[derive(Serialize)]
struct LiveMarker {
    at: [f64; 2],
    label: String,
}

#[derive(Serialize)]
struct LiveOutline {
    rings: Vec<Vec<[f64; 2]>>,
    lines: Vec<Vec<[f64; 2]>>,
    /// The first (the preferred) shape.
    main: bool,
}

/// The JSON of the map of a dialog: of a frame, and of the markers and the
/// shapes of its map (none for the map of a coordinate pin).
pub fn live(frame: &Frame, map: Option<&Map>, label: impl Fn(Pid) -> String) -> String {
    let at = |lat: f64, lon: f64| [round4(lat), round4(lon)];
    let markers = map
        .into_iter()
        .flat_map(|m| &m.markers)
        .filter_map(|m| match m.value {
            ValueShown::Coordinate { lat, lon, .. } => Some(LiveMarker {
                at: at(lat, lon),
                label: label(m.pid),
            }),
            _ => None,
        })
        .collect();
    let outlines = map
        .into_iter()
        .flat_map(|m| &m.shapes)
        .enumerate()
        .map(|(i, s)| {
            let (rings, lines) = s
                .lines
                .iter()
                .partition(|l| l.len() > 2 && l.first() == l.last());
            let latlon = |ls: Vec<&Vec<[f64; 2]>>| {
                ls.into_iter()
                    .map(|l| l.iter().map(|p| at(p[1], p[0])).collect())
                    .collect()
            };
            LiveOutline {
                rings: latlon(rings),
                lines: latlon(lines),
                main: i == 0,
            }
        })
        .collect();
    let bounds = frame.bounds.map(|[lat, lon]| at(lat, lon));
    serde_json::to_string(&Live {
        pin: at(frame.lat, frame.lon),
        bounds,
        markers,
        outlines,
    })
    .expect("numbers and strings serialize")
}

fn round4(x: f64) -> f64 {
    (x * 1e4).round() / 1e4
}

/// The box of a frame centred on Web Mercator pixel coordinates at a zoom:
/// south-west and north-east.
fn bounds(cx: f64, cy: f64, zoom: i32) -> [[f64; 2]; 2] {
    let (south, west) = unmercator_continuous(cx - WIDTH / 2.0, cy + HEIGHT / 2.0, zoom);
    let (north, east) = unmercator_continuous(cx + WIDTH / 2.0, cy - HEIGHT / 2.0, zoom);
    [[south, west], [north, east]]
}

/// The latitude and the longitude (from -180 to 180) of Web Mercator pixel
/// coordinates at a zoom.
fn unmercator(x: f64, y: f64, zoom: i32) -> (f64, f64) {
    let (lat, lon) = unmercator_continuous(x, y, zoom);
    (lat, (lon + 180.0).rem_euclid(360.0) - 180.0)
}

/// The latitude and the longitude of Web Mercator pixel coordinates at a
/// zoom; a longitude east of 180 stays east of 180.
fn unmercator_continuous(x: f64, y: f64, zoom: i32) -> (f64, f64) {
    let n = 256.0 * 2f64.powi(zoom);
    let lon = x / n * 360.0 - 180.0;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * y / n))
        .sinh()
        .atan()
        .to_degrees();
    (lat, lon)
}

/// The latitude where Web Mercator ends (its map is a square); a pole is
/// beyond it.
const MAX_LAT: f64 = 85.051_128_78;

/// Web Mercator pixel coordinates at a zoom. A latitude beyond the end of
/// the map is at its edge.
fn mercator(lat: f64, lon: f64, zoom: i32) -> (f64, f64) {
    let n = 256.0 * 2f64.powi(zoom);
    let s = lat.clamp(-MAX_LAT, MAX_LAT).to_radians().sin();
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

    #[test]
    fn the_box_of_a_frame_is_around_its_centre() {
        let (x, y) = mercator(22.8, 83.0, 5);
        let [[south, west], [north, east]] = bounds(x, y, 5);
        assert!(south < 22.8 && 22.8 < north && west < 83.0 && 83.0 < east);
        // 800 pixels at zoom 5 are 800 / (256 × 32) of 360 degrees.
        assert!((east - west - 800.0 / 8192.0 * 360.0).abs() < 1e-9);
        // Across the antimeridian the east stays east of 180.
        let (x, y) = mercator(-17.0, 179.0, 5);
        assert!(bounds(x, y, 5)[1][1] > 180.0);
    }

    #[test]
    fn a_point_frame_has_its_point_in_the_middle() {
        let f = point_frame(48.85, 2.35);
        assert_eq!(f.pin, Some((50.0, 50.0)));
        let [[south, west], [north, east]] = f.bounds;
        assert!(((west + east) / 2.0 - 2.35).abs() < 1e-9);
        assert!(south < 48.85 && 48.85 < north);
    }

    #[test]
    fn a_pole_is_at_the_edge_of_the_map() {
        let [[south, _], [north, _]] = point_frame(90.0, 0.0).bounds;
        assert!(south < north && north.is_finite());
    }
}
