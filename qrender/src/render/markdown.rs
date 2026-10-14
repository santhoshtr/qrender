//! The Markdown renderer: the text of a page for readers that read text,
//! LLMs among them. It reads the view, as the HTML page does, so the two
//! cannot disagree. It keeps what a page shows behind "+N" (all values),
//! the qualifier notes, provenance in words, absences, and the language of
//! a label that is not in the page language.
//!
//! Each primitive writes its own part (its module's `markdown` function);
//! this file has the page, the spreads and the small parts they share.

use content::{EntityId, Pid};

use crate::localize::format::Part;
use crate::localize::{Label, SpreadView, View};
use crate::primitives::{
    absent, credits, fact, identifiers, kind, map, media, names, network, peers, pictures, prose,
    provenance, scale, score, series, sitelinks, sounds, split, succession, timeline, title, works,
};
use crate::schema::{Block, Provenance, StatementRef};

pub fn page(view: &View) -> String {
    let mut md = Markdown {
        view,
        out: String::new(),
    };
    for spread in &view.spreads {
        md.spread(spread);
    }
    if !view.unmatched_sections.is_empty() {
        let links: Vec<_> = view
            .unmatched_sections
            .iter()
            .map(|l| format!("[{}]({})", l.text, l.href))
            .collect();
        md.paragraph(&format!("Article sections: {}", links.join(" · ")));
    }
    md.paragraph(&format!(
        "Composed by rule from Wikidata item {} ({}, {}, {}).",
        view.qid, view.composition, view.schema, view.design
    ));
    md.out
}

pub struct Markdown<'v> {
    pub view: &'v View,
    out: String,
}

impl Markdown<'_> {
    fn spread(&mut self, s: &SpreadView) {
        if s.spread.id != "identity"
            && let Some(title) = &s.title
        {
            self.heading(2, &title.text);
        }
        if let Some(p) = &s.prose_text {
            self.paragraph(&format!("{} ([{}]({}))", plain(&p.html), p.title, p.href));
        }
        for block in s.spread.slots.values().flatten() {
            self.block(block);
        }
        let topics: Vec<String> = s.spread.topics.iter().map(|t| self.value(t)).collect();
        if !topics.is_empty() {
            self.paragraph(&format!("Topics: {}", topics.join(" · ")));
        }
    }

    fn block(&mut self, b: &Block) {
        match b {
            Block::Title(b) => title::markdown(b, self),
            Block::Kind(b) => kind::markdown(b, self),
            Block::Prose(b) => prose::markdown(b, self),
            Block::Media(b) => media::markdown(b, self),
            Block::Pictures(b) => pictures::markdown(b, self),
            Block::Sounds(b) => sounds::markdown(b, self),
            Block::Names(b) => names::markdown(b, self),
            Block::Map(b) => map::markdown(b, self),
            Block::Fact(b) => fact::markdown(b, self),
            Block::Series(b) => series::markdown(b, self),
            Block::Split(b) => split::markdown(b, self),
            Block::Scale(b) => scale::markdown(b, self),
            Block::Peers(b) => peers::markdown(b, self),
            Block::Network(b) => network::markdown(b, self),
            Block::Gallery(b) => network::gallery_markdown(b, self),
            Block::Succession(b) => succession::markdown(b, self),
            Block::Credits(b) => credits::markdown(b, self),
            Block::Score(b) => score::markdown(b, self),
            Block::Works(b) => works::markdown(b, self),
            Block::Timeline(b) => timeline::markdown(b, self),
            Block::Absent(b) => absent::markdown(b, self),
            Block::Provenance(b) => provenance::markdown(b, self),
            Block::Sitelinks(b) => sitelinks::markdown(b, self),
            Block::Identifiers(b) => identifiers::markdown(b, self),
        }
    }

    // ------------------------------------------------------------ output

    pub fn heading(&mut self, level: usize, text: &str) {
        self.out
            .push_str(&format!("{} {text}\n\n", "#".repeat(level)));
    }

    pub fn paragraph(&mut self, text: &str) {
        self.out.push_str(text);
        self.out.push_str("\n\n");
    }

    /// "- **key**: value"
    pub fn item(&mut self, key: &str, value: &str) {
        self.out.push_str(&format!("- **{key}**: {value}\n"));
    }

    /// "- **key**:" and one sub-item for each value.
    pub fn items(&mut self, key: &str, values: &[String]) {
        if values.len() == 1 {
            return self.item(key, &values[0]);
        }
        self.out.push_str(&format!("- **{key}**:\n"));
        for v in values {
            self.out.push_str(&format!("  - {v}\n"));
        }
    }

    /// The end of a list of items.
    pub fn end(&mut self) {
        if !self.out.ends_with("\n\n") {
            self.out.push('\n');
        }
    }

    // ------------------------------------------------------------- parts

    /// A label, with its language when it is not in the page language.
    pub fn text(&self, l: &Label) -> String {
        match (&l.lang, l.fallback) {
            (Some(lang), true) => format!("{} [{lang}]", l.text),
            _ => l.text.clone(),
        }
    }

    pub fn label(&self, id: EntityId) -> String {
        match self.view.labels.get(&id) {
            Some(l) => self.text(l),
            None => id.to_string(),
        }
    }

    pub fn property(&self, pid: Pid) -> String {
        self.label(EntityId::Property(pid))
    }

    /// A property and its description, when the view has it (the property
    /// of measurements): "birth rate (the total number of live births per
    /// 1,000 population)".
    pub fn described_property(&self, pid: Pid) -> String {
        let label = self.property(pid);
        match self
            .view
            .labels
            .get(&EntityId::Property(pid))
            .and_then(|l| l.description.as_ref())
        {
            Some(d) => format!("{label} ({})", d.text),
            None => label,
        }
    }

    pub fn part(&self, p: &Part) -> String {
        match p {
            Part::Entity(id) => self.label(*id),
            Part::Number { text, unit, .. } => match unit {
                Some(u) => format!("{text} {}", self.label(EntityId::Item(*u))),
                None => text.clone(),
            },
            Part::Time { text, .. } => text.clone(),
            Part::Text {
                text,
                lang: Some(lang),
            } => format!("{text} ({lang})"),
            Part::Text { text, lang: None } => text.clone(),
            Part::Link { text, href } if text == href => format!("<{href}>"),
            Part::Link { text, href } => format!("[{text}]({href})"),
            Part::Qualified { pid, value } => {
                format!("{}: {}", self.property(*pid), self.part(value))
            }
            Part::File { pid, file, .. } => {
                format!("{}: <{}{file}>", self.property(*pid), super::ui::FILEPATH)
            }
        }
    }

    pub fn note(&self, parts: &[Part]) -> String {
        parts
            .iter()
            .map(|p| self.part(p))
            .collect::<Vec<_>>()
            .join(" · ")
    }

    /// A statement: its value, its qualifiers, and its provenance in words.
    pub fn value(&self, s: &StatementRef) -> String {
        let mut out = self.part(&self.view.part(&s.value));
        let note = self.view.note(&s.qualifiers);
        if !note.is_empty() {
            out.push_str(&format!(" ({})", self.note(&note)));
        }
        if s.described
            && let Some(item) = s.value.item()
            && let Some(d) = self
                .view
                .labels
                .get(&EntityId::Item(item))
                .and_then(|l| l.description.as_ref())
        {
            out.push_str(&format!(" — {}", d.text));
        }
        out.push_str(provenance_words(s.provenance));
        out
    }
}

/// Provenance in words, by exception: nothing for a sourced statement.
pub fn provenance_words(p: Provenance) -> &'static str {
    match p {
        Provenance::Sourced => "",
        Provenance::Imported => " [imported from Wikipedia]",
        Provenance::None => " [no reference]",
    }
}

/// The text of cleaned article HTML: tags removed, a ruby gloss in
/// parentheses, entities decoded.
pub fn plain(html: &str) -> String {
    let html = html.replace("<rt>", " (").replace("</rt>", ")");
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    content::html::decode_entities(&out)
}

impl crate::schema::ValueShown {
    /// The item of an item value.
    pub fn item(&self) -> Option<content::Qid> {
        match self {
            Self::Item { item } => *item,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_of_article_html() {
        assert_eq!(
            plain(
                r#"capital of <ruby><a href="x">Kenya</a><rt><span>കെനിയ</span></rt></ruby> &amp; more"#
            ),
            "capital of Kenya (കെനിയ) & more"
        );
    }
}
