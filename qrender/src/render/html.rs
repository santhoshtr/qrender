//! The HTML renderer: a localized view to a page. Templates only
//! (`qrender/templates/`): every string was resolved by localize, every
//! decision made by the composer, and the design system does the geometry.
//!
//! Each shared part and each primitive is a template of its own, rendered
//! by a `Ui` method. A template calls a part, it does not inline it, so the
//! generated code grows with the number of templates, not with their
//! nesting.

use askama::Template;

use super::cite::{self, Card, Value as CiteValue};
use super::design::{REFERENCE_ICON, STYLESHEET, scripts};
use super::map::Frame;
use super::ui::{Arg, Ref, Shown, TopicArticle, Ui};
use crate::compose::picture::media_kind;
use crate::localize::format::Part;
use crate::localize::messages::format;
use crate::localize::{
    Article, Label, Note, ProseBlock, ProseFigure, ProseImage, ProseSection, SpreadView, View,
};
use crate::primitives::fact::Fact;
use crate::primitives::map::{Map, MapShape};
use crate::primitives::media::{Media, MediaRole};
use crate::primitives::{
    absent, credits, earlier, fact, identifiers, kind, links, map, media, names, network, peers,
    pictures, prose, provenance, scale, score, series, sitelinks, sounds, split, succession,
    timeline, title, works,
};
use crate::schema::{Block, Credit, StatementRef};
use crate::vocab::{self, Slot};

/// How the page gets its stylesheet.
pub enum Styles<'a> {
    /// In the page: one file to open.
    Inline,
    /// Linked from a base URL (the server's versioned design route).
    Linked(&'a str),
}

/// `service`: the base path of the server that serves the page. The page
/// links its Markdown and JSON versions there.
pub fn page(view: &View, styles: Styles, service: Option<&str>) -> String {
    let ui = Ui::new(view);
    let shown: Vec<_> = view
        .spreads
        .iter()
        .flat_map(|s| s.spread.slots.values().flatten())
        .map(Block::primitive)
        .collect();
    render(Page {
        ui: &ui,
        view,
        styles,
        service,
        stylesheet: &STYLESHEET,
        scripts: scripts(&shown),
    })
}

fn render(t: impl Template) -> String {
    t.render().expect("templates render to a string")
}

#[derive(Template)]
#[template(path = "page.html")]
struct Page<'a> {
    ui: &'a Ui<'a>,
    view: &'a View,
    styles: Styles<'a>,
    service: Option<&'a str>,
    stylesheet: &'static str,
    scripts: String,
}

#[derive(Template)]
#[template(path = "spread.html")]
struct SpreadHtml<'a> {
    ui: &'a Ui<'a>,
    s: &'a SpreadView,
}

#[derive(Template)]
#[template(path = "slot.html")]
struct SlotHtml<'a> {
    ui: &'a Ui<'a>,
    slot: Slot,
    blocks: &'a [Block],
}

#[derive(Template)]
#[template(path = "parts/text.html")]
struct TextHtml<'a> {
    ui: &'a Ui<'a>,
    l: &'a Label,
    add: bool,
}

#[derive(Template)]
#[template(path = "parts/part.html")]
struct PartHtml<'a> {
    ui: &'a Ui<'a>,
    p: &'a Part,
}

#[derive(Template)]
#[template(path = "parts/note.html")]
struct NoteHtml<'a> {
    ui: &'a Ui<'a>,
    parts: &'a [Part],
}

#[derive(Template)]
#[template(path = "parts/line.html")]
struct LineHtml<'a> {
    ui: &'a Ui<'a>,
    s: &'a Shown<'a>,
}

#[derive(Template)]
#[template(path = "parts/row_lines.html")]
struct RowLinesHtml<'a> {
    ui: &'a Ui<'a>,
    /// The values that the composer marked as shown.
    values: &'a [Shown<'a>],
    /// The values behind "+N".
    rest: usize,
    /// The values of the dialog: all of them, and more when the block has
    /// more (the holder now, for the holders before).
    all: &'a [Shown<'a>],
    heading: &'a str,
}

#[derive(Template)]
#[template(path = "parts/lines.html")]
struct LinesHtml<'a> {
    ui: &'a Ui<'a>,
    values: &'a [Shown<'a>],
    heading: &'a str,
}

#[derive(Template)]
#[template(path = "parts/rows.html")]
struct RowsHtml<'a> {
    ui: &'a Ui<'a>,
    values: &'a [Shown<'a>],
}

#[derive(Template)]
#[template(path = "parts/figure.html")]
struct FigureHtml<'a> {
    ui: &'a Ui<'a>,
    s: &'a Shown<'a>,
    head: Option<&'a Label>,
}

#[derive(Template)]
#[template(path = "parts/about.html")]
struct AboutHtml<'a> {
    ui: &'a Ui<'a>,
    l: &'a Label,
}

#[derive(Template)]
#[template(path = "parts/toc.html")]
struct TocHtml<'a> {
    ui: &'a Ui<'a>,
    entries: Vec<(&'a str, &'a Label, &'static str)>,
}

#[derive(Template)]
#[template(path = "parts/credit.html")]
struct CreditHtml<'a> {
    ui: &'a Ui<'a>,
    c: &'a Credit,
}

#[derive(Template)]
#[template(path = "parts/map_frame.html")]
struct MapFrameHtml<'a> {
    ui: &'a Ui<'a>,
    f: &'a Frame<'a>,
    lazy: bool,
}

#[derive(Template)]
#[template(path = "parts/map_shapes.html")]
struct MapShapesHtml<'a> {
    ui: &'a Ui<'a>,
    shapes: &'a [MapShape],
}

#[derive(Template)]
#[template(path = "parts/map_dialog.html")]
struct MapDialogHtml<'a> {
    ui: &'a Ui<'a>,
    n: u32,
    heading: &'a str,
    f: &'a Frame<'a>,
    /// The JSON for map.js (render/map.rs `live`).
    data: &'a str,
    /// The map of the item; None for the map of a coordinate pin.
    b: Option<&'a Map>,
}

#[derive(Template)]
#[template(path = "parts/article_section.html")]
struct ArticleSectionHtml<'a> {
    ui: &'a Ui<'a>,
    s: &'a ProseSection,
    /// The tag of the title.
    h: &'static str,
}

#[derive(Template)]
#[template(path = "parts/topic_article.html")]
struct TopicArticleHtml<'a> {
    ui: &'a Ui<'a>,
    t: &'a TopicArticle<'a>,
}

#[derive(Template)]
#[template(path = "parts/translate_button.html")]
struct TranslateButtonHtml<'a> {
    ui: &'a Ui<'a>,
}

#[derive(Template)]
#[template(path = "parts/label_editor.html")]
struct LabelEditorHtml<'a> {
    ui: &'a Ui<'a>,
}

#[derive(Template)]
#[template(path = "parts/translator.html")]
struct TranslatorHtml<'a> {
    ui: &'a Ui<'a>,
    /// The language of the source text.
    from: &'a str,
}

#[derive(Template)]
#[template(path = "parts/article_subsection.html")]
struct ArticleSubsectionHtml<'a> {
    ui: &'a Ui<'a>,
    s: &'a ProseSection,
}

#[derive(Template)]
#[template(path = "parts/article_blocks.html")]
struct ArticleBlocksHtml<'a> {
    ui: &'a Ui<'a>,
    blocks: &'a [ProseBlock],
}

#[derive(Template)]
#[template(path = "parts/article_figure.html")]
struct ArticleFigureHtml<'a> {
    ui: &'a Ui<'a>,
    f: &'a ProseFigure,
}

#[derive(Template)]
#[template(path = "parts/lead_picture.html")]
struct LeadPictureHtml<'a> {
    ui: &'a Ui<'a>,
    a: &'a Article,
    f: &'a ProseFigure,
    /// The one image of the figure.
    i: &'a ProseImage,
    /// The kind of the file: "drawing" or "photo".
    kind: &'static str,
}

#[derive(Template)]
#[template(path = "parts/cite.html")]
struct CiteHtml<'a> {
    ui: &'a Ui<'a>,
    c: &'a Card<'a>,
}

#[derive(Template)]
#[template(path = "parts/cite_mark.html")]
struct CiteMarkHtml<'a> {
    ui: &'a Ui<'a>,
    /// The id of the popover.
    n: u32,
    /// One at least.
    cards: Vec<Card<'a>>,
}

#[derive(Template)]
#[template(path = "parts/note_mark.html")]
struct NoteMarkHtml<'a> {
    ui: &'a Ui<'a>,
    number: usize,
}

#[derive(Template)]
#[template(path = "parts/note_body.html")]
struct NoteBodyHtml<'a> {
    ui: &'a Ui<'a>,
    note: &'a Note,
    cards: Vec<Card<'a>>,
}

#[derive(Template)]
#[template(path = "parts/notes.html")]
struct NotesHtml<'a> {
    ui: &'a Ui<'a>,
    /// Each note, and its body as HTML: in its popover and in the list.
    notes: Vec<(&'a Note, String)>,
}

#[derive(Template)]
#[template(path = "parts/lightbox.html")]
struct LightboxHtml<'a> {
    ui: &'a Ui<'a>,
    b: &'a Media,
    /// The id of the dialog, which the figure opens.
    n: u32,
}

/// The parts, for the templates.
impl<'v> Ui<'v> {
    pub fn spread(&self, s: &SpreadView) -> String {
        render(SpreadHtml { ui: self, s })
    }

    /// The contents of the page: the spreads that have a title. Empty
    /// when none has.
    pub fn toc(&self) -> String {
        let entries: Vec<_> = self
            .view
            .spreads
            .iter()
            .filter(|s| !matches!(s.spread.template, vocab::Template::Identity))
            .filter_map(|s| {
                Some((
                    s.spread.id.as_str(),
                    s.title.as_ref()?,
                    s.spread.dimension.name(),
                ))
            })
            .collect();
        if entries.is_empty() {
            return String::new();
        }
        render(TocHtml { ui: self, entries })
    }

    /// The contents go after this slot of the head: after the banner, or
    /// after the title when there is no banner.
    pub fn toc_after(&self, s: &SpreadView, slot: impl Arg<Slot>) -> bool {
        let slots = &s.spread.slots;
        matches!(s.spread.template, vocab::Template::Identity)
            && match slot.get() {
                Slot::Band => true,
                Slot::Head => !slots.contains_key(&Slot::Band),
                _ => false,
            }
    }

    pub fn slot(&self, slot: impl Arg<Slot>, blocks: &[Block]) -> String {
        render(SlotHtml {
            ui: self,
            slot: slot.get(),
            blocks,
        })
    }

    /// The description of a property behind an (i) button; nothing when
    /// it has none.
    pub fn about(&self, l: impl Ref<Label>) -> String {
        render(AboutHtml { ui: self, l: l.r() })
    }

    /// A label; with `add`, a fallback has the action to add it.
    pub fn text(&self, l: impl Ref<Label>, add: bool) -> String {
        render(TextHtml {
            ui: self,
            l: l.r(),
            add,
        })
    }

    pub fn part(&self, p: impl Ref<Part>) -> String {
        render(PartHtml { ui: self, p: p.r() })
    }

    /// Parts joined by " · ".
    pub fn note(&self, parts: &[Part]) -> String {
        render(NoteHtml { ui: self, parts })
    }

    /// The values of one property, joined by ", ".
    pub fn values_of(&self, parts: &[Part]) -> String {
        parts
            .iter()
            .map(|p| self.part(p))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn line<'a>(&self, s: impl Ref<Shown<'a>>) -> String {
        render(LineHtml { ui: self, s: s.r() })
    }

    pub fn lines(&self, values: &[Shown], heading: impl Ref<str>) -> String {
        render(LinesHtml {
            ui: self,
            values,
            heading: heading.r(),
        })
    }

    /// Values as rows with their pictures, as in a dialog: the ones that
    /// the composer marked as shown; all of them (`all`) in a dialog.
    pub fn rows_with(
        &self,
        statements: &[StatementRef],
        all: &[Shown],
        heading: impl Ref<str>,
    ) -> String {
        let values: Vec<Shown> = statements
            .iter()
            .filter(|s| s.shown != Some(false))
            .map(|s| self.value(s))
            .collect();
        render(RowLinesHtml {
            ui: self,
            values: &values,
            rest: self.hidden_refs(statements),
            all,
            heading: heading.r(),
        })
    }

    /// All the values of a pictured list, as rows: its dialog.
    pub fn rows(&self, values: &[Shown]) -> String {
        render(RowsHtml { ui: self, values })
    }

    pub fn figure<'a>(&self, s: impl Ref<Shown<'a>>) -> String {
        render(FigureHtml {
            ui: self,
            s: s.r(),
            head: None,
        })
    }

    /// A figure with the name of its property above the value.
    pub fn figure_with<'a>(&self, s: impl Ref<Shown<'a>>, head: impl Ref<Label>) -> String {
        render(FigureHtml {
            ui: self,
            s: s.r(),
            head: Some(head.r()),
        })
    }

    pub fn credit(&self, c: impl Ref<Credit>) -> String {
        render(CreditHtml { ui: self, c: c.r() })
    }

    /// The dialog of a file of the item: the file large, its words under
    /// it.
    pub fn lightbox(&self, b: &Media, n: impl Arg<u32>) -> String {
        render(LightboxHtml {
            ui: self,
            b,
            n: n.get(),
        })
    }

    /// A topic of a spread as a card: its Wikipedia article.
    pub fn topic_article(&self, t: &TopicArticle) -> String {
        render(TopicArticleHtml { ui: self, t })
    }

    /// A section of the article; with `spread`, a spread of its own, with
    /// the title of a spread.
    pub fn article_section(&self, s: &ProseSection, spread: bool) -> String {
        render(ArticleSectionHtml {
            ui: self,
            s,
            h: if spread {
                "h2"
            } else {
                self.section_tag(s.level)
            },
        })
    }

    /// Article HTML with the mark of each note (`<sup data-note="3"></sup>`,
    /// as the localize step writes it) as a button that opens the note.
    pub fn prose(&self, html: impl Ref<str>) -> String {
        const MARK: &str = "<sup data-note=\"";
        const END: &str = "\"></sup>";
        let mut out = String::with_capacity(html.r().len());
        let mut rest = html.r();
        while let Some(start) = rest.find(MARK) {
            out.push_str(&rest[..start]);
            let after = &rest[start + MARK.len()..];
            let Some(end) = after.find(END) else {
                out.push_str(&rest[start..]);
                return out;
            };
            if let Ok(number) = after[..end].parse() {
                out.push_str(&render(NoteMarkHtml { ui: self, number }));
            }
            rest = &after[end + END.len()..];
        }
        out.push_str(rest);
        out
    }

    /// "Note 3": the name of a note mark for a screen reader.
    pub fn note_label(&self, number: impl Arg<usize>) -> String {
        format(self.msg("qrender-note"), &[&number.get().to_string()])
    }

    /// The sources of a statement: a button and a popover with a card for
    /// each. Nothing without one.
    pub fn sources(&self, citations: &[String]) -> String {
        let cards: Vec<Card> = citations
            .iter()
            .filter_map(|hash| self.view.citations.get(hash))
            .map(|c| cite::card(c, self.view))
            .collect();
        if cards.is_empty() {
            return String::new();
        }
        render(CiteMarkHtml {
            ui: self,
            n: self.id(),
            cards,
        })
    }

    /// The notes of the article text: their popovers and their list.
    /// Nothing without notes.
    pub fn notes(&self) -> String {
        if self.view.notes.is_empty() {
            return String::new();
        }
        let notes = self
            .view
            .notes
            .iter()
            .map(|note| {
                let cards = note
                    .citations
                    .iter()
                    .map(|c| cite::card(c, self.view))
                    .collect();
                (
                    note,
                    render(NoteBodyHtml {
                        ui: self,
                        note,
                        cards,
                    }),
                )
            })
            .collect();
        render(NotesHtml { ui: self, notes })
    }

    pub fn cite(&self, c: &Card) -> String {
        render(CiteHtml { ui: self, c })
    }

    /// The icon of a source without a site icon.
    pub fn reference_icon(&self) -> &'static str {
        &REFERENCE_ICON
    }

    /// The action to translate a text of the article (translate.js).
    pub fn translate_button(&self) -> String {
        render(TranslateButtonHtml { ui: self })
    }

    /// The dialog of the page that edits a label in the page language.
    pub fn label_editor(&self) -> String {
        render(LabelEditorHtml { ui: self })
    }

    /// The dialog of the page that edits a translation from `from`.
    pub fn translator(&self, from: &str) -> String {
        render(TranslatorHtml { ui: self, from })
    }

    pub fn article_subsection(&self, s: &ProseSection) -> String {
        render(ArticleSubsectionHtml { ui: self, s })
    }

    /// The blocks of an article section, in order: texts and figures.
    pub fn article_blocks(&self, blocks: &[ProseBlock]) -> String {
        render(ArticleBlocksHtml { ui: self, blocks })
    }

    /// The images of a figure of the article, at the width of the text.
    pub fn article_figure(&self, f: &ProseFigure) -> String {
        render(ArticleFigureHtml { ui: self, f })
    }

    /// The static map of a frame; `lazy` in a dialog.
    pub fn map_frame(&self, f: &Frame, lazy: bool) -> String {
        render(MapFrameHtml { ui: self, f, lazy })
    }

    pub fn map_shapes(&self, shapes: &[MapShape]) -> String {
        render(MapShapesHtml { ui: self, shapes })
    }

    /// The dialog of a map: the map that the reader moves and zooms.
    pub fn map_dialog(
        &self,
        n: impl Arg<u32>,
        heading: impl Ref<str>,
        f: &Frame,
        data: impl Ref<str>,
        b: Option<&Map>,
    ) -> String {
        render(MapDialogHtml {
            ui: self,
            n: n.get(),
            heading: heading.r(),
            f,
            data: data.r(),
            b,
        })
    }

    /// A file of the item. The head picture gives its place to the picture
    /// of the lead of the article, when the lead has one.
    pub fn media(&self, b: &Media) -> String {
        let lead = self.view.lead.as_ref();
        if b.role == MediaRole::Identity
            && let Some((a, f)) = lead.and_then(|a| Some((a, a.picture.as_ref()?)))
            && let Some(i) = f.images.first()
        {
            return render(LeadPictureHtml {
                ui: self,
                a,
                f,
                i,
                kind: media_kind(&i.file).name(),
            });
        }
        render(media::Html { ui: self, b })
    }

    pub fn row(&self, b: &Fact) -> String {
        render(fact::Row { ui: self, b })
    }

    /// A block in a sheet slot: a fact is a row there.
    pub fn sheet_block(&self, b: &Block) -> String {
        match b {
            Block::Fact(f) => self.row(f),
            other => self.block(other),
        }
    }

    /// One block: the template of its primitive.
    pub fn block(&self, b: &Block) -> String {
        match b {
            Block::Title(b) => render(title::Html { ui: self, b }),
            Block::Kind(b) => render(kind::Html { ui: self, b }),
            Block::Prose(b) => render(prose::Html { ui: self, b }),
            Block::Media(b) => self.media(b),
            Block::Pictures(b) => render(pictures::Html { ui: self, b }),
            Block::Sounds(b) => render(sounds::Html { ui: self, b }),
            Block::Earlier(b) => render(earlier::Html { ui: self, b }),
            Block::Names(b) => render(names::Html { ui: self, b }),
            Block::Map(b) => render(map::Html { ui: self, b }),
            Block::Fact(b) => render(fact::Html { ui: self, b }),
            Block::Series(b) => render(series::Html { ui: self, b }),
            Block::Split(b) => render(split::Html { ui: self, b }),
            Block::Scale(b) => render(scale::Html { ui: self, b }),
            Block::Peers(b) => render(peers::Html { ui: self, b }),
            Block::Network(b) => render(network::Html { ui: self, b }),
            Block::Gallery(b) => render(network::GalleryHtml { ui: self, b }),
            Block::Succession(b) => render(succession::Html { ui: self, b }),
            Block::Credits(b) => render(credits::Html { ui: self, b }),
            Block::Score(b) => render(score::Html { ui: self, b }),
            Block::Works(b) => render(works::Html { ui: self, b }),
            Block::Timeline(b) => render(timeline::Html { ui: self, b }),
            Block::Links(b) => render(links::Html { ui: self, b }),
            Block::Absent(b) => render(absent::Html { ui: self, b }),
            Block::Provenance(b) => render(provenance::Html { ui: self, b }),
            Block::Sitelinks(b) => render(sitelinks::Html { ui: self, b }),
            Block::Identifiers(b) => render(identifiers::Html { ui: self, b }),
        }
    }
}
