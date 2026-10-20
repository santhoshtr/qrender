//! What the templates ask for: lookups in the view, values ready to show,
//! picture sources, page-unique ids. Data only; the markup is in the
//! templates.

use std::cell::Cell;

use content::{EntityId, Pid, Qid};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use super::map::{Frame, frame, live, point_frame};
use crate::localize::format::{Part, coordinate, edit_url, number_part, percent};
use crate::localize::{Description, Label, SpreadView, View};
use crate::primitives::credits::{Credits, Person};
use crate::primitives::identifiers::{Identifier, Identifiers};
use crate::primitives::links::{Card, Links};
use crate::primitives::map::{Map, Marker};
use crate::primitives::media::Media;
use crate::primitives::names::{self, Names};
use crate::primitives::provenance;
use crate::primitives::succession::Succession;
use crate::primitives::timeline::When;
use crate::primitives::works::{Work, Works};
use crate::schema::{
    Credit, Picture, Provenance, QualifierPart, QualifierShown, StatementRef, ValueShown,
};
use crate::vocab::Slot;
use crate::wikidata::POINT_IN_TIME;

/// No citations: for a value that has no statement (a work by the item,
/// a row of the provenance legend).
pub static NO_CITATIONS: Vec<String> = Vec::new();

/// The values of a strip of pictures from which it has "View all".
const VIEW_ALL_MIN: usize = 3;

pub const FILEPATH: &str = "https://commons.wikimedia.org/wiki/Special:FilePath/";
pub const WIKIPEDIA_LOGO: &str =
    "https://commons.wikimedia.org/wiki/Special:FilePath/Wikipedia-logo-v2.svg?width=60";

/// The icons of the sites in the web spread: a Toolforge service, so the
/// sites do not see the readers.
pub const FAVICONS: &str = "https://favicons.toolforge.org/favicon";

/// The thumbnail widths that Wikimedia renders in advance.
const STANDARD_WIDTHS: [u32; 5] = [330, 500, 960, 1280, 1920];

/// Characters that Python's `urllib.parse.quote` leaves as they are.
const PATH: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'_')
    .remove(b'.')
    .remove(b'-')
    .remove(b'~')
    .remove(b'/');

/// A template argument: Askama passes a value or a reference to it (one
/// or two levels deep), depending on where it comes from.
pub trait Arg<T> {
    fn get(&self) -> T;
}

macro_rules! arg {
    ($($t:ty),*) => {$(
        impl Arg<$t> for $t {
            fn get(&self) -> $t {
                *self
            }
        }
    )*};
}

arg!(
    f64,
    i64,
    u8,
    u64,
    u32,
    usize,
    Qid,
    Pid,
    Slot,
    When,
    Option<f64>,
    Option<Provenance>
);

impl<T, U: Arg<T> + ?Sized> Arg<T> for &U {
    fn get(&self) -> T {
        (**self).get()
    }
}

impl Arg<Option<Provenance>> for Option<&Provenance> {
    fn get(&self) -> Option<Provenance> {
        self.copied()
    }
}

impl Arg<Option<Provenance>> for Provenance {
    fn get(&self) -> Option<Provenance> {
        Some(*self)
    }
}

/// A template argument by reference: an owned value (a label that a `Ui`
/// method made) or a reference to it.
pub trait Ref<T: ?Sized> {
    fn r(&self) -> &T;
}

macro_rules! by_ref {
    ($($t:ty),*) => {$(
        impl Ref<$t> for $t {
            fn r(&self) -> &$t {
                self
            }
        }
    )*};
}

by_ref!(Label, Part, Credit, str);

impl<'a> Ref<Shown<'a>> for Shown<'a> {
    fn r(&self) -> &Shown<'a> {
        self
    }
}

impl Ref<str> for String {
    fn r(&self) -> &str {
        self
    }
}

impl<T: ?Sized, U: Ref<T> + ?Sized> Ref<T> for &U {
    fn r(&self) -> &T {
        (**self).r()
    }
}

pub struct Ui<'v> {
    pub view: &'v View,
    /// Ids for popovers and dialogs, unique in one page.
    ids: Cell<u32>,
}

/// One value, ready to show: in a line, a figure or a dialog.
pub struct Shown<'a> {
    pub part: Part,
    /// The qualifiers, and for a person in the credits the roles first.
    pub note: Vec<Part>,
    /// For a value whose description is shown (a work).
    pub description: Option<Description>,
    pub image: Option<Image<'a>>,
    pub provenance: Option<Provenance>,
    /// The references that cite a source, by hash.
    pub citations: &'a Vec<String>,
    pub edit: Option<String>,
    /// False when the value is behind "+N".
    pub shown: bool,
}

/// A picture of a value: its own, or one from its article (`from`).
pub struct Image<'a> {
    pub picture: &'a Picture,
    pub from: Option<&'a str>,
}

impl<'v> Ui<'v> {
    pub fn new(view: &'v View) -> Self {
        Self {
            view,
            ids: Cell::new(0),
        }
    }

    /// The next id of the page.
    pub fn id(&self) -> u32 {
        self.ids.set(self.ids.get() + 1);
        self.ids.get()
    }

    pub fn msg(&self, key: &str) -> &'v str {
        self.view.messages.get(key).map_or("", String::as_str)
    }

    /// The label of an entity. Localize labels every entity of the
    /// composition; an entity without one shows its id.
    pub fn label(&self, id: &EntityId) -> Label {
        self.view.labels.get(id).cloned().unwrap_or_else(|| Label {
            text: id.to_string(),
            lang: None,
            fallback: true,
            add: None,
            href: crate::localize::format::entity_url(*id),
            description: None,
        })
    }

    pub fn property(&self, pid: impl Arg<Pid>) -> Label {
        self.label(&EntityId::Property(pid.get()))
    }

    pub fn item(&self, qid: impl Arg<Qid>) -> Label {
        self.label(&EntityId::Item(qid.get()))
    }

    /// A statement value as a part.
    pub fn value_part(&self, value: &ValueShown) -> Part {
        self.view.part(value)
    }

    /// Qualifiers as parts.
    pub fn qualifiers(&self, qualifiers: &[QualifierShown]) -> Vec<Part> {
        self.view.note(qualifiers)
    }

    /// Qualifiers grouped by property, each with its label: "statement
    /// disputed by: A, B".
    pub fn labelled(&self, qualifiers: &[QualifierShown]) -> Vec<(Label, Vec<Part>)> {
        self.view
            .labelled_note(qualifiers)
            .into_iter()
            .map(|(pid, parts)| (self.property(pid), parts))
            .collect()
    }

    pub fn year(&self, year: impl Arg<i64>) -> String {
        self.view.year(year.get())
    }

    pub fn edit(&self, pid: impl Arg<Pid>) -> String {
        edit_url(self.view.qid, pid.get())
    }

    /// The groups of languages of the names, for the page language.
    pub fn name_groups<'a>(&'a self, b: &'a Names) -> Vec<names::Group<'a>> {
        names::groups(b, self.view.lang.as_str())
    }

    /// The note of a value in the names block. An item value is without its
    /// languages: the group names the language. A text keeps them: they
    /// can say more than its code (a variety of the language).
    pub fn name_note(&self, s: &StatementRef) -> Vec<Part> {
        let item = matches!(s.value, ValueShown::Item { .. });
        let qualifiers: Vec<QualifierShown> = s
            .qualifiers
            .iter()
            .filter(|q| !item || q.pid != *crate::wikidata::LANGUAGE)
            .cloned()
            .collect();
        self.qualifiers(&qualifiers)
    }

    /// A strip of this many pictures has the "View all" card: its dialog
    /// shows every value with its qualifiers, more than the strip.
    pub fn view_all(&self, count: impl Arg<usize>) -> bool {
        count.get() >= VIEW_ALL_MIN
    }

    /// The file of a media statement.
    pub fn statement_file<'a>(&self, s: &'a StatementRef) -> &'a str {
        match &s.value {
            ValueShown::File { file } => file,
            _ => "",
        }
    }

    /// The map of a point, centred on it (render/map.rs).
    pub fn point_frame(&self, lat: impl Arg<f64>, lon: impl Arg<f64>) -> Frame<'static> {
        point_frame(lat.get(), lon.get())
    }

    /// The data of the map of a dialog (map.js): the frame, with the
    /// markers and the shapes of the map.
    pub fn live_map(&self, f: &Frame, b: &Map) -> String {
        live(f, Some(b), |pid| self.property(pid).text)
    }

    /// The place of a marker, as text: a coordinate part would bring its
    /// own pin and dialog into the dialog of the map.
    pub fn marker_place(&self, mk: &Marker) -> String {
        match mk.value {
            ValueShown::Coordinate { lat, lon, .. } => coordinate(lat, lon),
            _ => String::new(),
        }
    }

    /// The data of the map of a coordinate pin: the point only.
    pub fn live_point(&self, f: &Frame) -> String {
        live(f, None, |pid| self.property(pid).text)
    }

    /// The date of an event: a year, or a span ("1947 – 1975").
    pub fn when(&self, w: impl Arg<When>) -> String {
        let w = w.get();
        let year = |y: Option<i64>| y.map(|y| self.view.year(y)).unwrap_or_default();
        match w.single() {
            Some(y) => self.view.year(y),
            None => format!("{} – {}", year(w.start), year(w.end))
                .trim()
                .to_string(),
        }
    }

    /// The label of "point in time" (P585): the heading of the dates of a
    /// series.
    pub fn point_in_time(&self) -> Label {
        self.property(*POINT_IN_TIME)
    }

    /// The words for an imported statement: the label of P143.
    pub fn imported(&self) -> String {
        self.property("P143".parse::<Pid>().expect("a property id"))
            .text
    }

    // -------------------------------------------------------------- values

    /// A statement, ready to show.
    pub fn value<'a>(&'a self, s: &'a StatementRef) -> Shown<'a> {
        let item = match s.value {
            ValueShown::Item { item } => item,
            _ => None,
        };
        Shown {
            part: self.value_part(&s.value),
            note: self.view.value_note(s),
            description: item
                .filter(|_| s.described)
                .and_then(|q| self.item(q).description),
            image: self.image(item, s.image.as_ref(), s.class),
            provenance: Some(s.provenance),
            citations: &s.citations,
            edit: Some(self.edit(s.pid)),
            shown: s.shown.unwrap_or(true),
        }
    }

    /// A person in the credits: the roles are the first part of the note.
    pub fn person<'a>(&'a self, p: &'a Person) -> Shown<'a> {
        let mut shown = self.value(&p.statement);
        let mut note: Vec<Part> = p
            .pids
            .iter()
            .map(|pid| Part::Entity(EntityId::Property(*pid)))
            .collect();
        note.append(&mut shown.note);
        shown.note = note;
        shown
    }

    /// A work by the item: its year is its note.
    pub fn work<'a>(&'a self, w: &'a Work, index: usize, shown_count: usize) -> Shown<'a> {
        Shown {
            part: Part::Entity(EntityId::Item(w.item)),
            note: w
                .year
                .map(|y| Part::Text {
                    text: self.view.year(y),
                    lang: None,
                })
                .into_iter()
                .collect(),
            description: None,
            image: self.image(Some(w.item), w.image.as_ref(), false),
            provenance: None,
            citations: &NO_CITATIONS,
            edit: None,
            shown: index < shown_count,
        }
    }

    /// The picture of an item value: its own, else a page image. A class
    /// value has none.
    fn image<'a>(
        &'a self,
        item: Option<Qid>,
        own: Option<&'a Picture>,
        class: bool,
    ) -> Option<Image<'a>> {
        if let Some(picture) = own {
            return Some(Image {
                picture,
                from: None,
            });
        }
        let page = self.view.page_images.get(&item?).filter(|_| !class)?;
        Some(Image {
            picture: &page.picture,
            from: Some(&page.from),
        })
    }

    // ------------------------------------------------------------ pictures

    /// The Commons address of a file at a width.
    pub fn src(&self, file: &str, width: impl Arg<u32>) -> String {
        format!("{FILEPATH}{}?width={}", quote(file), width.get())
    }

    pub fn srcset(&self, file: &str) -> String {
        STANDARD_WIDTHS
            .iter()
            .map(|w| format!("{} {w}w", self.src(file, *w)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The file itself (a sound, a video).
    pub fn file(&self, file: &str) -> String {
        format!("{FILEPATH}{}", quote(file))
    }

    /// The page of a file on Commons.
    pub fn commons_page(&self, title: &str) -> String {
        let title = title.replace(' ', "_");
        let title = title.strip_prefix("File:").unwrap_or(&title);
        format!("https://commons.wikimedia.org/wiki/File:{}", quote(title))
    }

    /// The page of a Commons data file ("Data:Population.tab").
    pub fn commons_data(&self, title: &str) -> String {
        format!(
            "https://commons.wikimedia.org/wiki/{}",
            quote(&title.replace(' ', "_"))
        )
    }

    /// "12 MB" for a video.
    pub fn megabytes(&self, bytes: impl Arg<u64>) -> String {
        format!("{:.0} MB", bytes.get() as f64 / 1e6)
    }

    /// A number for a CSS custom property, without a unit.
    pub fn css_number(&self, x: impl Arg<f64>) -> String {
        let s = format!("{:.6}", x.get());
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// A row of the provenance legend.
pub struct LegendRow {
    pub kind: &'static str,
    pub mark: Provenance,
    pub labels: Vec<Label>,
    pub count: usize,
}

impl<'v> Ui<'v> {
    // ---------------------------------------------------------------- lists

    pub fn values<'a>(&'a self, statements: &'a [StatementRef]) -> Vec<Shown<'a>> {
        statements.iter().map(|s| self.value(s)).collect()
    }

    /// Some value of the list has a picture.
    pub fn any_image(&self, values: &[Shown]) -> bool {
        values.iter().any(|s| s.image.is_some())
    }

    /// Every holder of an office: the holder now, then the holders before.
    pub fn holders<'a>(&'a self, b: &'a Succession) -> Vec<Shown<'a>> {
        std::iter::once(&b.holder)
            .chain(&b.before)
            .map(|s| self.value(s))
            .collect()
    }

    pub fn persons<'a>(&'a self, credits: &'a Credits) -> Vec<Shown<'a>> {
        credits.values.iter().map(|p| self.person(p)).collect()
    }

    pub fn works<'a>(&'a self, works: &'a Works) -> Vec<Shown<'a>> {
        works
            .values
            .iter()
            .enumerate()
            .map(|(i, w)| self.work(w, i, works.shown_count))
            .collect()
    }

    /// How many values are behind "+N".
    pub fn hidden(&self, values: &[Shown]) -> usize {
        values.iter().filter(|v| !v.shown).count()
    }

    pub fn hidden_refs(&self, values: &[StatementRef]) -> usize {
        values.iter().filter(|v| v.shown == Some(false)).count()
    }

    /// Labels of properties, as one heading: "director, screenwriter".
    pub fn heading(&self, pids: &[Pid]) -> String {
        pids.iter()
            .map(|p| self.property(*p).text)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The title of the holders before: the office, else the property.
    pub fn succession_title(&self, b: &Succession) -> String {
        match b.office.as_ref().map(|o| &o.value) {
            Some(ValueShown::Item { item: Some(q) }) => self.item(*q).text,
            _ => self.property(b.pids[0]).text,
        }
    }

    /// The roles of a holder, as one label: "head of government · head of
    /// state".
    pub fn roles(&self, pids: &[Pid]) -> Label {
        let labels: Vec<Label> = pids.iter().map(|p| self.property(*p)).collect();
        Label {
            text: labels
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join(" · "),
            ..labels[0].clone()
        }
    }

    pub fn identifiers<'a>(&self, b: &'a Identifiers) -> Vec<&'a Identifier> {
        let mut ids: Vec<&Identifier> = b.values.iter().collect();
        ids.sort_by_cached_key(|i| self.property(i.pid).text.to_lowercase());
        ids
    }

    // ---------------------------------------------------------------- slots

    /// The slots that are a definition list of rows.
    pub fn is_sheet(&self, slot: impl Arg<Slot>) -> bool {
        matches!(slot.get(), Slot::Sheet | Slot::Absent | Slot::Web)
    }

    /// The slots whose content has a wrapper of the slot's name.
    pub fn wraps(&self, slot: impl Arg<Slot>) -> bool {
        matches!(slot.get(), Slot::Run | Slot::Feature | Slot::Band)
    }

    /// The number of blocks of a collapsed spread.
    pub fn block_count(&self, s: &SpreadView) -> usize {
        s.spread.slots.values().map(Vec::len).sum()
    }

    /// The links under a spread: its article sections (but those the
    /// spread shows: a paragraph or the content), then its topics.
    pub fn prose_links(&self, s: &SpreadView) -> Vec<Part> {
        let shown: Vec<&str> = s
            .prose_text
            .iter()
            .map(|p| p.href.as_str())
            .chain(s.sections.iter().map(|a| a.href.as_str()))
            .collect();
        let sections = s
            .prose_links
            .iter()
            .filter(|l| !shown.contains(&l.href.as_str()))
            .map(|l| Part::Link {
                text: l.text.clone(),
                href: l.href.clone(),
            });
        let topics = s.spread.topics.iter().map(|t| self.value_part(&t.value));
        sections.chain(topics).collect()
    }

    // --------------------------------------------------------------- blocks

    pub fn media_file<'a>(&self, m: &'a Media) -> &'a str {
        match &m.statement.value {
            ValueShown::File { file } => file,
            _ => "",
        }
    }

    /// An aspect ratio for CSS, 4:3 when it is not known.
    pub fn aspect(&self, aspect: impl Arg<Option<f64>>) -> String {
        self.css_number(aspect.get().unwrap_or(4.0 / 3.0))
    }

    pub fn credit_text(&self, credit: &Credit) -> String {
        [credit.artist.as_str(), credit.license.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }

    pub fn number(&self, x: impl Arg<f64>) -> Part {
        number_part(x.get(), None)
    }

    pub fn percent(&self, ratio: impl Arg<f64>) -> Part {
        percent(ratio.get())
    }

    /// A share for a bar, at most the whole.
    pub fn share(&self, ratio: impl Arg<f64>) -> String {
        self.css_number(ratio.get().min(1.0))
    }

    pub fn coordinate(&self, lat: impl Arg<f64>, lon: impl Arg<f64>) -> String {
        coordinate(lat.get(), lon.get())
    }

    pub fn map<'b>(&self, b: &'b Map) -> Option<Frame<'b>> {
        frame(b)
    }

    pub fn legend(&self, b: &provenance::Provenance) -> Vec<LegendRow> {
        let label = |p: &str| self.property(p.parse::<Pid>().expect("a property id"));
        vec![
            LegendRow {
                kind: "sourced",
                mark: Provenance::Sourced,
                labels: vec![label("P248"), label("P854")],
                count: b.sourced,
            },
            LegendRow {
                kind: "imported",
                mark: Provenance::Imported,
                labels: vec![label("P143")],
                count: b.imported,
            },
            LegendRow {
                kind: "none",
                mark: Provenance::None,
                labels: Vec::new(),
                count: b.none,
            },
        ]
    }

    /// The name of a recording's choice: its language, else its note,
    /// else its file.
    pub fn sound_label(&self, s: &crate::primitives::sounds::Sound) -> String {
        if let Some(language) = s.language {
            return self.item(language).text;
        }
        match self.qualifiers(&s.media.statement.qualifiers).first() {
            Some(part) => self.part_text(part),
            None => self.media_file(&s.media).to_string(),
        }
    }

    /// The links of the web spread, a row for each property, in order. A
    /// row whose addresses are all on one site has the icon of the site
    /// once; otherwise each address has its own.
    pub fn link_rows<'a>(&self, b: &'a Links) -> Vec<LinkRow<'a>> {
        let mut rows: Vec<LinkRow> = Vec::new();
        for card in &b.cards {
            match rows.last_mut() {
                Some(row) if row.pid == card.statement.pid => row.cards.push((card, None)),
                _ => rows.push(LinkRow {
                    pid: card.statement.pid,
                    icon: None,
                    cards: vec![(card, None)],
                }),
            }
        }
        for row in &mut rows {
            let domains: Vec<Option<&str>> = row
                .cards
                .iter()
                .map(|(c, _)| c.href.as_deref().and_then(domain))
                .collect();
            if domains.iter().all(|d| d.is_some() && *d == domains[0]) {
                row.icon = domains[0].map(favicon);
            } else {
                for ((_, icon), d) in row.cards.iter_mut().zip(domains) {
                    *icon = d.map(favicon);
                }
            }
        }
        rows
    }

    /// The note of a link: its languages, roles and dates; not its counts
    /// and codes (subscribers, a numeric account id), which the data has.
    pub fn link_note(&self, card: &Card) -> Vec<Part> {
        let qualifiers: Vec<QualifierShown> = card
            .statement
            .qualifiers
            .iter()
            .filter(|q| {
                matches!(
                    q.value,
                    QualifierPart::Item { .. } | QualifierPart::Year { .. }
                )
            })
            .cloned()
            .collect();
        self.qualifiers(&qualifiers)
    }

    /// The recording in the page language: the choice starts at it.
    pub fn sound_selected(&self, s: &crate::primitives::sounds::Sound) -> bool {
        s.lang.as_deref() == Some(self.view.lang.as_str())
    }

    /// The note of a recording, without its language: the choice names it.
    pub fn sound_note(&self, s: &crate::primitives::sounds::Sound) -> Vec<Part> {
        let qualifiers: Vec<QualifierShown> = s
            .media
            .statement
            .qualifiers
            .iter()
            .filter(|q| s.language.is_none() || q.pid != *crate::wikidata::LANGUAGE)
            .cloned()
            .collect();
        self.qualifiers(&qualifiers)
    }

    /// The first statement of a row.
    pub fn first<'a>(&self, statements: &'a [StatementRef]) -> &'a StatementRef {
        &statements[0]
    }

    /// The part of the value at a place.
    pub fn part_at(&self, values: &[Shown], i: usize) -> Part {
        values[i].part.clone()
    }

    /// The amount of a quantity statement (the low end of a range).
    pub fn amount(&self, s: &StatementRef) -> f64 {
        match s.value {
            ValueShown::Amount { amount, .. } => amount,
            _ => 0.0,
        }
    }

    /// "aged 95": the age at death.
    pub fn age(&self, years: impl Arg<i64>) -> String {
        crate::localize::messages::format(self.msg("qrender-age"), &[&years.get().to_string()])
    }

    pub fn entity(&self, qid: impl Arg<Qid>) -> Part {
        Part::Entity(EntityId::Item(qid.get()))
    }

    /// The neutral text of a part, for an attribute.
    pub fn part_text(&self, p: &Part) -> String {
        match p {
            Part::Entity(id) => self.label(id).text,
            Part::Number { text, .. }
            | Part::Time { text, .. }
            | Part::Text { text, .. }
            | Part::Link { text, .. } => text.clone(),
            Part::Qualified { pid, value } => {
                format!("{}: {}", self.property(*pid).text, self.part_text(value))
            }
            Part::File { pid, file, .. } => format!("{}: {file}", self.property(*pid).text),
            Part::Coordinate { text, .. } => text.clone(),
        }
    }

    /// A place in a frame, in percent, for CSS.
    pub fn percent_css(&self, x: impl Arg<f64>) -> String {
        format!("{:.2}", x.get())
    }

    /// "imported", "none", or "" for a sourced statement: its mark.
    /// The citations that every value of a list has (`cite::shared`).
    pub fn shared_citations<'a>(&self, statements: &'a [StatementRef]) -> Option<&'a Vec<String>> {
        super::cite::shared(statements)
    }

    /// The values of a list; a value whose citations are the `shared`
    /// ones without them: the list shows them once.
    pub fn values_beside<'a>(
        &'a self,
        statements: &'a [StatementRef],
        shared: &Option<&Vec<String>>,
    ) -> Vec<Shown<'a>> {
        let mut values = self.values(statements);
        for v in &mut values {
            if *shared == Some(v.citations) {
                v.citations = &NO_CITATIONS;
            }
        }
        values
    }

    /// No citations, for a mark of provenance without a statement.
    pub fn no_citations(&self) -> &'static Vec<String> {
        &NO_CITATIONS
    }

    pub fn mark(&self, provenance: impl Arg<Option<Provenance>>) -> &'static str {
        match provenance.get() {
            Some(Provenance::Imported) => "imported",
            Some(Provenance::None) => "none",
            _ => "",
        }
    }

    /// No property name above a figure (a set of one property).
    pub fn no_head(&self) -> Option<Label> {
        None
    }

    pub fn wikipedia_logo(&self) -> &'static str {
        WIKIPEDIA_LOGO
    }

    /// The heading of an article section of a level: the spread title is
    /// `h2`, so a section (level 2) is `h3`, its subsection `h4`.
    pub fn section_tag(&self, level: impl Arg<u8>) -> &'static str {
        match level.get() {
            0..=2 => "h3",
            3 => "h4",
            4 => "h5",
            _ => "h6",
        }
    }

    /// The language of the texts that the reader can translate into the
    /// page language: the fallback article's, when the page shows its lead
    /// or its sections. None: the page has no translator.
    pub fn translation_source(&self) -> Option<&'v str> {
        let view = self.view;
        let lead = view.lead.as_ref().filter(|a| !a.local).map(|a| &a.lang);
        let section = view
            .spreads
            .iter()
            .flat_map(|s| &s.sections)
            .chain(&view.more_sections)
            .find(|s| !s.local)
            .map(|s| &s.lang);
        lead.or(section).map(|l| l.as_str())
    }

    /// The icon of the translation action.
    pub fn language_icon(&self) -> &'static str {
        &super::design::LANGUAGE_ICON
    }

    /// "Translate to ગુજરાતી": the page language named by `intl.js`.
    pub fn translate_label(&self) -> String {
        let lang = escape(self.view.lang.as_str());
        let name = format!(r#"<span data-lang-name="{lang}">{lang}</span>"#);
        crate::localize::messages::format(&escape(self.msg("qrender-translate")), &[&name])
    }

    /// The note at the end of the page, with a link to the item.
    pub fn footer(&self) -> String {
        let qid = self.view.qid;
        let link = format!(r#"<a href="https://www.wikidata.org/wiki/{qid}">{qid}</a>"#);
        let versions = format!(
            "{}, {}, {}",
            self.view.composition, self.view.schema, self.view.design
        );
        let template = escape(self.msg("qrender-footer"));
        crate::localize::messages::format(&template, &[&link, &escape(&versions)])
    }
}

/// Text for HTML.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn quote(s: &str) -> String {
    utf8_percent_encode(s, PATH).to_string()
}

/// A row of the web spread: the links of one property.
pub struct LinkRow<'a> {
    pub pid: Pid,
    /// The icon of the one site of all the links of the row.
    pub icon: Option<String>,
    /// Each link, with its own icon when the row has none.
    pub cards: Vec<(&'a Card, Option<String>)>,
}

/// The host of an address: "https://www.x.com/usagov" gives "www.x.com".
fn domain(href: &str) -> Option<&str> {
    let rest = href.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = host.split(':').next()?;
    (!host.is_empty()).then_some(host)
}

/// The icon of a site (128 px; it shows at 16 px).
pub(crate) fn favicon(domain: &str) -> String {
    format!("{FAVICONS}?url={}", utf8_percent_encode(domain, PATH))
}
