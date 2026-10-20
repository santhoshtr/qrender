# References: typed citations for prose and facts

Status: done (2026-10-19/20): steps 1–6 (7ca51d3; 8b7f51a, 2ef417c;
27ac2f8; 58834b4; d28d65c; e62e52c; da7ad59), step 8, the United
States (§10), and a review of the cards (92984f9, §6). Step 7, the editor, is postponed (2026-10-20): its plan
and a handoff are in `.plans/future/citation-editor.md`.
Decisions in §9.

Notes on step 1 (the types differ from §3 in these points):
- The fields of every work (title, url, authors, date, accessed,
  archive, language, ids) are one struct `Work`, flattened into `Web`,
  `News`, `Journal`, `Book`. The JSON stays flat.
- `Archive.original` is `live` / `dead` / `unfit` (CS1 `url-status`;
  `usurped` is `unfit`: no link to the original).
- `Origin` is `template` / `coins` / `html` / `wikidata { hash }`. No
  snaks: an edit reads the live entity.
- `WebUrl` is a validated newtype: http and https only, `//` becomes
  `https:`; `host()` gives the favicon host.
- The JSON Schema generates (checked); it is checked in at step 5.

Notes on step 2:
- The replay test has a snapshot of the article prose after the
  localize step (`replay__q3870.{en,gu}.prose.json`). It is the
  invariant of a change of how the fetcher reads an article.
- Q3870 was recorded twice, with the same article and item revisions:
  first with the old fetcher (only drift: the order of identifiers,
  which follows the rows of the query service), then with the new one.
- The diff of the new fetcher: the hidden forms of a coordinate
  (`geo-nondefault`) are gone from the Geography section text; `"`
  becomes `&quot;` and `&#32;` a space (the same when shown).
- 50 recorded responses became 39. `Section.index` is gone.
- `wikipedia::read` is pure: it gives the lead, the outline and the
  first paragraph of each section from the Parsoid HTML. Step 3 adds
  the notes to it.

Notes on step 3:
- `qrender/citations.toml` is a third embedded file. Its types
  (`CitationStyle`, `ByKind`, `Parameters`) are in `content::settings`,
  closed with `deny_unknown_fields`; the fetcher gets them in
  `Settings.citations`, so the fingerprint follows them. One style:
  CS1, with `templates` (name → kind) and `classes` (the `<cite>`
  class → kind, for a COinS-only citation).
- `fetch/notes.rs`: the parse. In a note, each transclusion group (the
  nodes of one `about`): a template of a style gives a citation; else
  its COinS. A loose `span.Z3988` gives a citation. No citation: the
  note is one `Text`. Text with words beside citations is a `Text` of
  its own (none in Nairobi). An explanatory note (`data-mw-group`) keeps
  only its text.
- `{{webarchive}}` and `{{dead link}}` are not read: in Nairobi they
  follow free text, which keeps them as text. Read them when Q30 needs
  it.
- No archive or access date from COinS (it has neither).
- CS1 rule: an archive URL without `url-status` means the original is
  dead.
- `Date::parse`: ISO and English month names; else text. All Nairobi
  dates parse.
- `plain()`: wikitext to text (comments, templates, links, quotes,
  tags, entities). `{{!}}` is "|"; another template is removed.
- The mark in stored HTML: `<sup data-note="cite_note-12"></sup>`.
  Localize still removes it (no class `reference`), so the page does
  not change in this step.
- Tests: `fetch/notes.rs` (each path on real pieces in
  `fixtures/notes/`, hostile input, `plain`, languages);
  `app/tests/notes.rs` (all 186 notes of the recorded article, the
  invariants, snapshots of the notes and of the report); the replay test
  (every mark has its note, every note is marked).
- Report for Nairobi: template 154, COinS 13, text 18, explanatory 1;
  templates in no style: webarchive 5, dead link 3, cbignore 1.
- `content::fetch::article_notes(html, styles)` is public: the corpus
  test and the Q30 report use it.

Notes on step 4 (it differs from §4.4):
- The change is additive. `Content.references` (the property sets)
  stays, and the composer reads it for `Provenance` and
  `Order::References` as before. New: `statement_citations`
  (statement → hashes) and `citations` (hash → `Cited`), empty by
  default. Reason: the eight content fixtures have the old shape; a new
  fetch of all of them would change every snapshot by data drift.
- `sparql::references` gives each reference's values (`Snak`: datatype,
  language, time precision, the formatter URL of an identifier).
  `fetch/references.rs` maps them: P248 → `Record` (its external
  identifier, the URL from P854 or from the formatter), else P854 →
  `Web` (P1476 title and its language, P577, P813, P2093, archive
  P1065/P2960), else no citation (P143, P813 alone, P4656).
- The items of `Record.stated_in` are in the label set of the text.
- Recording: only the new requests were added (the reference query,
  the label batches that the new items shift); the other recorded
  responses stayed, so no drift.
- Q3870: 24 citations (19 records, 5 web pages); snapshot
  `replay__q3870.citations.json`.
- For step 5: `fixtures/content/q3870.json` has no citations (fetched
  before this step). The view and page tests of step 5 need Nairobi
  with citations: fetch that fixture again (and its text fixtures, for
  the notes), and review the drift on its own, before the step 5
  change.
- Done (d28d65c): the Q3870 content and text fixtures come from the
  recorded responses (`qrender fetch --replay fixtures/http/q3870`), so
  the page snapshots equal the replay snapshots. Text: 19 notes for en,
  169 for gu. The old fixture's lead had a "Cite error" paragraph; it is
  gone.

Notes on step 5:
- `StatementRef.citations` (hashes; the statement and its joined
  texts), and `citations` beside every other `provenance` field: map
  marker, scale measure, score value, split share, account followers,
  series point and file series. `Composer::citations(s)`.
- The identifiers block cites none (as the provenance primitive does
  not count identifiers). So `Composition.citations` is only the
  citations that a block cites: the hashes under `citations` keys.
  Q3870: 7 of 24. A test checks the table for every fixture.
- The mark in the view is neutral: `<sup data-note="3"></sup>`, empty.
  Each renderer makes its form in step 6 (HTML: the popover button;
  Markdown: `[^3]`). Until then the pages are the same (checked: the
  snapshots differ only by the empty marks).
- `article::Numbering`: a note gets its number at its first mark; the
  lead, the spreads and the more sections are built before the notes,
  so the numbers follow reading order. A note that an explanatory note
  marks comes after. A mark whose note is missing goes.
- `Links.external`: in the HTML of a free-text citation, `http` and
  `https` links stay (`&` escaped); prose still drops them.
- `View.notes` (number, group, lang, citations, text) and
  `View.citations`. `"stated_in"` is a labelled key.
- The messages of the field names move to step 6, where the templates
  use them.
- en: 16 notes on the page; gu: 169 (177 marks).

Notes on step 6:
- `render/cite.rs` (the card, shared), templates `parts/cite.html`,
  `cite_mark.html`, `note_mark.html`, `note_body.html`, `notes.html`;
  `Ui::prose`, `sources`, `notes`, `cite`; `m::prov(ui, kind,
  citations)` at its 17 places; `Shown.citations`.
- The Codex "reference" icon (`assets/icons/reference.svg`, a copy from
  the Codex repository), inline by the same loader as "language".
- Messages: `qrender-sources`, `qrender-note`, `qrender-cite-*` (in
  `en.json` and `qqq.json`).
- CSS in `base.css`: `.src`, `.ref`, `.pop.cites`, `.cite` (`.head`,
  `.host`, `.field`, `.field-name`, `.field-value`), `.sources`. The
  card's field classes are not `.k`/`.v`: `.v` already has a weight.
  The "Sources" `<details>` spans the subgrid of the spread.
- Checked in Chrome (Nairobi en, gu): a note popover by its mark; a
  fact popover in a picture's caption; the "Sources" list; a mark in a
  collapsed section of the fallback article (popover `lang="en"`).
- Size: the markup of q3870.gu grows from 258 KB to 623 KB (the note
  cards are in the popovers and in the list); gzip 58 KB to 101 KB.
  q3870.en: 125 KB to 161 KB; gzip 23.6 KB to 27.1 KB. To halve the
  growth, drop the list; a decision for later.
- The other fixtures' snapshots change only by the script
  (`translate.js`) and the new messages in the view.

Scope: the first version is for Nairobi (Q3870). Then the United States
(Q30) shows more kinds of citations (§2.4); the model must not block
them, but it does not implement them before step 8.

## 1. The problem

The page shows no references. The fetcher removes the reference marks
from the article prose (`fetch/wikipedia.rs`: `REFERENCE`,
`write_element`), and the localize step removes them again
(`localize/article.rs`). For a statement, the content keeps only the
property set of each reference (`Content.references:
IndexMap<StatementId, Vec<Vec<Pid>>>`). The composer uses it for
`Provenance` and for `Order::References`. The values of a reference are
not fetched.

A reader must see where a text or a fact comes from. A reader must also
be able to correct a reference. So:

- references are data in the backend, as strict types (an enum of
  kinds), not as HTML;
- the page shows a reference as a card: the favicon (or an icon) and
  the title as the link, and below, the other fields as labelled rows;
- the same card for a reference from the article and for a reference
  from Wikidata;
- the typed data is also the model of the editor.

## 2. What Nairobi has (measured, 2026-10-19)

### 2.1 The English article

The Parsoid HTML of the whole article is 884 KB. It has 186 notes
(`li#cite_note-*` in `ol.mw-references`), in two groups: the default
group (numbers) and `lower-alpha` (1 explanatory note, with a
reference in it). (Corrected in step 3: an early count of "4" counted
attributes, not notes.)

| Source of the data in a note | Notes | Example |
|---|---|---|
| A citation template, with its parameters in `data-mw` | 154 | `{{cite web}}` 135, `{{cite news}}` 10, `{{cite journal}}` 6, `{{cite book}}` 3 |
| Only COinS (`span.Z3988`, OpenURL key-value text) | 13 | a reference that the infobox template made |
| Free text, no template | 19 | "Sana Aiyar, Indians in Kenya…, Harvard University Press, 2015, p.42" |

Findings:

- Parsoid puts the template data on the **first node** of the
  transclusion. For CS1 that node is the TemplateStyles `<style>` or
  `<link rel="mw-deduplicated-inline-style">`, not the `<cite>`. The
  `<cite>` has the same `about="#mwt…"`. The prototype's
  `[about=…][typeof~=mw:Transclusion]` lookup is correct; a lookup on
  `cite[data-mw]` is not.
- A reference that a template makes inside another template (the
  infobox) has no `data-mw` of its own. COinS is then the only
  structured data.
- The parameters are **wikitext** (`{"wt": "[[Daily Nation]]"}`), not
  text. Some contain links, italics or templates.
- The parameters used: `cite web`: title, url (135), access-date (132),
  archive-url/archive-date/url-status (81), website (73), date (63),
  language (63), publisher (55), last/first (43). `cite journal`: also
  journal, volume, issue, pages, doi, s2cid, issn, jstor, bibcode, hdl,
  and `*-access` flags. `cite book`: isbn, oclc, page.
- Other templates in notes: `{{webarchive}}` (5), `{{dead link}}` (3),
  `{{cbignore}}` (1). In Nairobi they all follow free text (step 3).

### 2.2 The Wikidata item

153 statements; 42 have a reference; 42 references, 32 distinct (the
same reference hash on more than one statement).

| Reference properties | Count |
|---|---|
| P143 imported from Wikimedia project | 14 |
| P248 stated in | 8 |
| P248 stated in, P813 retrieved | 7 |
| P854 reference URL | 5 |
| P248, an identifier (P214 VIAF ID), P813 | 3 |
| P854, P1476 title, P813 | 1 |
| P248, P577 publication date / P1065 archive URL, P2960 archive date | 3 |

So a Wikidata reference is mostly "a database, maybe with an
identifier" or "a URL". Only one has a title.

### 2.3 Citoid

Citoid (`https://en.wikipedia.org/api/rest_v1/data/citation/mediawiki/{q}`)
gives Zotero data for a DOI, an ISBN or a URL. The DOI test worked
(type `journalArticle`, title, journal, volume, pages, authors). The URL
test failed: "Unable to load URL" (the site blocks it). So Citoid is
good for an editor's "fill from URL/DOI/ISBN", not as a source for every
reference at fetch time.

### 2.4 The United States (Q30), a first look

The English article has 723 notes: 418 with a template, 31 with only
COinS, 274 without a template. Most of the 274 are **short footnotes**
(`{{sfn}}` 206, `{{sfnm}}` 2): "Mintz 1996, p. 134", a link to
`#CITEREFMintz1996`, a full citation in the "Bibliography" section,
outside the notes. The page has 380 `CITEREF` anchors.

More templates: `cite magazine` (7), `cite report` (4),
`cite dictionary` (2), `cite encyclopedia`, `cite press release`;
in notes also `{{harvnb}}`, `{{multiref2}}`, `{{isbn}}`, `{{usc}}`.
New parameters: `quote`, `chapter`, `editor-*`, `id`, `loc`, up to six
authors.

So the model needs (in step 8, not before): a short citation that
points to a full one with a locator (page, pages, loc), and the full
citations of a bibliography section.

### 2.5 Favicons

`https://favicons.toolforge.org/favicon?url={host}` gives an icon for a
host. Checked: it serves the image itself (no redirect, so the reader's
browser does not contact the cited site), `cache-control: max-age=86400`,
and a default icon for an unknown host.

## 3. The typed model

### 3.1 Principle: a standard vocabulary, not template names

`cite web` and `cite news` are names of English Wikipedia templates.
German Wikipedia uses `Internetquelle` with German parameters. Wikidata
has no templates. So the kinds and fields follow CSL/Zotero item types
(the vocabulary that Citoid and Zotero use). A mapping table converts a
template and its parameters to a kind and its fields. That table is
data (§4.3).

### 3.2 The types (in `content/src/citation.rs`)

A first version, for what Nairobi uses. No other kind until a page
needs it. As built (step 1):

```rust
/// A source that a text or a statement cites.
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Citation {
    Web(Web),         // a page on a site
    News(News),       // an article in a newspaper or a news site
    Journal(Journal), // an article in a journal
    Book(Book),
    /// A database or a work that Wikidata names (P248), maybe with the
    /// identifier of the record in it.
    Record(Record),
    /// Free text that no template made. Not parsed: no guess.
    Text { html: String },
}

/// The fields of every work, flattened into each kind (the JSON is flat).
pub struct Work {
    pub title: Option<String>,
    pub url: Option<WebUrl>,
    pub authors: Vec<Person>,
    pub date: Option<Date>,     // of publication
    pub accessed: Option<Date>,
    pub archive: Option<Archive>,
    pub language: Option<Lang>,
    pub ids: Vec<Identifier>,
}
pub struct Web { work: Work, website: Option<String>, publisher: Option<String> }
pub struct News { work: Work, newspaper: Option<String>, publisher: Option<String> }
pub struct Journal { work: Work, journal, volume, issue, pages: Option<String> }
pub struct Book { work: Work, publisher, place, pages: Option<String> }

pub struct Record {
    pub stated_in: Qid,
    pub id: Option<RecordId>,   // { pid, value }
    pub url: Option<WebUrl>,    // P854, else the identifier's formatter URL
    pub title: Option<String>,
    pub published: Option<Date>,
    pub accessed: Option<Date>,
    pub archive: Option<Archive>,
}

pub enum Person { Name { family, given: Option<String> }, Literal { name } }
pub struct Archive { pub url: WebUrl, pub date: Option<Date>, pub original: Original }
pub enum Original { Live, Dead, Unfit }   // CS1 url-status; usurped = unfit
/// A date as a Wikidata time with a precision, so the browser formats it
/// with `Intl`; else the text as written.
pub enum Date { Time { time: String, precision: u8 }, Text { text: String } }
pub enum Identifier { Doi, Isbn, Issn, Oclc, Jstor, S2cid, Bibcode, Hdl }
/// An http or https URL; "//" becomes "https:". Only it becomes a link.
pub struct WebUrl(String);  // .host(): the favicon host
```

Changed in step 1: the common fields are one struct `Work` (the plan
repeated them in each kind); `title` and `url` are optional (a bare
Wikidata URL has no title); `Archive.dead: bool` became `original`
(three states: a usurped original gets no link); `WebUrl` is a
validated newtype, so a `javascript:` URL cannot be stored.

Notes on the types:

- Text fields are plain text. The parser converts wikitext to text
  (`plain()`: `[[a|b]]` → b, `''x''` → x, `{{!}}` → "|"; another
  template is removed).
- A field that a kind does not have is not in its struct. Unknown
  template parameters are dropped. (The mapping table is closed with
  `deny_unknown_fields`; the input data from the wiki is not.)
- A Wikidata reference with P248 becomes `Record`; else with P854,
  `Web` (title from P1476).
- P143 "imported from" is not a source (`layouts.toml` already says
  so). It is not a citation; `Provenance::Imported` shows it as before.

### 3.3 What an edit needs later

An edit must write back to the origin, so each citation keeps where it
came from. Only what a save needs, nothing more:

```rust
pub struct Cited {
    pub citation: Citation,
    pub origin: Origin,
}
pub enum Origin {
    /// A template in the article: its name and its raw parameters, so an
    /// edit changes the same parameters (`access-date`, not `accessed`).
    Template { name: String, params: IndexMap<String, String> },
    /// COinS of a citation without template data.
    Coins,
    /// The free text of a note: an edit replaces the whole reference.
    Html,
    /// A Wikidata reference, by its hash.
    Wikidata { hash: String },
}
```

Changed in step 1: `Rendered` became `Coins` and `Html` (the report
counts each path); `Wikidata` keeps no snaks (an edit reads the live
entity, which it needs for the base revision anyway).

## 4. The backend

### 4.1 The article: one Parsoid parse

Today the fetcher parses the lead with the legacy parser (`section=0`),
then each matched section, then (for a fallback article) the whole
article with Parsoid. A section alone does not have its references: a
list-defined reference or a named reference that is defined in another
section gives "Cite error" (that is why `paragraphs()` filters it).

Change: **one Parsoid parse of the whole article** for every article.
From it: the lead (section 0), the first paragraph of each matched
section, the outline (fallback only), and the notes. Fewer requests
(1 in place of 2 + N), and the same document for prose and notes.
Cost: about 0.9 MB for a large article per fetch, cached as before.

### 4.2 Notes and marks

In the stored prose, a reference mark is kept in one canonical form:
`<sup data-note="cite_note-10"></sup>`. The `clean` function of the
fetcher keeps only this attribute on this element.

`Article` gets the notes of the article:

```rust
/// The notes of the article, by the id of their list item.
pub notes: IndexMap<String, Note>,

pub struct Note {
    /// None for the default group (numbers); "lower-alpha" for notes.
    pub group: Option<String>,
    /// The citations in the note, in order. Most notes have one.
    pub citations: Vec<Cited>,
    /// An explanatory note: its text, with its own marks.
    pub text: Option<String>,
}
```

The parse of one note (`fetch/notes.rs`), for each transclusion group
in it (the nodes of one `about`):

1. a template whose name is in the mapping table → its kind and fields;
2. else the COinS spans of the group → kind from the class of the
   `<cite>` before the span (`citation web`), else from `rft.genre`;
   fields from `rft.atitle`, `rft.btitle`, `rft.jtitle`, `rft.au`,
   `rft.aulast`/`aufirst`, `rft.date`, `rft_id` (URL, `info:doi/`,
   `info:oclcnum/`, `info:bibcode/`, `info:hdl/`), `rft.isbn`, `rft.issn`.

A loose `span.Z3988` (no template data: the infobox made it) also gives
a citation. A note with no citation is one `Citation::Text` with its
cleaned HTML. Text with words beside citations ("See also …") is a
`Text` of its own. An explanatory note (`data-mw-group`) keeps only its
text, with its own marks. The input is wild: no note fails, every note
gives a result.

Changed in step 3: `{{webarchive}}` and `{{dead link}}` are not read.
In Nairobi they follow free text, which keeps them as text; read them
when Q30 needs it (step 8). COinS gives no archive and no access date.
The kind of a COinS citation comes from the `<cite>` class first,
because CS1 writes `rft_val_fmt=book` for some `cite web`.

Only the notes that the stored prose marks are kept (the lead and the
matched sections; all of them for a fallback article with an outline).

### 4.3 The mapping table

`qrender/citations.toml`, a third embedded file of the configuration.
Its types (`CitationStyle`, `ByKind`, `Parameters`) are in
`content::settings` with `deny_unknown_fields`; the fetcher gets them in
`Settings.citations`, so the cache fingerprint follows them. A style is
a family of templates that share their parameters:

```toml
[[style]]
name = "CS1"
[style.templates]          # template names of each kind
web = ["cite web"]
news = ["cite news"]
journal = ["cite journal"]
book = ["cite book"]
[style.classes]            # the <cite> class of each kind (for COinS)
web = ["web"]
[style.parameters]         # the first parameter with a value wins
title = ["title"]
container = ["website", "work", "newspaper", "journal", "magazine"]
date = ["date", "year"]
accessed = ["access-date", "accessdate"]
[style.parameters.authors] # "{n}": empty or 1 for the first author
family = ["last{n}", "surname{n}"]
given = ["first{n}", "given{n}"]
name = ["author{n}"]
[style.parameters.archive]
url = ["archive-url", "archiveurl"]
date = ["archive-date", "archivedate"]
status = ["url-status"]    # absent: the original is dead (CS1)
```

A template name is compared without regard to case, with "_" as a
space. Another wiki's templates are a new style, when a page needs
them. A template in no style goes to COinS.

Changed in step 3: one table per style (shared parameters), not one
per template; the kinds are fixed keys, so a misspelt kind is a load
error; the `classes` table is new.

### 4.4 Wikidata references

`sparql::references` gives the values of each reference (`Snak`: the
property, its datatype, the value, its language, the time precision, and
the formatter URL of an external identifier, in the same query). The
reference node URI has the hash. `fetch/references.rs` maps them: P248
→ `Record` (its external identifier; the URL from P854, else from the
identifier's formatter URL); else P854 → `Web` (P1476 title and its
language, P577, P813, P2093, archive P1065/P2960); else no citation
(P143, P813 alone, P4656). The content gets:

```rust
/// For each statement, the property set of each of its references.
pub references: IndexMap<StatementId, Vec<Vec<Pid>>>,   // as before
/// For each statement, the hashes of its references that cite a source.
pub statement_citations: IndexMap<StatementId, Vec<String>>,
/// Those references, by hash: shared by all statements that cite them.
pub citations: IndexMap<String, Cited>,
```

The items of `Record.stated_in` go to the label set of the text.

Changed in step 4: additive, not a new shape of `references`. The
property sets stay, and `Provenance` and `Order::References` read them
as before. Reason: the eight content fixtures have the old shape, and a
new fetch of all of them would change every snapshot by data drift.

Alternative not taken: `wbgetentities` with `props=claims` gives the
exact snaks, no query-service lag, and the order of the snaks. But it
is a large response for big items (India). An edit reads the live
entity anyway (it needs the base revision), so SPARQL is enough for
display.

## 5. Composition and view

- `StatementRef.citations: Vec<String>` (hashes; the statement and its
  joined texts). The same `citations` list is beside every other
  `provenance` field: map marker, scale measure, score value, split
  share, account followers, series point and file series.
- `Composition.citations`: the citations that a block cites (the hashes
  under `citations` keys). The identifiers block cites none, as the
  provenance primitive does not count identifiers. (Localize does not
  read `Content`.)
- Localize:
  - turns each `<sup data-note="cite_note-12">` into a neutral mark with
    the page number of its note: `<sup data-note="3"></sup>`, empty. A
    mark of a note that the article does not have goes;
  - numbers the notes **on the page** at their first mark, in reading
    order (the lead, the spreads, the more sections; a note that an
    explanatory note marks comes after). The page shows some paragraphs
    of the article, so the article's numbers do not apply;
  - puts the shown notes in `View.notes` (number, group, language,
    citations, text), the statement citations in `View.citations`, and
    the labels of `Record.stated_in` in `labels`;
  - cleans the HTML of a free-text citation as prose, but keeps its
    `http`/`https` links (prose drops links to other sites).
- The view keeps the citations typed. Each renderer makes its form of
  the mark and decides the card.

Changed in step 5: the field is `citations`, not `references`; the
identifiers block cites none, so the composition's table is filtered;
the mark in the view is neutral and empty (each renderer makes its
form in step 6); the messages of the field names move to step 6.

## 6. Rendering

### 6.1 The card

`render/cite.rs` makes a `Card` of a `Cited`: a title line (title, link,
host, favicon, kind) and rows (a message key and a value). Both
renderers read it: `templates/parts/cite.html` for the page, one line
of text for a Markdown footnote.

```
[icon] History of Mombasa  mombasa.go.ke
       Authors     mombasa.go.ke
       Publisher   Mombasa County
       Published   July 28, 2018
       Retrieved   June 14, 2019
       Archived    January 26, 2021 · web.archive.org · the original is offline
```

- Title line: the favicon (`favicons.toolforge.org/favicon?url={host}`,
  `img.favicon`, lazy), the title as the link, the host without "www.".
  Only the host goes to the favicon tool, from a parsed `WebUrl`. No
  favicon (a record without a URL, free text): the Codex "reference"
  icon, inline.
- The title links to the original URL; to the archive when the original
  is dead or unfit (as CS1 does). No title: the address without its
  scheme, percent-decoded to read.
- A record: the label of its `stated_in` as the title, linked to the
  record's URL, else to the item on Wikidata; rows Identifier (the
  property and the value), Title, Published, Retrieved, Archived.
- A work: rows Authors ("Family, Given"; "; " between), the fields of
  its kind (Website / Newspaper / Journal, Publisher, Place, Volume,
  Issue, Pages), Published, Identifiers, Language, Retrieved, Archived.
- Dates are `Part::Time` from `View::time`, so `intl.js` formats them
  as statement dates; a language is a code that `intl.js` names
  (`data-lang-name`).
- Identifiers link to their resolvers: doi.org, Wikidata's
  Special:BookSources (ISBN), portal.issn.org, WorldCat, JSTOR,
  Semantic Scholar, ADS, hdl.handle.net.
- Free text: its cleaned HTML (with its links to other sites).
- Markup: spans only, laid out by a CSS grid, because a card is in a
  popover that can be in a line (`<p>`, `<h3>`, `<figcaption>`); a
  `<dl>` in a `<p>` would close the paragraph.

### 6.2 Where cards show

- **Prose**: `Ui::prose` turns the neutral mark of the view into
  `<sup class="ref"><button popovertarget="note-3">3</button></sup>`
  (`parts/note_mark.html`; its name for a screen reader "Note 3").
  The popover of each note is rendered once, after the spreads
  (`parts/notes.html`), with the `lang` of the article. No script.
- **A fact**: the `m::prov` macro takes the statement's citations. With
  citations: a button with the favicon of the first source (and the
  count when there is more than one), and its popover with the cards,
  next to it (`parts/cite_mark.html`): inside a "+N" dialog it stays in
  the dialog. Without: the provenance marks as before ("W", the open
  mark).
- **"Sources"**: at the end of the page, a collapsed `<details>` with
  the notes of the prose by number (`<ol>`, `<li value>`), the same
  body as the popovers.
- **Markdown**: `[^3]` in the prose and `[^3]: …` at the end; a value
  with citations gets `[^s1]` marks, and `[^s1]: …` lines after the
  notes.
- Popovers use the existing `.pop` pattern: `position-area` and
  `position-try-fallbacks`, no polyfill (a browser without anchor
  positioning shows the popover in the middle). The cards reset the
  colours and the pointer of a picture's caption that they are in (a
  popover inherits from its place in the page).

`qrender/tests/pages.rs` checks that every popover target exists, that
no link is in a link, and that ids are unique; `favicons.toolforge.org`
was already allowed for `img.favicon` (the web spread).

Changed after step 8 (2026-10-20, a review of the cards):
- **The fact button is the reference icon** for every statement, with
  the count when there is more than one; not the favicon of the first
  source (the logos of news sites on facts did not help).
- **The head of a card**: the favicon 32px, beside two lines: the title,
  then the host (subtle).
- **No value twice**: a value that is the host (an author or a
  publisher "mombasa.go.ke"), or a value of a line before (a publisher
  that is the website), is not shown.
- **Compact lines**: a journal or a magazine, its volume, issue and pages
  on one line ("World Futures · vol. 70 · no. 2 · pp. 120–139"; messages
  `qrender-cite-volume-in`, `-issue-in`, `-page-in`, `-pages-in`); a
  book's publisher and place on one line; an encyclopedia and its
  publisher on one line.
- **The archive** is the favicon of its site, linked (its host as the
  name of the link), between its date and the state of the original;
  the Markdown keeps a text link.
- More room: padding in the popover, space between cards and lines.

Changed in step 6: the card is phrasing markup, not a `<dl>`; fact
popovers are next to their buttons, note popovers in one place after
the spreads; the Markdown also has the sources of statements as
footnotes (`[^s1]`), not only the notes of the prose; `translate.js`
drops the marks from the text that it sends to MinT; the favicon host
needed no new rule.

## 7. Editing (postponed)

Decided: a dialog that saves nothing, as the translation of the
fallback article. Its plan (2026-10-20) decided four things:

- the card of an edited source and the mapping of Citoid's data are
  made on the server (two endpoints: a card from a `Cited`, a `Citation`
  from a URL, DOI or ISBN), so the card has one renderer;
- the editor edits existing sources only (adding a source to a
  statement without one is later);
- "what would be published" (the template's parameters, the Wikidata
  reference) is later;
- a free-text note is editable as a new structured source.

Postponed (2026-10-20), before it started. The plan and a handoff (what
changed in the code after the plan: more kinds, short footnotes, the
card head, sources once per list) are in
`.plans/future/citation-editor.md`.

## 8. Steps

Each step is a commit with tests, and the page works after each.

1. **Types.** `content/src/citation.rs`: the types of §3, serde, JSON
   Schema. Unit tests on the shapes.
2. **One Parsoid parse** (its own step). Lead, section texts and
   outline from one document. Tests first: snapshot the lead and the
   section texts of the fixtures before the change; after it, they
   must be the same, except where a section lost text to a "Cite
   error" (each such difference reviewed). Record
   `fixtures/http/q3870` again. Raise `FETCH_VERSION`.
3. **Notes from the article.** Marks kept, `Article.notes`, the parse
   of §4.2, the mapping table of §4.3. Tests in §8.1.
4. **Wikidata references.** The query of §4.4, `Content.citations`
   (additive; provenance unchanged). Tests: each
   reference shape of §2.2 maps to its kind; P143 alone is
   `Imported` and not in the table; a shared hash is stored once.
5. **Composition and view.** `StatementRef.citations`,
   `Composition.citations`, page numbering, `View.notes`,
   `View.citations`. Regenerate the schemas.
6. **HTML and Markdown.** The card, the marks, the popovers, "Sources",
   footnotes, the messages of the field names. CSS in `design/`; Codex
   tokens; logical properties.
7. **Editor**: postponed (§7).
8. **The United States**: fetch Q30, run the parse, read the report of
   §8.1, then add what §2.4 needs (short citations, the bibliography,
   the new kinds). Its plan and record: §10 (2026-10-20), done before
   step 7.

### 8.1 Tests of the parse

Citations are wild. The parse must never fail a fetch, and every note
must give a result. The tests:

- **One unit test for each path**, on a small HTML piece copied from
  the Nairobi article: a template on the TemplateStyles `<style>`; on
  the deduplicated `<link>`; COinS only; free text; `{{webarchive}}`
  after a citation; `{{dead link}}`; two citations in one note; a
  `lower-alpha` note with a reference in it; a named reference used
  three times (one note, three marks).
- **Hostile input**, each a unit test: `data-mw` that is not JSON; an
  `about` with no node; a parameter with a template or a link in it; an
  empty title; a date that is not a date ("c. 1990", "Spring 2004"); a
  protocol-relative URL; a `javascript:` URL; COinS with an unknown
  genre; a mark whose note is not in the list; a note with no mark.
- **The corpus test**: the parse of the whole recorded Nairobi article
  (from `fixtures/http/q3870`). Asserts: every note gives at least one
  citation; every mark in the prose has its note; every `href` is
  http(s). An insta snapshot of all the notes, as JSON: a change of the
  parse shows as a diff to review.
- **The report**: a test (or `qrender fetch` output) gives the count
  of notes for each path (template / COinS / text) and for each kind,
  and the template names that are not in the mapping table. For
  Nairobi now: 154 / 13 / 19. For Q30 this report is the start of
  step 8.

## 9. Decisions (2026-10-19)

1. **Favicon**: from `favicons.toolforge.org`, by host (§2.5, §6.1).
2. **Editing**: a dialog that saves nothing (§7).
3. **Numbering**: numbers for prose, icons for facts. The numbers have
   no meaning of their own (§6.2).
4. **One Parsoid parse**: yes, as its own step with tests (step 2).

## 10. Step 8: the United States (Q30)

Status: step 8 done (2026-10-20): 909b01f, 2ab2865, 20e8878, 8d5dfa2,
f2e52ef, 90248e7; the design route fix 5b1142c. Notes in §10.7.

### 10.1 The dev server

The dev server (port 4243, in the devcontainer) ran an old build. It
was built again and restarted (2026-10-20). No cache flush: every cache
key has `FETCH_VERSION` (13) and the settings fingerprint, so the new
build fetches again. `/en/Q3870` shows the marks (16) and the fact
sources (7).

The pages still looked old in a browser: the design files are cached
for a week as immutable, and their address had only the design version
(`/design/0.2/`), so a browser kept an old stylesheet (44 KB, without
the Codex tokens and the cards). Fixed (a separate commit): the address
has a hash of the files (`/design/0.2-efbf1e3c/`); the route serves the
files of the build at any address of the same design version.

### 10.2 What Q30 has (measured, 2026-10-20)

#### 10.2.1 The page as it is now

`/en/Q30`: 1.62 MB of HTML (128 KB gzip), first fetch 60 s. 72 note
marks in the prose; **640 fact source buttons**, each with its popover.

The fact popovers are 389 KB of the page, but only 66 are different.
496 of the 640 buttons are in the "+N" dialogs that list every value,
and there one source repeats for every value:

| Source | Times |
|---|---|
| "All 300-plus languages spoken in American homes" (languages used) | 210 |
| state.gov countries and areas list (diplomatic relations) | 190 |
| capitalizemytitle.com, the 50 states | 78 |
| ISO 3166-1 | 59 |

A column of 210 equal icons is noise, and the markup is repeated.

#### 10.2.2 The notes of the article

Parsoid HTML of revision 1380971971: 2.7 MB. 723 notes: 695 references,
28 explanatory notes (`lower-alpha`).

| Path | Notes |
|---|---|
| A template of a style | 400 (2 with text beside) |
| COinS only | 44 (2 with text beside) |
| Free text | 247 |

Citations by kind: web 366, news 65, book 20, journal 13, text 251.

Templates in the notes that no style has: `harvnb` 19, `isbn` 13,
`cite magazine` 7, `cite report` 4, `multiref2` 3, `usc` 3,
`omission` 3, `cite dictionary` 2, `harvard citation no brackets` 2,
`webarchive` 2, `cite encyclopedia` 1, `cite press release` 1,
`dead link` 1, `cbignore` 1.

#### 10.2.3 Short footnotes

Most of the free text is a **short footnote** (`{{sfn}}`, `{{harvnb}}`):
a link to a full citation in the bibliography, and a page.

- 211 text citations link to an anchor in the page (`#CITEREFFaingold2018`);
  197 targets, 226 uses.
- Their shapes: "LINK, p. 8." (56), "LINK." (51), "LINK, pp. 2–3." (22),
  a few with a quotation after; 9 name a work with no link ("Baym &
  Levine 2018, pp. …": the bibliography has no such anchor).
- 194 of the 197 targets are a `<cite id="…">` in the "References"
  section (the bibliography); 3 are not in the page.
- A bibliography entry has **no template data** (the list is in a
  template, so Parsoid does not keep the data of each entry), but it
  has COinS: the COinS path reads it as a typed work (Faingold 2018:
  a book, its title, publisher, year, author).

### 10.3 What to change

#### 10.3.1 Short citations

The content (`content/src/citation.rs`, `text.rs`):

```rust
pub enum Citation {
    …,
    /// A short footnote: a work of the bibliography, at a place in it.
    Short(Short),
}
pub struct Short {
    /// The anchor of the work in the article ("CITEREFFaingold2018").
    pub work: String,
    /// The words of the link: "Faingold 2018".
    pub label: String,
    /// The place: "p. 8", "pp. 2–3".
    pub at: Option<String>,
}

pub struct Article {
    …,
    /// The works that short footnotes cite, by anchor: the entries of
    /// the bibliography, read as notes are (template, else COinS).
    pub works: IndexMap<String, Cited>,
}
```

The parse (`fetch/notes.rs`): a note whose whole text is one link to an
anchor of the page, then optionally ", p. N" or ", pp. N–M", then
optionally "." is a `Short`. Anything else stays `Text` (the quotation
after a page, two links, a work with no link). The entries: every
`<cite id>` of the page outside the lists of notes, read with the
`<cite>` and its COinS (the code of the COinS path); kept only when a
kept note cites it.

The view: `View.works` (anchor → `Cited`), the works that the shown
notes cite. The card (`render/cite.rs`): a `Short` is the card of its
work, with a row "Cited at: p. 8" (a new message); a `Short` whose
work is missing is its label and its place, as text. Markdown: the
same line, "…. Cited at: p. 8".

One work cited by ten short footnotes is stored once, and its card is
made for each note.

#### 10.3.2 More kinds

The CS1 templates of Q30 that have no kind:

| Template | Times | Kind |
|---|---|---|
| `cite magazine` | 7 | **magazine**: magazine, volume, issue, pages |
| `cite report` | 4 | **report**: publisher (institution), number, pages |
| `cite encyclopedia`, `cite dictionary` | 3 | **entry**: the entry (title), the work (encyclopedia), publisher, edition |
| `cite press release` | 1 | web (it is a page of the publisher) |

Each a struct with the `Work` fields and its own (§3.2), keys in `citations.toml` (`templates`, `classes`), messages of
the new field names. Citoid's `magazineArticle`, `report`,
`encyclopediaArticle`, `dictionaryEntry` map to them later (the editor
plan).

Not now: `{{usc}}` (a law, 3), `{{webarchive}}` (2), `{{dead link}}`
(1). Their notes stay text.

#### 10.3.3 The repeated fact sources (a decision, §10.6)

The citations that more than half of the values of a list have (two
values at least) show once: after the name of the property (a network
block) or with the tools of the row (a fact row), and on no value with
exactly those citations, on the page and in its dialog. A value whose
citations differ keeps its own button. The Markdown puts the marks after
the key of the list.

Changed in step 8.4: first "every value has the same citations"; on the
Q30 fixture that missed the languages (210 of 212 cite one census, 2 cite
another) and the states (64 of 67): the rule became "more than half". This is a decision of the renderer (`ui.rs`, the
rows and the dialogs), from the view; the composition does not change.

Q30: the 210 + 190 + 78 + 59 buttons become 4; the markup loses most of
the 389 KB.

### 10.4 Steps

1. **Q30 corpus.** Save the Parsoid HTML of the article as a fixture,
   compressed (`fixtures/notes/q30.en.html.gz`, about 300 KB), and add it
   to `app/tests/notes.rs` beside Nairobi: the invariants, the report,
   a snapshot of the report (not of all the notes: too large; a sample
   of 20 notes of each path).
2. **Short citations** (§10.3.1). Unit tests on pieces of Q30: "LINK, p.
   8.", "LINK.", "LINK, pp. 2–3.", a quotation after (text), a link to
   no anchor (text), a work cited twice (stored once). The report:
   short citations counted; target found or not.
3. **More kinds** (§10.3.2). Unit tests on one piece of each template.
4. **The repeated fact sources** (§10.3.3), if decided. Test: `pages.rs`
   on Q30 (a new fixture page) — or on a fixture page that has a list
   with one source for all values.
5. **Fixtures**: Q30 content and text from recorded responses (as
   Nairobi), so its pages are snapshots too.

### 10.5 Cost

- The fixture: the HTML of Q30 is 2.7 MB; gzip about 300 KB. A test reads
  it with `flate2` (a new dev-dependency) or the fixture is stored
  plain. See §10.6, question 3.
- The text of Q30: the bibliography entries kept are those of the
  shown notes only.

### 10.6 Decisions (2026-10-20)

1. **Short citations**: a `Short` kind and a table of works (§10.3.1).
2. **Repeated fact sources**: shown once per list when every value has
   the same sources (§10.3.3).
3. **The Q30 fixture**: compressed, read with `flate2` (a
   dev-dependency of `app`).
4. **Order**: this step before the editor (step 7).

### 10.7 Notes

Step 8.1 (909b01f): `fixtures/notes/q30.en.html.gz` (426 KB, gzip -9 -n of
revision 1380971971), read with `flate2`; `app/tests/notes.rs` checks
both articles with one `check`, which also allows links to a place in
the page (`#`). The Q30 snapshots: the report, and ten notes of each
path (all 723 are too many).

Step 8.2 (short citations), as §10.3.1, with these points:
- `article_notes` gives `ArticleNotes { notes, works }`; `Note::works()`
  (in `content::text`, no `fetch` feature) gives the anchors that a
  note's short footnotes cite, for the fetcher and for localize.
- The rule (`notes::short`): the cleaned HTML of the note is one
  `<a href="#…">`, then maybe ", p. N" or ", pp. N–M", then maybe ".".
  A work is read from its `<cite id>`: the template of its
  transclusion (`template_of`), else the COinS after it, else its text.
- Found on Q30: the messages of citation templates for editors
  ("sfn error: multiple targets") leaked into the text of notes: the
  cleaner removes classes. `write_element` now drops what the wiki's
  CSS hides: `harv-error`, `cs1-hidden-error`, `cs1-maint` (beside the
  hidden forms of a coordinate).
- Q30: 201 short citations; 177 works cited, 175 in the page (126
  books, 42 journals, 7 web pages); 50 citations stay text (36 written
  by hand; several short footnotes in one note; a quotation after the
  place; "In Lauter 1994a, pp. …").
- Not done: several short footnotes in one note (`{{sfnm}}`, "A 2007,
  p. 5; B 2015") stay text.
- The card: the work's card with a row "Cited at: p. 226"; no work: the
  label and the place. Message `qrender-cite-at`. `View.works`.

Step 8.3 (more kinds), as §10.3.2, with these points:
- `Magazine`, `Report`, `Entry` in `content::citation`; `ByKind` has
  `magazine`, `report`, `entry`; `citations.toml`: `cite magazine`,
  `cite report`, `cite encyclopedia` and `cite dictionary` (class
  `encyclopaedia`), `cite press release` as web (class `pressrelease`).
  The container parameters also `encyclopedia`, `encyclopaedia`,
  `dictionary`.
- The parse builds a citation in one place (`assemble`, from `Fields`)
  for template parameters and for COinS; before, each had its own
  match of every kind. COinS: a report takes its title from
  `rft.btitle`, as a book; an entry's encyclopedia is `rft.btitle`.
- Messages `qrender-cite-magazine`, `qrender-cite-encyclopedia`.
- Q30: magazine 7, report 6, entry 5; templates in no style now only
  templates that are not citations (`harvnb`, `isbn`, `usc`, …).

Step 8.4 (sources once per list), as §10.3.3:
- `cite::shared(statements)` (both renderers); `Ui::shared_citations`,
  `Ui::values_beside(statements, &shared)` (a value with exactly the
  shared citations has none); `network.html` (the heading) and
  `fact-row.html` (the tools); `Markdown::list_values` for the network
  and fact blocks.
- Not done: the names block (official names in many languages, each
  citing ISO 3166-1: 59 on Q30) has its own template; the same rule
  would fit there.

Step 8.5 (the Q30 fixture):
- `fixtures/content/q30.json` (1.15 MB) and `fixtures/text/q30.en.json`
  (0.58 MB), from a live fetch (2026-10-20), in the lists of
  `compositions.rs`, `views.rs`, `pages.rs`, `schema.rs`. Changed from
  §10.4: no HTTP recording for Q30 (288 responses, 9.5 MB); the replay
  test stays with Nairobi.
- The page: 640 source buttons became 171; 1.54 MB of markup became
  1.28 MB (126 KB gzip). Sources once for diplomatic relations (182),
  the states (56), and four fact rows (language used, ethnic group,
  plug type, the states).
- Q30 has a lexeme value (`WikibaseLexeme`): the model leaves it to
  `Value::Other` on purpose (`Datatype::Other`), so
  `content/tests/fixtures.rs` allows Other for the lexeme datatypes.
