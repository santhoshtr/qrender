//! The HTML renderer: a localized view to a page. Templates only
//! (`qrender/templates/`): every string was resolved by localize, every
//! decision made by the composer, and the design system does the geometry.
//!
//! Each shared part and each primitive is a template of its own, rendered
//! by a `Ui` method. A template calls a part, it does not inline it, so the
//! generated code grows with the number of templates, not with their
//! nesting.

use askama::Template;

use super::design::{STYLESHEET, scripts};
use super::map::Frame;
use super::ui::{Arg, Ref, Shown, Ui};
use crate::localize::format::Part;
use crate::localize::{Label, ProseSection, SpreadView, View};
use crate::primitives::fact::Fact;
use crate::primitives::map::{Map, MapShape};
use crate::primitives::media::Media;
use crate::primitives::{
    absent, credits, earlier, fact, identifiers, kind, links, map, media, names, network, peers,
    pictures, prose, provenance, scale, score, series, sitelinks, sounds, split, succession,
    timeline, title, works,
};
use crate::schema::{Block, Credit};
use crate::vocab::{self, Slot};

/// How the page gets its stylesheet.
pub enum Styles<'a> {
    /// In the page: one file to open.
    Inline,
    /// Linked from a base URL (the server's versioned design route).
    Linked(&'a str),
}

pub fn page(view: &View, styles: Styles) -> String {
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
#[template(path = "parts/lines.html")]
struct LinesHtml<'a> {
    ui: &'a Ui<'a>,
    values: &'a [Shown<'a>],
    /// The values of the dialog: all of them, and more when the block has
    /// more (the holder now, for the holders before).
    all: &'a [Shown<'a>],
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
    /// At the end of the page: the whole section collapsed to its title.
    collapsed: bool,
}

#[derive(Template)]
#[template(path = "parts/article_subsection.html")]
struct ArticleSubsectionHtml<'a> {
    ui: &'a Ui<'a>,
    s: &'a ProseSection,
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
        self.lines_with(values, values, heading)
    }

    /// Lines of `values`, with `all` in the dialog.
    pub fn lines_with(&self, values: &[Shown], all: &[Shown], heading: impl Ref<str>) -> String {
        render(LinesHtml {
            ui: self,
            values,
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

    /// A section of the fallback article; `collapsed` at the end of the
    /// page.
    pub fn article_section(&self, s: &ProseSection, collapsed: bool) -> String {
        render(ArticleSectionHtml {
            ui: self,
            s,
            collapsed,
        })
    }

    pub fn article_subsection(&self, s: &ProseSection) -> String {
        render(ArticleSubsectionHtml { ui: self, s })
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

    pub fn media(&self, b: &Media) -> String {
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
