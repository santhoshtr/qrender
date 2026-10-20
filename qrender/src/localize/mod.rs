//! Localize: the composition and the text of one language give the view.
//!
//! All the work that depends on the language, and none of the editorial
//! decisions, which the composition already made. The composition stays as
//! it is; the view adds the language in tables (labels, page images,
//! messages) and the prose of each spread. Values keep their raw form: the
//! browser formats them for the locale (`format.rs`).

pub mod article;
pub mod format;
pub mod locale;
pub mod messages;
pub mod view;

use std::collections::{BTreeMap, BTreeSet};

use content::{Citation, Cited, EntityId, Lang, Pid, Qid, Text};
use serde_json::Value;

pub use view::{
    Article, Coverage, Description, Label, Link, Note, PageImage, Project, ProseSection,
    SectionProse, SpreadView, VIEW_SCHEMA, View,
};

use crate::compose::picture::media_kind;
use crate::config::Config;
use crate::schema::{
    Block, COMPOSITION_SCHEMA, Composition, Credit, MediaKind, Picture, PictureFacts, Spread,
    round_to,
};
use crate::wikidata::{AGE, POINT_IN_TIME};
use article::{Links, Numbering, article_url};
use format::entity_url;

const SET_LABEL: &str = "https://www.wikidata.org/wiki/Special:SetLabel";

/// The language of the fallback article.
const FALLBACK: &str = "en";

/// Keys of the composition whose values are identifiers to label.
const LABELLED: [&str; 9] = [
    "item",
    "pid",
    "unit",
    "class",
    "container",
    "via",
    "pids",
    "day",
    "stated_in",
];

/// Keys whose identifiers the coverage counts: what a reader sees.
const COVERED: [&str; 4] = ["item", "pid", "unit", "class"];

/// Reference properties that name the kinds of provenance: stated in,
/// reference URL, imported from Wikimedia project.
const PROVENANCE: [&str; 3] = ["P248", "P854", "P143"];

pub fn localize(composition: &Composition, text: &Text, config: &Config) -> View {
    Localizer::new(composition, text, config).view()
}

struct Localizer<'a> {
    composition: &'a Composition,
    text: &'a Text,
    config: &'a Config,
    lang: &'a Lang,
    /// The composition as JSON, to walk it by the shape of its data.
    json: Value,
    /// The numbers of the notes of the article that the page shows.
    numbering: Option<Numbering<'a>>,
}

impl<'a> Localizer<'a> {
    fn new(composition: &'a Composition, text: &'a Text, config: &'a Config) -> Self {
        Self {
            composition,
            text,
            config,
            lang: &text.lang,
            json: serde_json::to_value(composition).expect("a composition serializes"),
            numbering: text
                .article
                .as_ref()
                .or(text.fallback_article.as_ref())
                .map(|a| Numbering::new(&a.notes)),
        }
    }

    fn view(&self) -> View {
        let qid = self.composition.qid;
        let mut ids = BTreeSet::from([
            EntityId::Item(qid),
            EntityId::Property(*AGE),
            EntityId::Property(*POINT_IN_TIME),
        ]);
        ids.extend(
            PROVENANCE
                .iter()
                .map(|p| EntityId::Property(p.parse().expect("a property id"))),
        );
        walk_ids(&self.json, &LABELLED, &mut ids);
        let mut described = BTreeSet::from([qid]);
        let mut unpictured = BTreeSet::new();
        walk_values(&self.json, &mut described, &mut unpictured);
        let measured = self.measured();
        let mut labels: BTreeMap<_, _> = ids
            .iter()
            .map(|id| {
                let mut label = self.label(*id);
                let shown = match id {
                    EntityId::Item(q) => described.contains(q),
                    EntityId::Property(p) => measured.contains(p),
                };
                if shown {
                    label.description = self.description(*id);
                }
                (*id, label)
            })
            .collect();
        for qid in self.block_concepts() {
            labels.insert(EntityId::Item(qid), self.concept(qid));
        }
        // The prose in reading order, then its notes: a note gets its
        // number at its first mark.
        let lead = self.lead();
        let spreads = self
            .composition
            .spreads
            .iter()
            .map(|s| self.spread(s))
            // A spread without blocks stays for its topics or its
            // prose (a history without significant events).
            .filter(|v| {
                !v.spread.slots.is_empty()
                    || !v.spread.topics.is_empty()
                    || v.prose_text.is_some()
                    || !v.sections.is_empty()
            })
            .collect();
        let more_sections = self.more_sections();
        View {
            schema: VIEW_SCHEMA,
            composition: COMPOSITION_SCHEMA,
            design: self.composition.design,
            qid,
            lang: self.lang.clone(),
            locale: locale::locale(self.lang),
            dir: locale::dir(self.lang),
            page_title: labels[&EntityId::Item(qid)].text.clone(),
            page_images: unpictured
                .iter()
                .filter_map(|q| Some((*q, self.page_image(*q)?)))
                .collect(),
            messages: messages::for_lang(self.lang),
            lead,
            coverage: self.coverage(),
            projects: self.projects(),
            spreads,
            unmatched_sections: self.unmatched_sections(),
            more_sections,
            set_aside: self.composition.set_aside.clone(),
            notes: self.notes(),
            citations: self
                .composition
                .citations
                .iter()
                .map(|(hash, c)| (hash.clone(), c.clone()))
                .collect(),
            labels,
        }
    }

    // ------------------------------------------------------------- labels

    /// A label with its language and a fallback mark. A label that is not in
    /// the page language carries the address where a reader adds it: one
    /// small edit that every page using it gains.
    fn label(&self, id: EntityId) -> Label {
        let found = self.text.labels.get(&id).and_then(|t| t.label.as_ref());
        let (text, lang) = match found {
            Some(l) => (l.text.clone(), Some(l.lang.clone())),
            None => (id.to_string(), None),
        };
        let fallback = !lang.as_ref().is_some_and(|l| self.is_page_language(l));
        Label {
            text,
            lang,
            fallback,
            add: fallback.then(|| format!("{SET_LABEL}/{}/{id}", self.lang)),
            href: entity_url(id),
            description: None,
        }
    }

    /// The properties of measurements (series, dated rows): their
    /// description says what the number is.
    fn measured(&self) -> BTreeSet<Pid> {
        self.blocks()
            .flat_map(|b| match b {
                Block::Series(s) => s.lines.iter().map(|l| l.pid).collect(),
                Block::Fact(f) if f.measured => vec![f.pid],
                _ => Vec::new(),
            })
            .collect()
    }

    /// The concepts that title a block (a split): labelled as the titles
    /// of spreads are, from the concepts of the text.
    fn block_concepts(&self) -> BTreeSet<Qid> {
        self.blocks()
            .filter_map(|b| match b {
                Block::Series(s) => s.concept,
                Block::Split(s) => Some(s.concept),
                _ => None,
            })
            .collect()
    }

    fn blocks(&self) -> impl Iterator<Item = &'a Block> {
        self.composition
            .spreads
            .iter()
            .flat_map(|s| s.slots.values().flatten())
    }

    fn description(&self, id: EntityId) -> Option<Description> {
        let d = self.text.labels.get(&id)?.description.as_ref()?;
        Some(Description {
            text: d.text.clone(),
            lang: d.lang.clone(),
            fallback: !self.is_page_language(&d.lang),
        })
    }

    /// The label of a concept: the title of a spread.
    fn concept(&self, qid: Qid) -> Label {
        let label = self.text.concepts.get(&qid).and_then(|c| c.label.as_ref());
        Label {
            text: label.map_or_else(|| qid.to_string(), |l| l.text.clone()),
            lang: label.map(|l| l.lang.clone()),
            fallback: !label.is_some_and(|l| self.is_page_language(&l.lang)),
            add: None,
            href: entity_url(EntityId::Item(qid)),
            description: None,
        }
    }

    /// A `mul` label is the same in every language, so it is not a fallback.
    fn is_page_language(&self, lang: &Lang) -> bool {
        lang == self.lang || lang.as_str() == "mul"
    }

    /// Label coverage in the page language, over the items and properties
    /// a reader sees: every spread but evidence.
    fn coverage(&self) -> Coverage {
        let mut ids = BTreeSet::new();
        for spread in self.json["spreads"].as_array().into_iter().flatten() {
            if spread["id"] != "evidence" {
                walk_ids(&spread["slots"], &COVERED, &mut ids);
            }
        }
        let have = ids
            .iter()
            .filter(|id| {
                let label = self.text.labels.get(*id).and_then(|t| t.label.as_ref());
                label.is_some_and(|l| self.is_page_language(&l.lang))
            })
            .count();
        Coverage {
            have,
            total: ids.len(),
        }
    }

    // ------------------------------------------------------------- images

    /// A picture from the item's article, for a value without a picture.
    fn page_image(&self, qid: Qid) -> Option<PageImage> {
        let image = self.text.page_images.get(&qid)?;
        let kind = match media_kind(&image.file) {
            MediaKind::Drawing => MediaKind::Drawing,
            _ => MediaKind::Photo,
        };
        Some(PageImage {
            picture: Picture {
                file: image.file.clone(),
                facts: PictureFacts {
                    kind,
                    aspect: (image.height > 0)
                        .then(|| round_to(f64::from(image.width) / f64::from(image.height), 4)),
                    megapixels: None,
                    bytes: None,
                    url: None,
                    assessed: Vec::new(),
                    credit: Credit {
                        artist: image.artist.clone(),
                        license: image.license.clone(),
                    },
                },
            },
            from: image.source.clone(),
        })
    }

    // -------------------------------------------------------------- prose

    /// The article in the page language, or the fallback article.
    fn article(&self) -> Option<(&'a content::Article, &'a str)> {
        match (&self.text.article, &self.text.fallback_article) {
            (Some(a), _) => Some((a, self.lang.as_str())),
            (None, Some(a)) => Some((a, FALLBACK)),
            (None, None) => None,
        }
    }

    fn rewrite(&self, html: &str, article: &content::Article, source: &str) -> String {
        self.rewrite_links(html, article, source, false)
    }

    /// `external`: links to other sites stay (the text of a citation).
    fn rewrite_links(
        &self,
        html: &str,
        article: &content::Article,
        source: &str,
        external: bool,
    ) -> String {
        let links = Links {
            items: &article.links,
            source,
            page: self.lang.as_str(),
            labels: &self.text.labels,
            notes: self.numbering.as_ref(),
            external,
        };
        article::rewrite(html, &links)
    }

    /// The notes that the prose of the page marks, by number. A note that
    /// an explanatory note marks gets the next number, so it comes after.
    fn notes(&self) -> Vec<Note> {
        let (Some(numbering), Some((article, source))) = (&self.numbering, self.article()) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut number = 1;
        while let Some((_, note)) = numbering.note(number) {
            out.push(Note {
                number,
                group: note.group.clone(),
                lang: source.parse().expect("a language code"),
                citations: note
                    .citations
                    .iter()
                    .map(|c| self.cited(c, article, source))
                    .collect(),
                text: note.text.as_ref().map(|t| self.rewrite(t, article, source)),
            });
            number += 1;
        }
        out
    }

    /// A citation of a note: the HTML of free text is cleaned as prose is,
    /// with its links to other sites.
    fn cited(&self, cited: &Cited, article: &content::Article, source: &str) -> Cited {
        match &cited.citation {
            Citation::Text { html } => Cited {
                citation: Citation::Text {
                    html: self.rewrite_links(html, article, source, true),
                },
                origin: cited.origin.clone(),
            },
            _ => cited.clone(),
        }
    }

    fn lead(&self) -> Option<Article> {
        let (article, source) = self.article()?;
        let mut paragraphs: Vec<String> = article
            .paragraphs
            .iter()
            .map(|p| self.rewrite(p, article, source))
            .collect();
        let more = paragraphs.split_off(paragraphs.len().min(1));
        Some(Article {
            lang: source.parse().expect("a language code"),
            local: source == self.lang.as_str(),
            host: format!("{source}.wikipedia.org"),
            url: article_url(source, &article.title),
            paragraphs,
            more,
        })
    }

    fn spread(&self, spread: &Spread) -> SpreadView {
        let concepts = spread
            .prose
            .as_ref()
            .map_or(&[][..], |p| p.concepts.as_slice());
        let sections = self.outline_sections(concepts);
        SpreadView {
            spread: spread.clone(),
            title: spread.concept.map(|q| self.concept(q)),
            prose_text: sections
                .is_empty()
                .then(|| self.section_prose(concepts))
                .flatten(),
            prose_links: self.section_links(concepts),
            sections,
        }
    }

    /// The sections of the concepts, with their content: the outline of
    /// the fallback article (empty for an article in the page language).
    fn outline_sections(&self, concepts: &[Qid]) -> Vec<ProseSection> {
        let Some((article, source)) = self.article() else {
            return Vec::new();
        };
        let mut anchors: Vec<&String> = concepts
            .iter()
            .flat_map(|q| article.concept_sections.get(q).into_iter().flatten())
            .collect();
        let mut seen = BTreeSet::new();
        anchors.retain(|a| seen.insert(*a));
        anchors
            .into_iter()
            .filter_map(|a| article.outline.iter().find(|s| s.anchor == *a))
            .map(|s| self.prose_section(s, article, source))
            .collect()
    }

    fn prose_section(
        &self,
        s: &content::OutlineSection,
        article: &content::Article,
        source: &str,
    ) -> ProseSection {
        ProseSection {
            title: s.title.clone(),
            anchor: s.anchor.clone(),
            href: format!("{}#{}", article_url(source, &article.title), s.anchor),
            lang: source.parse().expect("a language code"),
            level: s.level,
            blocks: s
                .blocks
                .iter()
                .map(|b| self.rewrite(b, article, source))
                .filter(|b| !b.is_empty())
                .collect(),
            sections: s
                .sections
                .iter()
                .map(|c| self.prose_section(c, article, source))
                .collect(),
        }
    }

    /// The first paragraph of the first matched section of the concepts.
    fn section_prose(&self, concepts: &[Qid]) -> Option<SectionProse> {
        let (article, source) = self.article()?;
        concepts
            .iter()
            .flat_map(|q| article.concept_sections.get(q).into_iter().flatten())
            .find_map(|anchor| {
                let html = article.section_text.get(anchor)?;
                Some(SectionProse {
                    html: self.rewrite(html, article, source),
                    lang: source.parse().expect("a language code"),
                    local: source == self.lang.as_str(),
                    title: section_title(article, anchor),
                    href: format!("{}#{anchor}", article_url(source, &article.title)),
                })
            })
    }

    fn section_links(&self, concepts: &[Qid]) -> Vec<Link> {
        let Some((article, source)) = self.article() else {
            return Vec::new();
        };
        concepts
            .iter()
            .flat_map(|q| article.concept_sections.get(q).into_iter().flatten())
            .map(|anchor| section_link(article, source, anchor))
            .collect()
    }

    /// The titles of the sections that no spread shows, when the page does
    /// not show their content (`more_sections`).
    fn unmatched_sections(&self) -> Vec<Link> {
        match self.article() {
            Some((article, source)) if article.outline.is_empty() => self
                .unmatched(article, source)
                .into_iter()
                .map(|s| section_link(article, source, &s.anchor))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The sections that no spread shows, with their content: for a
    /// fallback article.
    fn more_sections(&self) -> Vec<ProseSection> {
        let Some((article, source)) = self.article() else {
            return Vec::new();
        };
        self.unmatched(article, source)
            .into_iter()
            .filter_map(|s| article.outline.iter().find(|o| o.anchor == s.anchor))
            .map(|s| self.prose_section(s, article, source))
            .collect()
    }

    /// The top-level sections of the article that no spread of this page
    /// shows, without the apparatus (notes, references, links).
    fn unmatched(&self, article: &'a content::Article, source: &str) -> Vec<&'a content::Section> {
        // The sections that a spread of this page shows: the sections of its
        // prose concepts. A concept of another layout shows nothing here.
        let used: BTreeSet<&String> = self
            .composition
            .spreads
            .iter()
            .filter_map(|s| s.prose.as_ref())
            .flat_map(|p| &p.concepts)
            .flat_map(|q| article.concept_sections.get(q).into_iter().flatten())
            .collect();
        // Notes, references, links: not sections to read.
        let apparatus: Vec<String> = self
            .config
            .layouts
            .apparatus
            .get(source)
            .into_iter()
            .flatten()
            .map(|t| t.to_lowercase())
            .collect();
        article
            .sections
            .iter()
            .filter(|s| !used.contains(&s.anchor))
            .filter(|s| !apparatus.contains(&s.title.trim().to_lowercase()))
            .collect()
    }

    // ----------------------------------------------------------- projects

    fn projects(&self) -> Vec<Project> {
        let families = self
            .composition
            .spreads
            .iter()
            .flat_map(|s| s.slots.values().flatten())
            .find_map(|b| match b {
                Block::Sitelinks(s) => Some(&s.families),
                _ => None,
            });
        let mut projects: Vec<Project> = families
            .into_iter()
            .flatten()
            .map(|(name, codes)| Project {
                name: name.clone(),
                count: codes.len(),
                local: codes.iter().any(|c| c == self.lang.as_str()),
            })
            .collect();
        projects.sort_by_key(|p| std::cmp::Reverse(p.count));
        projects
    }
}

fn section_title(article: &content::Article, anchor: &str) -> String {
    article
        .sections
        .iter()
        .find(|s| s.anchor == anchor)
        .map_or_else(|| anchor.to_string(), |s| s.title.clone())
}

fn section_link(article: &content::Article, source: &str, anchor: &str) -> Link {
    Link {
        text: section_title(article, anchor),
        lang: source.parse().expect("a language code"),
        href: format!("{}#{anchor}", article_url(source, &article.title)),
    }
}

/// Every identifier under one of `keys`, as a string or in a list.
fn walk_ids(value: &Value, keys: &[&str], ids: &mut BTreeSet<EntityId>) {
    match value {
        Value::Object(o) => {
            for (k, v) in o {
                if keys.contains(&k.as_str()) {
                    let strings = v
                        .as_str()
                        .into_iter()
                        .chain(v.as_array().into_iter().flatten().filter_map(Value::as_str));
                    ids.extend(strings.filter_map(|s| s.parse::<EntityId>().ok()));
                }
                walk_ids(v, keys, ids);
            }
        }
        Value::Array(a) => a.iter().for_each(|v| walk_ids(v, keys, ids)),
        _ => {}
    }
}

/// The item values whose description is shown, and the item values that
/// have no picture of their own (a statement or a work, not a class).
fn walk_values(value: &Value, described: &mut BTreeSet<Qid>, unpictured: &mut BTreeSet<Qid>) {
    match value {
        Value::Object(o) => {
            if let Some(item) = o
                .get("item")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<Qid>().ok())
            {
                if o.get("described") == Some(&Value::Bool(true)) {
                    described.insert(item);
                }
                let is_value = o.contains_key("rank") || o.contains_key("via");
                let pictured = o.get("image").is_some_and(|i| !i.is_null());
                let class = o.get("class") == Some(&Value::Bool(true));
                if is_value && !pictured && !class {
                    unpictured.insert(item);
                }
            }
            o.values()
                .for_each(|v| walk_values(v, described, unpictured));
        }
        Value::Array(a) => a.iter().for_each(|v| walk_values(v, described, unpictured)),
        _ => {}
    }
}
