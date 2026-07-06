//! Text-oriented backends over the card IR: plain text and markdown for
//! LLM/chat consumption, wikitext for wikis, and a plain semantic HTML
//! fragment. One IR, four serializations - no per-format templates.

use std::fmt::Write;

use crate::cards::{CardKind, FactValue, FactoidPage};

const WIKIDATA_URL: &str = "https://www.wikidata.org/wiki";

/// "(2018 – 2021; role: guest)" - span first, remaining qualifiers after.
fn context_suffix(span: &Option<crate::cards::TemporalSpan>, note: &Option<String>) -> String {
    let parts: Vec<String> = span
        .iter()
        .map(|s| s.display())
        .chain(note.iter().cloned())
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}

/// Plain-text form of a fact value; items and links lose their URLs.
fn fact_value_text(value: &FactValue) -> String {
    match value {
        FactValue::Item(chip) => {
            format!("{}{}", chip.label, context_suffix(&chip.span, &chip.note))
        }
        FactValue::Link { url } => url.clone(),
        FactValue::Text { value, span, note } => {
            format!("{value}{}", context_suffix(span, note))
        }
    }
}

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
            CardKind::Indicators { indicators } => {
                for indicator in indicators {
                    let _ = writeln!(
                        out,
                        "{}: {} ({})",
                        indicator.label,
                        indicator.current,
                        indicator.note.as_deref().unwrap_or("latest")
                    );
                    for point in &indicator.series {
                        let _ = writeln!(out, "  {}: {}", point.label, point.display);
                    }
                }
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, "{lat}, {lon}");
            }
            CardKind::Timeline { events } => {
                for event in events {
                    match &event.detail {
                        Some(detail) => {
                            let _ = writeln!(out, "{}: {detail} ({})", event.display, event.label);
                        }
                        None => {
                            let _ = writeln!(out, "{}: {}", event.display, event.label);
                        }
                    }
                }
            }
            CardKind::Facts { rows } => {
                for row in rows {
                    let values = row
                        .values
                        .iter()
                        .map(fact_value_text)
                        .collect::<Vec<_>>()
                        .join("; ");
                    let _ = writeln!(out, "{}: {values}", row.label);
                }
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
            CardKind::Indicators { indicators } => {
                for indicator in indicators {
                    let _ = writeln!(
                        out,
                        "- **{}**: {} ({})",
                        indicator.label,
                        indicator.current,
                        indicator.note.as_deref().unwrap_or("latest")
                    );
                    for point in &indicator.series {
                        let _ = writeln!(out, "  - {}: {}", point.label, point.display);
                    }
                }
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, "{lat}, {lon}");
            }
            CardKind::Timeline { events } => {
                for event in events {
                    match &event.detail {
                        Some(detail) => {
                            let _ =
                                writeln!(out, "- **{}** {detail} ({})", event.display, event.label);
                        }
                        None => {
                            let _ = writeln!(out, "- **{}** {}", event.display, event.label);
                        }
                    }
                }
            }
            CardKind::Facts { rows } => {
                for row in rows {
                    let values = row
                        .values
                        .iter()
                        .map(|value| match value {
                            FactValue::Item(chip) => format!(
                                "[{}]({WIKIDATA_URL}/{}){}",
                                chip.label,
                                chip.qid,
                                context_suffix(&chip.span, &chip.note)
                            ),
                            FactValue::Link { url } => format!("<{url}>"),
                            text => fact_value_text(text),
                        })
                        .collect::<Vec<_>>()
                        .join("; ");
                    let _ = writeln!(out, "- **{}**: {values}", row.label);
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
            CardKind::Indicators { indicators } => {
                for indicator in indicators {
                    let _ = writeln!(
                        out,
                        ";{}: {} ({})",
                        indicator.label,
                        indicator.current,
                        indicator.note.as_deref().unwrap_or("latest")
                    );
                    for point in &indicator.series {
                        let _ = writeln!(out, ":* {}: {}", point.label, point.display);
                    }
                }
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, ":* {lat}, {lon}");
            }
            CardKind::Timeline { events } => {
                for event in events {
                    match &event.detail {
                        Some(detail) => {
                            let _ =
                                writeln!(out, ":* {}: {detail} ({})", event.display, event.label);
                        }
                        None => {
                            let _ = writeln!(out, ":* {}: {}", event.display, event.label);
                        }
                    }
                }
            }
            CardKind::Facts { rows } => {
                for row in rows {
                    let _ = writeln!(out, ";{}", row.label);
                    for value in &row.values {
                        match value {
                            FactValue::Item(chip) => {
                                let _ = writeln!(
                                    out,
                                    ":* [[wikidata:{}|{}]]{}",
                                    chip.qid,
                                    chip.label,
                                    context_suffix(&chip.span, &chip.note)
                                );
                            }
                            FactValue::Link { url } => {
                                let _ = writeln!(out, ":* {url}");
                            }
                            text => {
                                let _ = writeln!(out, ":* {}", fact_value_text(text));
                            }
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
            CardKind::Indicators { indicators } => {
                let _ = writeln!(out, "<dl>");
                for indicator in indicators {
                    let _ = writeln!(out, "<dt>{}</dt>", escape(&indicator.label));
                    let _ = writeln!(
                        out,
                        "<dd><strong>{}</strong> ({})</dd>",
                        escape(&indicator.current),
                        escape(indicator.note.as_deref().unwrap_or("latest"))
                    );
                    for point in &indicator.series {
                        let _ = writeln!(
                            out,
                            "<dd>{}: {}</dd>",
                            escape(&point.label),
                            escape(&point.display)
                        );
                    }
                }
                let _ = writeln!(out, "</dl>");
            }
            CardKind::Map { lat, lon, .. } => {
                let _ = writeln!(out, "<p>{lat}, {lon}</p>");
            }
            CardKind::Timeline { events } => {
                let _ = writeln!(out, "<ol>");
                for event in events {
                    let detail = event
                        .detail
                        .as_ref()
                        .map(|d| format!("{} ({})", escape(d), escape(&event.label)))
                        .unwrap_or_else(|| escape(&event.label));
                    let _ = writeln!(out, "<li>{}: {detail}</li>", escape(&event.display));
                }
                let _ = writeln!(out, "</ol>");
            }
            CardKind::Facts { rows } => {
                let _ = writeln!(out, "<dl>");
                for row in rows {
                    let _ = writeln!(out, "<dt>{}</dt>", escape(&row.label));
                    for value in &row.values {
                        let rendered = match value {
                            FactValue::Item(chip) => {
                                format!(
                                    "<a href=\"{WIKIDATA_URL}/{}\">{}</a>{}",
                                    chip.qid,
                                    escape(&chip.label),
                                    escape(&context_suffix(&chip.span, &chip.note))
                                )
                            }
                            FactValue::Link { url } => {
                                format!("<a href=\"{}\">{}</a>", escape(url), escape(url))
                            }
                            text => escape(&fact_value_text(text)),
                        };
                        let _ = writeln!(out, "<dd>{rendered}</dd>");
                    }
                }
                let _ = writeln!(out, "</dl>");
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
