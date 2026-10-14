# Three-layer composition: content, composition, layout

Status: **plan for discussion. Do not start implementation until the plan is
confirmed.** The first prototype of this plan is `prototype/v3/` (Nairobi, in
English and Gujarati); what it showed is in `three-layer-findings.md`, and
this plan is updated with it.

This plan records the decisions from the prototype work (the first-generation
pages, the second-generation composer `prototype/v2/compose_v2.py`, and the page
grammar study). It defines the architecture for a new implementation in Rust.

The current Rust code (`qjson/`, `qrender/`) does not constrain this plan. We
can discard it. We keep only knowledge that the prototypes proved (see §9),
not code.

## 1. The problem this plan solves

The prototypes found good parts: scale against the container, the place among
peers, provenance marks, absences made visible, the English lead with glosses
in the page language. They did not find a page structure. Each layout attempt
(fixed card sizes, a shelf packer, justified rows) moved the problem to a new
place. Two causes:

1. We designed layout mechanics before we had a design grammar.
2. Editorial decisions, layout decisions and HTML were mixed in one program.

The fix is to separate three layers, and to give each layer one owner, one
test method and one version. Between the composition and the renderers, one
step does all the work that depends on the language: **localize**.

## 2. The three layers

| Layer | What it holds | Owner | Depends on language? | Tested by |
|---|---|---|---|---|
| 1. Content | Typed statements, qualifiers, labels with their language, provenance, media metadata, side data (peers, container, class statistics) | Data layer | Labels only | Fixture tests |
| 2. Composition | Editorial decisions: spreads, templates, slots, primitives, priorities, silences, absences, prose slots, symbolic layout tokens | Composer | No (see §5) | JSON snapshots per item |
| — Localize (a step, not a layer) | Nothing of its own: it derives a localized view from the composition and the text of one language | Localizer | Yes, only this | Tests per language |
| 3. Layout | Responsive geometry: what a token means at each width | CSS design system | No | Visual tests per template |

**Localize** turns the composition into a **localized view** for one
language: every identifier resolved to a label with its language and a
fallback flag, every value formatted, the lead chosen and its links
rewritten, prose sections attached to spreads (§5). It stores no judgment of
its own, so it is a step, not a layer.

A **renderer** joins the localized view with layer 3. The template part is
thin: loops, escaping, attribute copies, no decisions and no layout
arithmetic (one exception: §7).

```
Layer 1   content/<qid>.json              text/<qid>.<lang>.json
          (per QID: structure, no labels)  (per QID × language: labels, lead,
                │                            sections, page images)
             compose                                 │
                ▼                                    │
Layer 2   composition/<qid>.json                     │
          (per QID, no language)                     │
                └──────────────────┬─────────────────┘
                               localize
                                   ▼
          view/<qid>.<lang>.json (per QID × language: every string resolved)
                                   │
                  ┌────────────────┼──────────────────┐
                  ▼                ▼                  ▼
           HTML templates     app renderer     text / markdown for LLMs
                  │
                  ▼
                HTML  ◄── Layer 3: design system CSS (grid, templates, tokens)
```

The design system does not read the view. CSS sees only the markup that the
templates emit, with `lang` and `dir` attributes for script-specific rules.

## 3. Layer 1: content

Layer 1 has two parts, because they change for different reasons:

- **Structure**, one file per QID, the same for every language: statements
  with identifiers and raw values only (one SPARQL query without the label
  service), references, property classes, container and peers, Commons
  metadata, sitelinks.
- **Text**, one file per QID and language: the label and description of
  every identifier the structure uses, with the language each came from;
  the lead with its link map; the section titles of the article; the labels
  and aliases of the concept items that name spreads and prose sections, and
  the sections they match; page images (they come from an article, so they
  have a language).

In more detail:

- One record per item. It holds every statement, grouped by statement node
  (never by label), with its qualifiers, rank and references.
- Every label carries its language (`ig`, `mul`, `en`). The label chain is
  `{lang}, mul, en`. A `mul` label is not a fallback.
- Every media file carries its size (width, height, bytes), author, licence
  and Commons assessments (Featured, Quality, Valued).
- Side data, each from one generic query, cached apart from the item:
  - property classes (P31 of each property);
  - the references of each statement, with their properties;
  - the container item (P397, P171, P17, P131, P361, in this order) and its
    normalized quantities;
  - the peer set (the most specific class with 5 to 150 members), the peer
    quantities and the property frequency among peers;
  - the Wikipedia leads as HTML, with a map from each link to its item, its
    local article and its local label;
  - page images for items without P18 (from wiki-display-image).
- Cache keys follow how often each part changes: property classes per
  property, peers per class, item data per item. They do not all expire
  together.

## 4. Layer 2: composition

### 4.1 Vocabulary

The composition uses the grammar from the page grammar study:

- **Primitive**: one question, one visual form, one data shape, one silence
  threshold. The candidate list: identity, kind and context, scale, place,
  anatomy, time, network, peers, measures, evidence. The list may change;
  it is data, not code structure.
- **Spread**: one idea. It has one hero primitive and up to three
  satellites. A page is a short sequence of spreads.
- **Template**: a named arrangement of slots for a spread
  (`hero-plus-stack`, `lead-beside`, `media-run`, `sheet`, …). A small
  fixed set, owned by the design system (§6).
- **Slot**: a named place in a template (`hero`, `stack`, `aside`, `run`).
- **Class layout**: for a kind of item (settlement, person, taxon,
  astronomical object, generic), the order of spreads and the primitives
  each spread prefers. Community-editable configuration.

### 4.2 What the composer decides

- Which primitives pass their thresholds, and which stay silent, with the
  reason ("time: 1 dated point, needs 4").
- Which spreads exist, in which order, and which template each one uses.
- What fills each slot, with a priority.
- The hero of each spread, from data signals: Commons assessment, the kind of
  view (config), lead portraits (config), priority.
- Symbolic layout tokens and intrinsic facts (§4.4).
- Prose slots: sections the class layout expects, with how many languages
  have them, and whether this language has.
- Absences: properties most peers have and the item lacks, with the count.
- Provenance summaries for every slot and for the page.

### 4.3 What the composer never decides

- Pixels, columns at a given width, rows at a given width.
- Row breaks of a run of media.
- How many chips fit in a box. It can give a coarse cardinality tier
  (`few`, `some`, `many`), not a count.

### 4.4 Symbolic layout: tokens and intrinsic facts

The JSON can say two kinds of layout things, and no others:

1. **Tokens from the design system**: a template name, a slot name, a span
   token (`full`, `wide`, `half`, `third`, `narrow`), a density token.
   Each token has one meaning, which the CSS defines.
2. **Intrinsic facts about the content**: aspect ratio, megapixels, text
   length class, cardinality tier, priority, `to_scale: true`.

Rule: if a value depends on the viewport, the font or the language of the
page, it does not go in the JSON.

### 4.5 Sketch

```json
{
  "schema": "composition/0.1",
  "templates": "design/0.1",
  "qid": "Q3870",
  "class_layout": "settlement",
  "spreads": [
    {
      "id": "identity",
      "template": "lead-beside",
      "slots": {
        "hero": { "primitive": "identity", "media": "Nairobi, view from KICC.JPG",
                  "aspect": 1.333, "assessed": [], "megapixels": 5.9 },
        "aside": [ { "primitive": "kind", "classes": ["Q1549591"], "container": "Q114" } ]
      }
    },
    {
      "id": "scale",
      "template": "hero-plus-stack",
      "slots": {
        "hero": { "primitive": "scale", "mode": "share", "container": "Q114",
                  "measures": [ { "pid": "P1082", "ratio": 0.11 },
                                { "pid": "P2046", "ratio": 0.0012 } ] },
        "stack": [ { "primitive": "peers", "pid": "P1082", "class": "Q1549591",
                     "rank": 1, "of": 6, "span": "narrow" } ]
      }
    },
    {
      "id": "history",
      "template": "sheet",
      "prose_slot": { "section": "history", "exists_in": 38, "local": false },
      "silent": [ { "primitive": "time", "reason": "below_threshold",
                    "points": 1, "needs": 4 } ]
    }
  ],
  "sheet": [ { "pid": "P2044", "priority": 0.7, "provenance": "none" } ],
  "evidence": { "sourced": 18, "imported": 11, "none": 72 },
  "absent": [ { "pid": "P575", "peers_with_it": 47, "peers": 49 } ]
}
```

The schema is a sketch. The v3 prototype writes a real composition
(`prototype/v3/data/composition/q3870.json`); its shape, not this sketch, is
the starting point for the schema.

## 5. Composition is language-independent; localize does the language work

Which spreads exist, which picture leads, the ratios, the ranks, the
thresholds: none of these depend on the page language. Labels do, and label
availability feeds the coverage counts, nothing else. The v3 prototype
confirmed it: the Gujarati Nairobi page was rendered from the same
composition file as the English one (same checksum), with only a text file
added.

Consequences:

- Compute the composition once per QID. For 300 languages, this is one
  composition, not 300.
- Language affects length (a label in Malayalam is longer than in English).
  The composition must not depend on length; CSS handles length (§6.3).

### 5.1 The localize step

Input: the composition and the text file of one language. Output: the
**localized view**, `view/<qid>.<lang>.json`. It has the shape of the
composition, with display fields added:

- a **label object** next to every identifier:
  `{"text": "Kenya", "lang": "en", "fallback": true}`. A `mul` label is not a
  fallback;
- a **display form** for every value: numbers, dates and percentages in the
  formats of the page language; qualifiers as a note made of parts
  (`[{"text": "1920 – 1963"}, {"text": "Kenya Colony", "href": …}]`);
- the **lead**: the article in the page language, or the fallback article,
  with its markup cleaned and its links rewritten (to the article in the
  page language, or to the item with a gloss in the page language);
- the **prose links** of every spread (matched article sections and topics)
  and the sections no spread claims;
- **page images** for items without P18, marked with their source;
- **label coverage** in the page language, over what the reader sees;
- the page title, the text direction.

The view is data, not HTML. The article paragraphs are the one exception:
they are prose markup from Wikipedia.

Why a separate step:

1. **Every renderer shares it.** An app, a print renderer and the text
   renderer for LLMs read the view and need no language logic of their own.
   Otherwise each renderer repeats fallbacks and number formats, and they
   drift apart.
2. **It is the cache and API unit per language.** Compositions are cached per
   QID; views per QID × language. A label edit on Wikidata invalidates views,
   never compositions.
3. **Its tests are language tests**, apart from editorial snapshots: digits
   and separators, RTL, `mul`, fallback marks, section matching.

In v3 the localize step is 259 lines, about as much as all the HTML
templates (268), so it is a real component, not glue. A view is about twice
the size of its composition (100 KB against 49 KB for Nairobi), because every
string is resolved.

## 6. Layer 3: the design system

### 6.1 The page grid

- The page defines one grid of 12 column tracks.
- Each spread uses `grid-template-columns: subgrid`, so every component
  aligns to the same page columns in every section.
- Subgrid for columns only. Rows are content-sized. Fixed row heights caused
  the cropping in the prototypes.

### 6.2 Templates

- A template is a `grid-template-areas` arrangement with named slots.
- Container queries reflow each template: hero beside satellites when wide,
  hero above satellites when narrow.
- The template set is small (six to ten) and finite. It is the main design
  work, and a designer owns it, not the composer.
- Media runs use justified rows in CSS only. Each figure gets
  `flex: calc(var(--a) * H) 1 calc(var(--a) * H px)` and `aspect-ratio:
  var(--a)`, the run wraps, and a trailing flex item stops the last row from
  stretching. The browser chooses the row breaks, at every width.

### 6.3 Text that must fit

Use CSS mechanisms, not counts computed ahead of time: `line-clamp`,
`<details>` for "+N", `:has()`, container query units. Language length is a
CSS problem.

### 6.4 Rules from the prototypes that the design system keeps

- One surface level: no boxes inside boxes.
- Colour means one thing: the dimension of the data.
- Size means magnitude only inside a scale primitive that says "to scale".
  Elsewhere size means priority.
- Texture (dashed, dotted) means absence or "not yet".
- Logical properties only, so RTL needs no special case. Maps use physical
  coordinates, because a map does not flip.
- A photograph is never cropped unless a property says it is a banner.
- Letter-spacing is never used on text that can be in an Indic script.
- A ruby gloss is one inline block (`<rt><span>…</span></rt>`, the span with
  `display: inline-block`). Otherwise Chromium spreads the gloss over the base
  word and splits the clusters of Indic scripts.
- Wikidata labels are lower case by convention; titles raise the first letter
  with `::first-letter`, which does nothing in a script without case.
- Thumbnails use the standard Wikimedia widths.

## 7. The renderer contract

- Input: a localized view (§5.1) and a design system version. Nothing else:
  no composition, no labels, no text file.
- Output: HTML whose classes and data attributes are the tokens of the
  composition, with no computed geometry.
- The renderer has no editorial logic, no language logic and no layout
  arithmetic. A renderer that needs one of them shows that the composition
  or the view lacks a field.
- One exception: a map. Its zoom depends on the frame the renderer draws, so
  the composition gives the extent of the item and each renderer computes the
  zoom for its own frame.
- Other renderers (native app, print, text and markdown for LLM consumers)
  implement the same template set. With a small set this is feasible.
- The text renderer keeps qualifier notes, absences and provenance, because
  LLM consumers need them.

## 8. Versioning, QA and materialization

**Versions.** Three numbers: the composition schema, the view schema and the
design system. A renderer declares which view and design versions it
supports. A new template is a design system release; nothing else changes.

**Tests.**
- Composer: JSON snapshots per fixture item, validated against a JSON Schema
  for the composition. A change that demotes a head of state is a one-line
  diff.
- Localize: view snapshots per fixture item and language, and unit tests per
  language (digits, separators, RTL, `mul`, fallback marks).
- Design system: one visual test per template, with a gallery of fixture
  compositions (thin item, rich item, long labels, RTL, phone width).
- Renderer: small enough that the template tests cover it.

**Materialization.**
- Compute compositions for the items that readers ask for, not for all
  115 million. Compute views for the languages that ask for them.
- Invalidate from the Wikidata recent changes stream: a statement of the
  item, its container or its peer class invalidates the composition and its
  views; a label or description invalidates only the views.
- A fast path recomputes one item on demand, so that an edit shows soon.
- A solved layout per breakpoint may be cached as a derived file. It is
  never the source of truth.

## 9. What we keep from the prototypes, and what we discard

**Keep (knowledge, not code):**
- The SPARQL query shape: one query, statements grouped by node, the label
  chain `{lang}, mul, en`, deprecated rank dropped, QID and language
  validated before interpolation.
- The side queries of `prototype/v2/fetch_v2.py`: property classes, references,
  container, peers, frequencies, leads with link maps, Commons metadata,
  page images.
- The rules of `prototype/v2/compose_v2.py`: dimension from datatype and
  property class, range from min and max qualifiers, series from distinct
  years, provenance classes (P143 and similar are not sources), the peer and
  container rules, absences at 60 % of peers, the feature signals.
- The judgment in `prototype/v2/v2.toml` and the card groups and footnotes of
  `qrender/groups.toml` (v3 keeps its own copy in `prototype/v3/layouts.toml`,
  so the plan does not depend on the old crates).
- The v3 prototype as the reference for the layers: `content.py`,
  `compose.py`, `localize.py`, `render.py`, `design/page.css`.
- The lead rewriting: links to local articles or to items, ruby glosses in
  the page language, alt text of images kept.
- The 3D viewer (`prototype/v1/viewer3d.js`), loaded only on request.

**Discard:**
- The current Rust crates as code.
- Fixed card size catalogues, the shelf packer, row breaks computed in
  Python, and fixed row heights.
- Per-item build scripts.

## 10. Phases

Each phase needs confirmation before it starts.

0. **Confirm this plan**, and finish the grammar study (check the references,
   run Nairobi through the comparison table, settle the size rule).
1. **Prototype the schema in Python.** *Started in `prototype/v3/`:* content,
   composition, localize and a template-only renderer, for Nairobi in English
   and Gujarati. Still to do: the Moon and the golden eagle without new code
   paths, a thin item, and a JSON Schema for the composition and the view.
2. **Design the template set** in CSS with a fixture gallery: the page grid,
   subgrid, the first templates (`lead-beside`, `hero-plus-stack`,
   `media-run`, `sheet`), CSS-only justified rows. Test at phone, tablet and
   desktop widths, and in RTL.
3. **Freeze composition schema 0.1** and design system 0.1.
4. **Rust: content layer.** Data fetching, side queries, caching.
5. **Rust: composer.** The rules and the class layouts, with JSON snapshots
   matching the Python output for the fixture items.
6. **Rust: localize, renderers and server.** The localize step, the thin
   HTML renderer, the text renderer, the JSON API (composition and view).
7. **Scale tests.** Compose the most-read items of a pilot language and
   measure: which spreads pass their thresholds, which primitives are
   silent, what the median page is.

## 11. Requirements for the details, by layer

The v3 pages showed that the layers work, and that the details are wrong in
ways a reader sees at once: a population count on a history timeline, a
person with a timeline instead of a life, a leftover sheet that looks like a
database, credits over the pictures, no edit actions. None of these is a
limit of the layers. Each one has a place in them:

| Requirement | Layer | Mechanism | Data |
|---|---|---|---|
| Credit behind an (i) icon | Templates, design | A button with a popover (a hover title alone fails on touch screens) | In the view |
| A time axis only for history | Layer 2 | A **property role** decides: events (inception, significant event) and spans that change what the item is (former country, capital of) go on the axis; dated measurements (population by year) are a series, never events | Role table in config |
| No time axis for a person | Layer 2 | The person class layout has no time spread; dates live in the life spread | Class layout |
| Sections like an article | Layer 2 | Class layouts list their spreads and the properties of each (sketches below) | Config, seeded from the v1 pages |
| Prose between spreads | Layer 1 text, localize, templates | Each spread names prose concepts; the text layer fetches the first paragraph of the matched article section; localize puts it in the view; without an article the slot invites | Section text per language |
| Office holder with predecessors | Layer 2, templates | A **succession** block: the office (P1313) as title, the current holder large, the predecessors newest first, a short list, the rest in a dialog | In the data |
| The leftover sheet ("a database") | Layer 2, design | Roles send extreme points to the map, codes and web accounts to evidence, the tabular population file to the series; what is left is **demoted**: collapsed at the end, not a spread | Roles; Commons tabular data |
| Absences from small peer sets | Layer 2 | A minimum number of peers for the absence rule | Threshold in config |
| Colour | Design | A deliberate palette per dimension | None |
| Edit a statement | View, templates | Every statement block has an edit action that opens the statement on Wikidata | Statement and property IDs, in the view |
| Localize a label quickly | Localize, templates | A label that fell back carries its ID; the template gives it an action to `Special:SetLabel/<lang>/<id>`; the coverage list is a list of those actions | ID on each label object |
| Provenance that is clear | Templates, design | Mark the exceptions: nothing for a sourced statement, a worded tag for "imported from Wikipedia", a quiet mark for "no reference"; "add a reference" in the edit action | In the view |

### 11.1 Property roles

A table in the configuration gives a property a role when its datatype is
not enough: `event`, `history_span`, `series`, `tabular`, `extreme_point`,
`office` (holder property to office property, for example P6 to P1313),
`web` (accounts, codes, URLs), `relation` (spouse, partner: an end date
does not make the value former). Roles are the property-to-role mapping of the
strategy document: community-editable data, and a task a community can
finish for the top few hundred properties.

### 11.2 Class layouts

- **Person**: identity (banner, image, description), lead, **life** (birth,
  death, age, places of birth, death and burial, cause of death,
  citizenship, languages), **family** (parents, spouses, children,
  siblings), **education** (schools, degrees, advisors, students), **work**
  (occupations, employers, positions, memberships, notable works, and works
  by the person from reverse statements), **awards** (received, nominated),
  names, media, more (collapsed), evidence.
- **Settlement**: identity, lead, **location** (map with extreme points,
  located in, country, borders, elevation, area, locator map), **names**
  (official and native names, demonyms, nicknames, motto, emblems,
  pronunciation, codes), **government** (office and holder with
  succession, legislative body, budget), **demography** (the population
  series, share of the country, peers, male and female population),
  **economy**, **connections** (twin cities, memberships, capital of),
  **history** (inception, significant events, former countries, what it
  replaced, as a time axis), media, more (collapsed), evidence.
- Every spread names its prose concepts, so the matched article section can
  fill the slot at its start.

### 11.3 Data that layer 1 must add

1. **Reverse statements.** A person's works are statements on the works
   (author, director, composer, creator, performer). A class layout names
   the reverse properties it wants; one query per item fetches them.
2. **Commons tabular data.** A property like P4179 (tabular population)
   points to a data file on Commons; its rows are the best series.
3. **Section text.** The first paragraph of each article section that a
   spread matches, in each language that has the article.

## 12. Open decisions

1. The final primitive list and the first template set (phase 0 and 2).
2. Class layouts: which classes, and how an item finds its class layout when
   P31 does not match (the signal scoring of the old `archetypes.toml` is
   one option).
3. Where the configuration lives so that a community can edit it without a
   deployment (a wiki page, a Wikidata item, a file in the repository).
4. Peer rules for people. The class "human" has 12 million members, so
   people need another peer definition (occupation and country, for
   example).
5. How far native renderers are in scope, or whether a web view is enough.
6. Links in the lead and in the page: when an item has no local article,
   the link should go to its composed page. That needs the route scheme of
   the production service.
