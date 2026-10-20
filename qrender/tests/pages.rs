//! The renderers, for every fixture page: snapshots of the HTML and the
//! Markdown, and the rules every HTML page keeps.

use std::fs;
use std::path::Path;

use content::{Content, Text};
use qrender::config::Config;
use qrender::localize::View;
use qrender::render::html::{self, Styles};
use qrender::render::markdown;
use qrender::render::ui::FAVICONS;
use scraper::{CaseSensitivity, Html, Selector};

const PAGES: [&str; 15] = [
    "q3870.en",
    "q3870.gu",
    "q90.en",
    "q90.ml",
    "q8023.tcy",
    "q117085614.en",
    "q405.en",
    "q405.ml",
    "q139890532.en",
    "q139890532.ml",
    "q668.en",
    "q668.ml",
    "q2.en",
    "q2.ml",
    "q30.en",
];

/// Hosts a page may load from: Commons (its pages and its media server)
/// and Wikimedia maps only.
const HOSTS: [&str; 3] = [
    "https://commons.wikimedia.org/",
    "https://upload.wikimedia.org/",
    "https://maps.wikimedia.org/",
];

fn view(page: &str) -> View {
    let read = |path: String| {
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path)).unwrap()
    };
    let qid = page.split('.').next().unwrap();
    let content: Content =
        serde_json::from_str(&read(format!("fixtures/content/{qid}.json"))).unwrap();
    let text: Text = serde_json::from_str(&read(format!("fixtures/text/{page}.json"))).unwrap();
    qrender::localize(
        &qrender::compose(&content, Config::embedded()),
        &text,
        Config::embedded(),
    )
}

fn select(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}

#[test]
fn html_pages() {
    for page in PAGES {
        // Linked styles: the snapshot holds the markup, not the stylesheet.
        let html = html::page(&view(page), Styles::Linked("design/0.2"), Some(""));
        let doc = Html::parse_document(&html);

        // No link in a link: the parser would close the outer link early.
        assert_eq!(
            doc.select(&select("a a")).count(),
            0,
            "{page}: a link in a link"
        );
        // Every invoker and popover button has its target.
        for button in doc.select(&select("[commandfor], [popovertarget]")) {
            let id = button
                .attr("commandfor")
                .or(button.attr("popovertarget"))
                .unwrap();
            let target = select(&format!("#{id}"));
            assert_eq!(
                doc.select(&target).count(),
                1,
                "{page}: no single target #{id}"
            );
        }
        // A picture of the item opens its own dialog.
        for button in doc.select(&select("button.zoom")) {
            let id = button.attr("commandfor").unwrap_or_default();
            let dialog = select(&format!("dialog.lightbox#{id}"));
            assert_eq!(
                doc.select(&dialog).count(),
                1,
                "{page}: zoom #{id} has no lightbox"
            );
        }
        // The data of each map of a dialog: a pin, and a box with its
        // south-west below and left of its north-east.
        for el in doc.select(&select("qr-map")) {
            let data: serde_json::Value =
                serde_json::from_str(el.attr("data-map").unwrap_or_default())
                    .unwrap_or_else(|e| panic!("{page}: qr-map data: {e}"));
            let [[south, west], [north, east]]: [[f64; 2]; 2] =
                serde_json::from_value(data["bounds"].clone()).unwrap();
            assert!(south < north && west < east, "{page}: qr-map bounds");
            assert!(data["pin"].is_array(), "{page}: qr-map pin");
        }
        // Pictures, sounds, videos and 3D models only from Wikimedia hosts,
        // and the icons of the sites in the web spread from their Toolforge
        // service.
        for el in doc.select(&select(
            "img[src], video[src], video[poster], audio[src], qr-model[src]",
        )) {
            let favicon = el
                .value()
                .has_class("favicon", CaseSensitivity::CaseSensitive);
            for attr in ["src", "poster"] {
                if let Some(url) = el.attr(attr) {
                    assert!(
                        HOSTS.iter().any(|h| url.starts_with(h))
                            || (favicon && url.starts_with(FAVICONS)),
                        "{page}: {attr} {url}"
                    );
                }
            }
        }
        // Ids are unique.
        let mut ids: Vec<&str> = doc
            .select(&select("[id]"))
            .filter_map(|e| e.attr("id"))
            .collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "{page}: duplicate ids");

        // A primitive's script is on the page when its element is.
        for name in ["qr-sounds", "qr-chart"] {
            let element = doc.select(&select(name)).count() > 0;
            let script = html.contains(&format!("customElements.define( '{name}'"));
            assert_eq!(
                element, script,
                "{page}: {name} element {element}, script {script}"
            );
        }

        insta::assert_snapshot!(format!("{page}.html"), html);
    }
}

#[test]
fn markdown_pages() {
    for page in PAGES {
        insta::assert_snapshot!(format!("{page}.md"), markdown::page(&view(page)));
    }
}
