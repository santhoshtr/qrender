//! The HTML renderer: a localized view to a page. Templates only
//! (`qrender/templates/`): every string was resolved by localize, every
//! decision made by the composer, and the design system does the geometry.
//!
//! Each shared part and each primitive is a template of its own, rendered
//! by a `Ui` method. A template calls a part, it does not inline it, so the
//! generated code grows with the number of templates, not with their
//! nesting.

use askama::Template;

use super::design::{STYLESHEET, TOKENS, scripts};
use super::ui::{Arg, Ref, Shown, Ui};
use crate::localize::format::Part;
use crate::localize::{Label, SpreadView, View};
use crate::primitives::fact::Fact;
use crate::primitives::media::Media;
use crate::primitives::{
    absent, credits, fact, identifiers, kind, map, media, names, network, peers, pictures, prose,
    provenance, scale, score, series, sitelinks, sounds, split, succession, time, title, works,
};
use crate::schema::{Block, Credit};
use crate::vocab::Slot;

/// How the page gets its stylesheets.
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
        tokens: TOKENS,
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
    tokens: &'static str,
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
    heading: &'a str,
}

#[derive(Template)]
#[template(path = "parts/figure.html")]
struct FigureHtml<'a> {
    ui: &'a Ui<'a>,
    s: &'a Shown<'a>,
    head: Option<&'a Label>,
}

#[derive(Template)]
#[template(path = "parts/credit.html")]
struct CreditHtml<'a> {
    ui: &'a Ui<'a>,
    c: &'a Credit,
}

/// The parts, for the templates.
impl<'v> Ui<'v> {
    pub fn spread(&self, s: &SpreadView) -> String {
        render(SpreadHtml { ui: self, s })
    }

    pub fn slot(&self, slot: impl Arg<Slot>, blocks: &[Block]) -> String {
        render(SlotHtml {
            ui: self,
            slot: slot.get(),
            blocks,
        })
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
            Block::Time(b) => render(time::Html { ui: self, b }),
            Block::Absent(b) => render(absent::Html { ui: self, b }),
            Block::Provenance(b) => render(provenance::Html { ui: self, b }),
            Block::Sitelinks(b) => render(sitelinks::Html { ui: self, b }),
            Block::Identifiers(b) => render(identifiers::Html { ui: self, b }),
        }
    }
}
