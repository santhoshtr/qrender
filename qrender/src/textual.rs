//! Text-oriented backends over the card IR: plain text and markdown for
//! LLM/chat consumption, wikitext for wikis, and a plain semantic HTML
//! fragment. One IR, four serializations - no per-format templates.

use std::fmt::Write;

use crate::cards::{CardKind, FactoidPage};

const WIKIDATA_URL: &str = "https://www.wikidata.org/wiki";

pub fn render_text(page: &FactoidPage) -> String {
    let mut out = String::new();
    if let Some(label) = &page.label {
        let _ = writeln!(out, "# {label}");
    }
    if let Some(description) = &page.description {
        let _ = writeln!(out, "{description}");
    }
    for card in page.all_cards() {
        let _ = writeln!(out, "\n## {}", card.title);
        match &card.kind {
            CardKind::Image { image } => {
                let _ = writeln!(out, "{}: {}", image.caption, image.file_url);
            }
            CardKind::Gallery { images } => {
                for image in images {
                    let _ = writeln!(out, "{}: {}", image.caption, image.file_url);
                }
            }
            CardKind::Stat { value, note } => match note {
                Some(note) => {
                    let _ = writeln!(out, "{value} ({note})");
                }
                None => {
                    let _ = writeln!(out, "{value}");
                }
            },
            CardKind::Meter { display, note, .. } => match note {
                Some(note) => {
                    let _ = writeln!(out, "{display} ({note})");
                }
                None => {
                    let _ = writeln!(out, "{display}");
                }
            },
            CardKind::StatSeries {
                current,
                note,
                series,
            } => {
                let _ = writeln!(out, "{current} ({})", note.as_deref().unwrap_or("latest"));
                for point in series {
                    let _ = writeln!(out, "  {}: {}", point.label, point.display);
                }
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, "{lat}, {lon}");
            }
            CardKind::KeyValues { entries } => {
                for entry in entries {
                    let _ = writeln!(out, "{}: {}", entry.key, entry.values.join("; "));
                }
            }
            CardKind::Links { entries } => {
                for entry in entries {
                    let _ = writeln!(out, "{}: {}", entry.label, entry.url);
                }
            }
            CardKind::ItemChips { items } => {
                let joined = items
                    .iter()
                    .map(|item| match &item.note {
                        Some(note) => format!("{} ({note})", item.label),
                        None => item.label.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let _ = writeln!(out, "{joined}");
            }
        }
    }
    out
}

pub fn render_markdown(page: &FactoidPage) -> String {
    let mut out = String::new();
    if let Some(label) = &page.label {
        let _ = writeln!(out, "# {label}");
    }
    if let Some(description) = &page.description {
        let _ = writeln!(out, "*{description}*");
    }
    for card in page.all_cards() {
        let _ = writeln!(out, "\n## {}", card.title);
        match &card.kind {
            CardKind::Image { image } => {
                let _ = writeln!(out, "![{}]({})", image.caption, image.thumb_url);
            }
            CardKind::Gallery { images } => {
                for image in images {
                    let _ = writeln!(out, "![{}]({})", image.caption, image.thumb_url);
                }
            }
            CardKind::Stat { value, note } => match note {
                Some(note) => {
                    let _ = writeln!(out, "**{value}** ({note})");
                }
                None => {
                    let _ = writeln!(out, "**{value}**");
                }
            },
            CardKind::Meter { display, note, .. } => match note {
                Some(note) => {
                    let _ = writeln!(out, "**{display}** ({note})");
                }
                None => {
                    let _ = writeln!(out, "**{display}**");
                }
            },
            CardKind::StatSeries {
                current,
                note,
                series,
            } => {
                let _ = writeln!(
                    out,
                    "**{current}** ({})",
                    note.as_deref().unwrap_or("latest")
                );
                for point in series {
                    let _ = writeln!(out, "- {}: {}", point.label, point.display);
                }
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, "{lat}, {lon}");
            }
            CardKind::KeyValues { entries } => {
                for entry in entries {
                    let _ = writeln!(out, "- **{}**: {}", entry.key, entry.values.join("; "));
                }
            }
            CardKind::Links { entries } => {
                for entry in entries {
                    let _ = writeln!(out, "- [{}]({})", entry.label, entry.url);
                }
            }
            CardKind::ItemChips { items } => {
                for item in items {
                    let link = format!("[{}]({WIKIDATA_URL}/{})", item.label, item.qid);
                    match &item.note {
                        Some(note) => {
                            let _ = writeln!(out, "- {link} ({note})");
                        }
                        None => {
                            let _ = writeln!(out, "- {link}");
                        }
                    }
                }
            }
        }
    }
    out
}

pub fn render_wikitext(page: &FactoidPage) -> String {
    let mut out = String::new();
    if let Some(label) = &page.label {
        let _ = writeln!(out, "'''{label}'''");
    }
    if let Some(description) = &page.description {
        let _ = writeln!(out, "{description}");
    }
    for card in page.all_cards() {
        let _ = writeln!(out, "\n== {} ==", card.title);
        match &card.kind {
            CardKind::Image { image } => {
                let _ = writeln!(out, "[[File:{}|thumb|{}]]", image.file_name, image.caption);
            }
            CardKind::Gallery { images } => {
                for image in images {
                    let _ = writeln!(out, "[[File:{}|thumb|{}]]", image.file_name, image.caption);
                }
            }
            CardKind::Stat { value, note } => match note {
                Some(note) => {
                    let _ = writeln!(out, ":* {value} ({note})");
                }
                None => {
                    let _ = writeln!(out, ":* {value}");
                }
            },
            CardKind::Meter { display, note, .. } => match note {
                Some(note) => {
                    let _ = writeln!(out, ":* {display} ({note})");
                }
                None => {
                    let _ = writeln!(out, ":* {display}");
                }
            },
            CardKind::StatSeries {
                current,
                note,
                series,
            } => {
                let _ = writeln!(
                    out,
                    ":* {current} ({})",
                    note.as_deref().unwrap_or("latest")
                );
                for point in series {
                    let _ = writeln!(out, ":* {}: {}", point.label, point.display);
                }
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, ":* {lat}, {lon}");
            }
            CardKind::KeyValues { entries } => {
                for entry in entries {
                    let _ = writeln!(out, ";{}", entry.key);
                    for value in &entry.values {
                        let _ = writeln!(out, ":* {value}");
                    }
                }
            }
            CardKind::Links { entries } => {
                for entry in entries {
                    let _ = writeln!(out, ":* [{} {}]", entry.url, entry.label);
                }
            }
            CardKind::ItemChips { items } => {
                for item in items {
                    match &item.note {
                        Some(note) => {
                            let _ = writeln!(
                                out,
                                ":* [[wikidata:{}|{}]] ({note})",
                                item.qid, item.label
                            );
                        }
                        None => {
                            let _ = writeln!(out, ":* [[wikidata:{}|{}]]", item.qid, item.label);
                        }
                    }
                }
            }
        }
    }
    out
}

/// Plain semantic HTML fragment (no styling), for embedding.
pub fn render_html(page: &FactoidPage) -> String {
    let mut out = String::from("<article>\n");
    if let Some(label) = &page.label {
        let _ = writeln!(out, "<h1>{}</h1>", escape(label));
    }
    if let Some(description) = &page.description {
        let _ = writeln!(out, "<p>{}</p>", escape(description));
    }
    for card in page.all_cards() {
        let _ = writeln!(out, "<section>\n<h2>{}</h2>", escape(&card.title));
        match &card.kind {
            CardKind::Image { image } => {
                let _ = writeln!(
                    out,
                    "<figure><img src=\"{}\" alt=\"{}\"></figure>",
                    escape(&image.thumb_url),
                    escape(&image.caption)
                );
            }
            CardKind::Gallery { images } => {
                for image in images {
                    let _ = writeln!(
                        out,
                        "<figure><img src=\"{}\" alt=\"{}\"></figure>",
                        escape(&image.thumb_url),
                        escape(&image.caption)
                    );
                }
            }
            CardKind::Stat { value, note } => {
                let note = note
                    .as_ref()
                    .map(|n| format!(" ({})", escape(n)))
                    .unwrap_or_default();
                let _ = writeln!(out, "<p><strong>{}</strong>{note}</p>", escape(value));
            }
            CardKind::Meter {
                value,
                display,
                note,
                min,
                max,
                ..
            } => {
                let note = note
                    .as_ref()
                    .map(|n| format!(" ({})", escape(n)))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "<p><strong>{}</strong>{note}</p>\n<meter min=\"{min}\" max=\"{max}\" value=\"{value}\"></meter>",
                    escape(display)
                );
            }
            CardKind::StatSeries {
                current,
                note,
                series,
            } => {
                let _ = writeln!(
                    out,
                    "<p><strong>{}</strong> ({})</p>\n<ul>",
                    escape(current),
                    escape(note.as_deref().unwrap_or("latest"))
                );
                for point in series {
                    let _ = writeln!(
                        out,
                        "<li>{}: {}</li>",
                        escape(&point.label),
                        escape(&point.display)
                    );
                }
                let _ = writeln!(out, "</ul>");
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, "<p>{lat}, {lon}</p>");
            }
            CardKind::KeyValues { entries } => {
                let _ = writeln!(out, "<dl>");
                for entry in entries {
                    let _ = writeln!(out, "<dt>{}</dt>", escape(&entry.key));
                    for value in &entry.values {
                        let _ = writeln!(out, "<dd>{}</dd>", escape(value));
                    }
                }
                let _ = writeln!(out, "</dl>");
            }
            CardKind::Links { entries } => {
                let _ = writeln!(out, "<ul>");
                for entry in entries {
                    let _ = writeln!(
                        out,
                        "<li><a href=\"{}\">{}</a></li>",
                        escape(&entry.url),
                        escape(&entry.label)
                    );
                }
                let _ = writeln!(out, "</ul>");
            }
            CardKind::ItemChips { items } => {
                let _ = writeln!(out, "<ul>");
                for item in items {
                    let note = item
                        .note
                        .as_ref()
                        .map(|n| format!(" ({})", escape(n)))
                        .unwrap_or_default();
                    let _ = writeln!(
                        out,
                        "<li><a href=\"{WIKIDATA_URL}/{}\">{}</a>{note}</li>",
                        item.qid,
                        escape(&item.label)
                    );
                }
                let _ = writeln!(out, "</ul>");
            }
        }
        let _ = writeln!(out, "</section>");
    }
    out.push_str("</article>\n");
    out
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
