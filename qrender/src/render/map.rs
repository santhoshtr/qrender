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

pub struct Frame<'b> {
    pub tile: String,
    pub lat: f64,
    pub lon: f64,
    /// Markers inside the frame, with their place in percent.
    pub markers: Vec<(f64, f64, &'b Marker)>,
}

pub fn frame(map: &Map) -> Option<Frame<'_>> {
    let ValueShown::Coordinate { lat, lon, .. } = map.statement.value else {
        return None;
    };
    let zoom = map.extent_m.map_or(DEFAULT_ZOOM, |extent| {
        // The item's extent takes a third of the frame width.
        let z = (WIDTH * METRES_PER_PIXEL * lat.to_radians().cos() / (3.0 * extent as f64)).log2();
        (z.round_ties_even() as i32).clamp(3, 14)
    });
    let (cx, cy) = mercator(lat, lon, zoom);
    let markers = map
        .markers
        .iter()
        .filter_map(|m| {
            let ValueShown::Coordinate { lat, lon, .. } = m.value else {
                return None;
            };
            let (x, y) = mercator(lat, lon, zoom);
            let (left, top) = (
                50.0 + (x - cx) / WIDTH * 100.0,
                50.0 + (y - cy) / HEIGHT * 100.0,
            );
            ((0.0..=100.0).contains(&left) && (0.0..=100.0).contains(&top))
                .then_some((left, top, m))
        })
        .collect();
    Some(Frame {
        tile: format!(
            "https://maps.wikimedia.org/img/osm-intl,{zoom},{lat},{lon},{WIDTH}x{HEIGHT}@2x.png"
        ),
        lat,
        lon,
        markers,
    })
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
}
