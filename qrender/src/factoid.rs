//! Factoid HTML renderer: an Askama page over the card IR. Output is a
//! self-contained document - Codex design tokens and the stylesheet are
//! embedded, images/tiles come from Commons and Wikimedia Maps only.

use askama::Template;

use crate::cards::{CardKind, FactoidPage, MediaKind, SeriesPoint};
use crate::error::QRenderError;

#[derive(Template)]
#[template(path = "factoid/page.html")]
struct PageTemplate<'a> {
    page: &'a FactoidPage,
    tokens_light: &'static str,
    tokens_dark: &'static str,
    stylesheet: &'static str,
}

impl PageTemplate<'_> {
    /// Bar length for a series point, relative to the series maximum.
    fn bar_percent(&self, value: &f64, series: &[SeriesPoint]) -> u32 {
        let max = series.iter().map(|p| p.value).fold(f64::MIN, f64::max);
        if max <= 0.0 {
            return 0;
        }
        ((value / max) * 100.0).round() as u32
    }
}

pub fn render_page(page: &FactoidPage) -> Result<String, QRenderError> {
    let template = PageTemplate {
        page,
        tokens_light: include_str!("../assets/codex-tokens-light.css"),
        tokens_dark: include_str!("../assets/codex-tokens-dark.css"),
        stylesheet: include_str!("../assets/factoid.css"),
    };
    Ok(template.render()?)
}
