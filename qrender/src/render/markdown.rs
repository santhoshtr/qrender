//! The Markdown renderer: the text of a page for readers that read text,
//! LLMs among them. It reads the view, as the HTML page does, so the two
//! cannot disagree. It keeps what a page shows behind "+N" (all values),
//! the qualifier notes, provenance in words, absences, the language of a
//! label that is not in the page language, and the sources as footnotes:
//! `[^3]` for a note of the article, `[^s1]` for a source of a statement.
//!
//! Each primitive writes its own part (its module's `markdown` function);
//! this file has the page, the spreads and the small parts they share.

use std::cell::RefCell;

use content::{EntityId, Pid};

use super::cite::{self, Card, Value as CiteValue};
use crate::localize::format::Part;
use crate::localize::{Label, ProseBlock, ProseSection, SpreadView, View};
use crate::primitives::{
    absent, credits, earlier, fact, identifiers, kind, links, map, media, names, network, peers,
    pictures, prose, provenance, scale, score, series, sitelinks, sounds, split, succession,
    timeline, title, works,
};
use crate::schema::{Block, Provenance, StatementRef};
use crate::vocab::Slot;

pub fn page(view: &View) -> String {
    let mut md = Markdown {
        view,
        out: String::new(),
        sources: RefCell::default(),
    };
    for spread in view.spreads_at(false) {
        md.spread(spread);
    }
    // The titles of the sections that no spread shows; with their content
    // (a fallback article), the titles only too.
    let links: Vec<_> = view
        .unmatched_sections
        .iter()
        .map(|l| format!("[{}]({})", l.text, l.href))
        .chain(
            view.more_sections
                .iter()
                .map(|s| format!("[{}]({})", s.title, s.href)),
        )
        .collect();
    if !links.is_empty() {
        md.paragraph(&format!("Article sections: {}", links.join(" · ")));
    }
    for spread in view.spreads_at(true) {
        md.spread(spread);
    }
    if !view.set_aside.is_empty() {
        md.heading(2, "Set aside (not of the best rank)");
        let mut pids: Vec<_> = view.set_aside.iter().map(|s| s.pid).collect();
        pids.dedup();
        for pid in pids {
            let values: Vec<String> = view
                .set_aside
                .iter()
                .filter(|s| s.pid == pid)
                .map(|s| md.value(s))
                .collect();
            md.items(&md.property(pid), &values);
        }
    }
    md.footnotes();
    md.paragraph(&format!(
        "Composed by rule from Wikidata item {} ({}, {}, {}).",
        view.qid, view.composition, view.schema, view.design
    ));
    md.out
}

/// The first text block of a section, or of its first subsection that has
/// one.
fn first_block(s: &ProseSection) -> Option<&String> {
    s.blocks
        .iter()
        .find_map(|b| match b {
            ProseBlock::Text(html) => Some(html),
            ProseBlock::Figure(_) => None,
        })
        .or_else(|| s.sections.iter().find_map(first_block))
}

pub struct Markdown<'v> {
    pub view: &'v View,
    out: String,
    /// The citations of statements, by hash, in the order of their first
    /// footnote mark: `[^s1]` is the first.
    sources: RefCell<Vec<String>>,
}

impl Markdown<'_> {
    fn spread(&mut self, s: &SpreadView) {
        if s.spread.id != "identity"
            && let Some(title) = &s.title
        {
            self.heading(2, &title.text);
        }
        // The order of the slots, but the stack right after the lead: a
        // second office beside the lead (the `beside` rule) stays with it,
        // before the blocks under the lead.
        let slots = &s.spread.slots;
        for (slot, blocks) in slots {
            match slot {
                Slot::Hero => {
                    let (lead, under) = blocks.split_at(blocks.len().min(1));
                    let stack = slots.get(&Slot::Stack).into_iter().flatten();
                    for block in lead.iter().chain(stack).chain(under) {
                        self.block(block);
                    }
                }
                Slot::Stack if slots.contains_key(&Slot::Hero) => {}
                _ => blocks.iter().for_each(|b| self.block(b)),
            }
        }
        // The facts first, then the text of the article.
        if let Some(p) = &s.prose_text {
            self.paragraph(&format!("{} ([{}]({}))", plain(&p.html), p.title, p.href));
        } else if let Some((section, block)) =
            s.sections.iter().find_map(|a| Some((a, first_block(a)?)))
        {
            // The sections with their content (a fallback article): their
            // first paragraph, as for a section without it.
            self.paragraph(&format!(
                "{} ([{}]({}))",
                plain(block),
                section.title,
                section.href
            ));
        }
        let topics: Vec<String> = s.spread.topics.iter().map(|t| self.topic(t)).collect();
        if !topics.is_empty() {
            self.paragraph(&format!("Topics: {}", topics.join(" · ")));
        }
    }

    /// A topic: a link to its Wikipedia article and the description of
    /// its item; without an article, the item.
    fn topic(&self, t: &StatementRef) -> String {
        let Some(item) = t.value.item() else {
            return self.value(t);
        };
        let Some(article) = self.view.articles.get(&item) else {
            return self.value(t);
        };
        let link = format!("[{}]({})", article.text, article.href);
        match self
            .view
            .labels
            .get(&EntityId::Item(item))
            .and_then(|l| l.description.as_ref())
        {
            Some(d) => format!("{link} — {}", d.text),
            None => link,
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
            Block::Earlier(b) => earlier::markdown(b, self),
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
            Block::Links(b) => links::markdown(b, self),
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

    /// The notes of the article (`[^3]`), then the sources of statements
    /// (`[^s1]`): each the text of its card.
    fn footnotes(&mut self) {
        let mut lines: Vec<String> = Vec::new();
        for note in &self.view.notes {
            let parts: Vec<String> = note
                .text
                .iter()
                .map(|t| plain(t))
                .chain(
                    note.citations
                        .iter()
                        .map(|c| self.card(&cite::card(c, self.view))),
                )
                .collect();
            lines.push(format!("[^{}]: {}", note.number, parts.join(" ")));
        }
        for (i, hash) in self.sources.borrow().iter().enumerate() {
            if let Some(c) = self.view.citations.get(hash) {
                lines.push(format!(
                    "[^s{}]: {}",
                    i + 1,
                    self.card(&cite::card(c, self.view))
                ));
            }
        }
        if !lines.is_empty() {
            self.paragraph(&lines.join("\n"));
        }
    }

    /// A source in one line: its title as a link, its site, its fields.
    fn card(&self, c: &Card) -> String {
        if let Some(html) = c.html {
            return plain(html);
        }
        let title = c.title.replace('[', "\\[").replace(']', "\\]");
        let mut out = match &c.href {
            Some(href) => format!("[{title}]({href})"),
            None => title,
        };
        if let Some(host) = &c.host {
            out.push_str(&format!(" ({host})"));
        }
        for row in &c.rows {
            let value = match &row.value {
                CiteValue::Parts(parts) => parts
                    .iter()
                    .map(|p| self.part(p))
                    .collect::<Vec<_>>()
                    .join(" · "),
                CiteValue::Language(code) => code.clone(),
                CiteValue::Archive {
                    date,
                    href,
                    host,
                    state,
                    ..
                } => {
                    let mut parts: Vec<String> = date.iter().map(|d| self.part(d)).collect();
                    parts.push(format!("[{host}]({href})"));
                    parts.extend(state.iter().cloned());
                    parts.join(" · ")
                }
            };
            let name = self
                .view
                .messages
                .get(row.key)
                .map_or(row.key, String::as_str);
            out.push_str(&format!(". {name}: {value}"));
        }
        out
    }

    /// The values of a list, and the footnote marks of the sources that
    /// most of them have (`cite::shared`): the list shows those once, after
    /// its key, and those values do not.
    pub fn list_values(&self, statements: &[StatementRef]) -> (Vec<String>, String) {
        let shared = super::cite::shared(statements);
        let marks = shared.map_or_else(String::new, |set| self.source_marks(set));
        let values = statements
            .iter()
            .map(|s| self.value_with(s, shared != Some(&s.citations)))
            .collect();
        (values, marks)
    }

    /// The footnote marks of the sources of a statement: `[^s1][^s2]`.
    fn source_marks(&self, citations: &[String]) -> String {
        let mut sources = self.sources.borrow_mut();
        citations
            .iter()
            .filter(|h| self.view.citations.contains_key(*h))
            .map(|h| {
                let i = sources.iter().position(|s| s == h).unwrap_or_else(|| {
                    sources.push(h.clone());
                    sources.len() - 1
                });
                format!("[^s{}]", i + 1)
            })
            .collect()
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
            Part::Coordinate { text, .. } => text.clone(),
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
        self.value_with(s, true)
    }

    /// A statement; without `sources`, no marks of its sources (its list
    /// has them).
    pub fn value_with(&self, s: &StatementRef, sources: bool) -> String {
        let mut out = self.part(&self.view.part(&s.value));
        let note = self.view.value_note(s);
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
        let marks = if sources {
            self.source_marks(&s.citations)
        } else {
            String::new()
        };
        if marks.is_empty() && sources {
            out.push_str(provenance_words(s.provenance));
        } else {
            out.push_str(&marks);
        }
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

/// The text of cleaned article HTML: tags removed, the mark of a note as
/// its footnote mark (`[^3]`), a formula as its TeX (`$…$`), entities
/// decoded.
pub fn plain(html: &str) -> String {
    let html = note_marks(html);
    let html = formulas(&html);
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

/// The marks of notes (`<sup data-note="3"></sup>`) as footnote marks.
fn note_marks(html: &str) -> String {
    const MARK: &str = "<sup data-note=\"";
    const END: &str = "\"></sup>";
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find(MARK) {
        out.push_str(&rest[..start]);
        let after = &rest[start + MARK.len()..];
        let Some(end) = after.find(END) else {
            out.push_str(&rest[start..]);
            return out;
        };
        out.push_str(&format!("[^{}]", &after[..end]));
        rest = &after[end + END.len()..];
    }
    out.push_str(rest);
    out
}

/// Each formula (`<math alttext="…">…</math>`) as its TeX, `$…$`,
/// without the `{\displaystyle …}` that Wikipedia puts around it. A
/// formula without TeX stays: its text is the text of its MathML.
fn formulas(html: &str) -> String {
    const START: &str = "<math";
    const END: &str = "</math>";
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find(START) {
        out.push_str(&rest[..start]);
        let math = &rest[start..];
        let Some(end) = math.find(END).map(|e| e + END.len()) else {
            break;
        };
        let tag = &math[..math.find('>').unwrap_or_default()];
        let tex = tag
            .split_once("alttext=\"")
            .and_then(|(_, value)| value.split_once('"'))
            .map(|(value, _)| value)
            .filter(|tex| !tex.is_empty());
        match tex {
            Some(tex) => {
                let tex = ["{\\displaystyle ", "{\\textstyle "]
                    .iter()
                    .find_map(|p| tex.strip_prefix(p)?.strip_suffix('}'))
                    .unwrap_or(tex);
                out.push_str(&format!("${tex}$"));
            }
            None => out.push_str(&math[..end]),
        }
        rest = &math[end..];
    }
    out.push_str(rest);
    out
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
            plain(r#"capital of <a href="x">Kenya</a> &amp; more"#),
            "capital of Kenya & more"
        );
    }

    #[test]
    fn marks_of_notes_are_footnote_marks() {
        assert_eq!(
            plain(r#"A city.<sup data-note="1"></sup><sup data-note="12"></sup> B"#),
            "A city.[^1][^12] B"
        );
    }

    #[test]
    fn formulas_are_their_tex() {
        let math =
            |tex: &str| format!("<math alttext=\"{tex}\"><semantics><mi>w</mi></semantics></math>");
        assert_eq!(
            plain(&format!(
                "If {} and {}, or {}.",
                math("{\\displaystyle w_{i}=v_{i}}"),
                math("a&lt;b"),
                "<math><mi>x</mi></math>"
            )),
            "If $w_{i}=v_{i}$ and $a<b$, or x."
        );
    }
}
