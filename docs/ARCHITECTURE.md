# QRender Architecture

QRender renders a Wikidata item as a page a human wants to read and as
text an LLM can consume — from one pipeline. This document explains the
problem it solves, the principles that shape it, and how the pieces
fit.

## 1. Problem statement

A Wikidata item is a bag of statements: hundreds of property–value
pairs of wildly varying importance, with no inherent order, no
hierarchy, and no guarantee of what is present. Rendering that bag
naively produces two failure modes:

- **The data dump.** Every statement becomes an equal-weight row or
  card. The page is complete but unreadable: "population 5,545,000"
  sits next to "maintained by WikiProject Kenya" with identical visual
  weight, and the reader does the editorial work the renderer refused
  to do.
- **The rigid template.** A hand-designed layout per topic (the
  Wikipedia-infobox approach) looks great for the exemplar item it was
  designed against and breaks for everything else, because Wikidata
  items are not uniform. A designer's mockup for Paris assumes a
  population series, maps, a coat of arms, and significant events;
  most places have a fraction of that. Douglas Adams has a career's
  worth of dated statements; most humans on Wikidata have five facts.

The design problem is therefore not visual styling but **editorial
judgment under uncertainty**: decide what is the headline, what is
supporting material, and what is noise — for any item, including ones
whose shape nobody anticipated — and degrade gracefully as data thins
out.

Two constraints sharpen the problem:

- **Language independence.** Wikidata is multilingual; the renderer
  must not accumulate translatable UI strings of its own (there is no
  translation pipeline here, and there should not need to be one).
- **Delivery constraints.** Factoid pages ship no JavaScript beyond
  one small progressive-enhancement script (the cover-chips popover
  morph; the page is complete without it), reference only Wikimedia
  hosts (Commons thumbnails, Wikimedia Maps tiles), and must work in
  both color schemes and both text directions.

## 2. Design principles

1. **One IR, many backends.** Every output format — plain text,
   markdown, wikitext, semantic HTML, the visual factoid page, and the
   JSON API — renders the same intermediate representation (the "card"
   document). Editorial decisions are made once, in the IR; backends
   only serialize.

2. **Derive from data, configure judgment.** Whatever *can* be derived
   from the data's own types, is: an image value becomes an image
   card, coordinates become a map, a quantity series becomes a chart.
   Whatever is genuinely human judgment — grouping, importance,
   "0.6 is a mediocre HDI", "P31 Q5 means person" — lives in checked-in
   config (`groups.toml`, `archetypes.toml`), never in code.

3. **Composition is a progressive enhancement.** The generic page
   (hero + tiered card grid) is the baseline product and must stay
   good, because the long tail of items will never match a curated
   archetype. A recipe may *promote* cards into sections, *synthesize*
   cross-property features, and *skin* the page — it may never make a
   sparse item look broken. Features carry minimum-content thresholds
   and collapse below them; unclaimed cards always render; an item
   with nothing but a label degrades to a dignified header.

4. **Demote, don't delete.** Real data that makes a bad card
   (Wikimedia curation metadata) is collapsed, not dropped. Only
   structural properties that can never make a card (P31, P279,
   "reason for preferred rank") are ignored outright.

5. **i18n is designed away, not deferred.** Property labels, value
   labels, and descriptions arrive already localized from the WDQS
   label service (fallback chain `{lang}, mul, en`). The renderer's
   own vocabulary is icons, numbers, and images. Group and section
   names are machine names, visually hidden behind icons; the footnote
   toggle shows a count. The page contains no string that would need a
   translation file.

6. **Reading order is DOM order.** Cards are sorted in Rust; CSS never
   reorders content. Screen readers, text backends, and the visual
   grid all see the same sequence.

7. **Modern CSS does the work; JS only enhances.** Bento layout is
   `grid-auto-flow: dense` with per-card span variables; cards adapt
   to their own size with container queries; full-bleed media uses
   `:has()`; the all-values popover is the native popover attribute;
   everything is logical-properties-only so RTL needs no special
   casing. The single inline script morphs a card into its popover
   with a view transition — cosmetic only, never load-bearing.

## 3. Pipeline

```
qjson crate                          qrender crate
───────────                          ─────────────────────────────────────────────
WDQS SPARQL ──► typed WikidataItem ──► synthesize ──► compose ──► density ──► FactoidPage (IR)
   (one query)   (Redis, 7-day TTL)    (cards from    (archetype   (page-level     │
                                        data shapes,   recipes)     consolidation)  ├─► textual.rs   text / markdown / wikitext / HTML fragment
                                        variant per                                 ├─► factoid.rs   visual page (Askama + Codex tokens)
                                        card: plan.rs)                              └─► JSON as-is   server /api route
```

### 3.1 Data layer (`qjson`)

One SPARQL query fetches everything: labels, descriptions, all
statements with their qualifiers, ranks, value-node details (time
precision, quantity units), and — importantly for the visual layer —
the P18 image of every *referenced* item (`statementValueImage`), which
is what lets chips and timeline events carry thumbnails without extra
queries.

Key decisions:

- **Statement identity is the statement node URI.** WDQS returns one
  row per (statement × qualifier); rows are grouped back into
  statements by node URI, never by label (a bug class in the older Go
  tool this replaces: two statements with the same value label used to
  merge).
- **The label-service chain is `{lang}, mul, en`.** Wikidata is
  migrating language-independent labels to the `mul` code; omitting it
  makes labels resolve to bare QIDs.
- **QID and language are validated before interpolation** into the
  query. Never bypass this.
- **Deprecated-rank statements are dropped** at transform time.
  Preferred rank survives into the model and drives "current value"
  selection later.
- **Redis is an optional cache** (7-day TTL, key `qjson:{qid}:{lang}`);
  any cache failure degrades to a direct WDQS fetch.
- Time precision is only available for statement values, not for
  qualifier values — consumers of qualifier dates must assume year
  granularity.

### 3.2 Synthesis (`qrender/src/cards/synthesis.rs`)

`synthesize()` turns the typed item into a flat pool of `Card`s. Card
kinds are **derived from value types**, not configured per property:

| Data shape | Card kind |
|---|---|
| Commons media value (audio/video detected by extension) | `Image` / `Gallery` |
| Coordinate value | `Map` (static Wikimedia Maps tile) |
| Quantity statements with point-in-time qualifiers | `StatSeries` (headline value = preferred rank, else latest) |
| Quantity on a config-declared scale (HDI, Democracy Index) | `Meter` (native `<meter>`) |
| Lone plain single-valued property | `Stat` |
| Everything else | `Facts` — labeled rows of rich values (item chips with thumbnails, links, text) |

Property grouping comes from `groups.toml`, and grouping means
grouping: a group's non-visual properties become labeled rows of **one
`Facts` card** (father/mother/spouse/child are one family card, not
four fragments), while its images pool into one gallery. Ungrouped
properties each get their own card, ordered by PID. External
identifiers are suppressed for human-facing output.

Statement order inside a row is editorial: preferred rank first, then
statements without an end-time qualifier, then history chronologically
by start time — so "country: France" leads and wartime occupations
follow in order, none dropped. Preferred-rank chips carry
`current: true` and render with emphasis. For media the rule is
stronger: when any image statement is preferred, the others are
treated as superseded variants (old flags, alternate crops) and only
the preferred ones render.

Two shape rules refine quantity handling: point-in-time quantities
whose **years are distinct** form a time series; when the years
collide and other qualifiers vary, they are parallel measurements
(social media followers per platform), grouped by qualifier signature
into labeled rows — a chart labeled "2021, 2021, 2021" explains
nothing.

Qualifiers are never dropped, but they are typed before display:
start-time/end-time/point-in-time become a structured `TemporalSpan`
rendered at year granularity — "Emma Portner (2018 – 2021)", never
"start time: 2018-01-01, end time: 2021-01-01" — and the remaining
qualifiers render as `(label: value)` notes, because they carry
essential context (ordinals, roles) — especially for LLM consumers.
The span's `ended()` drives current-vs-former presentation.

Two content-consumption rules avoid duplication with the header: P18's
preferred (else first) image becomes the page hero, consuming the
property when that leaves it nothing else to show; a single-statement
hero-emblem property (person: P109 signature) is consumed the same way.

Each card then gets three resolved attributes:

- **icon** — a Material Symbols name from the vendored set (per-PID
  config → group config → kind default); the page embeds a tree-shaken
  sprite of exactly the symbols it uses.
- **variant + size** — see presentation planning below. Only the
  `sort` weight is configurable; cards are sorted by it in Rust.
- **tier** — `standard` or `footnote`. Footnote properties
  (`footnote = true` in `groups.toml`: categories, WikiProject
  maintenance, topic templates, focus lists, described-by-source, …)
  are real data but curation noise; they sort last in every backend
  and collapse behind a `<details>` disclosure on the factoid page.

### 3.2b Presentation planning (`qrender/src/cards/plan.rs`)

Every card gets a **variant** — its visual treatment — chosen from a
census of what the card actually holds (value count, images, temporal
spans), never per property or per archetype: a spouse, an employer,
and a citizenship with the same data shape get the same treatment, on
any item. Item cards split into `portrait` (1–2 items, at least one
pictured — the image fills the card), `fact-line` (1–2 plain values,
one compact line), `tile-strip` (3+ items, mostly pictured — big
scroll-snapped tiles, three visible), `current-history` (ended spans
stack as quiet lines under what holds now), and `chip-list`. The other
kinds map one-to-one (`media-full`, `gallery-strip`, `stat-block`,
`trend`, `gauge`, `map-panel`, `timeline`, `facts-table`,
`indicator-table`).

Sizes follow the **fill contract**: each variant declares a fixed grid
size from a small catalog and guarantees to fill it — images stretch
(`object-fit: cover`), strips clip and scroll, text clamps. No stage
predicts rendered height (Rust cannot know fonts and wrapping; a dense
grid with guessed row spans is exactly the wasted-space bug this
replaced). Sizes are deliberately **not configurable** — config keeps
judgment (grouping, ordering, footnotes, meter scales), code keeps
geometry.

### 3.2c Density (`qrender/src/cards/density.rs`)

Per region (each section, the overflow grid, the footnotes), runs of
same-shape cards consolidate — a dense item like a country otherwise
produces dozens of individually fine cards that aggregate into an
unscannable wall:

- **3+ time-series cards** merge into one `indicator-table`: label,
  sparkline, current value per row. Fifteen identical bar charts bury
  the story one table tells.
- **2+ map cards** merge into one labeled coordinate list (static map
  tiles cannot mark multiple points; the extreme-point properties
  P1332–P1335 were four near-identical country tiles).

Merging never crosses a region boundary and never drops data: every
series point and coordinate stays in every backend. Separately, the
**visual page** caps any row at 12 values with a "+N" tag linking to
Wikidata — a 444-language enumeration is a wall, not a card — while
the JSON API and textual backends keep full lists.

### 3.3 Composition (`qrender/src/cards/compose.rs`, `archetypes.toml`)

Composition adds the editorial hierarchy the flat pool lacks. It is
driven by an **archetype**: which kind of thing the item is, hence
which recipe applies.

**Resolution** (`archetype.rs`) is two-stage and needs no extra
queries:

1. **P31 match.** Curated instance-of QIDs per archetype (`Q5` →
   person; city/country/settlement QIDs → place). P31 itself is never
   rendered — it is ignored for display but read for classification.
2. **Signal scoring.** P31 values are too diverse for any curated list
   (Nairobi is "big city" Q1549591, not "city" Q515), so each
   archetype also declares signal properties; four or more present
   selects it. Below threshold the item stays `generic`.

Misresolution only affects presentation, and `generic` is always safe,
so the thresholds are deliberately conservative.

**Recipes** declare three things, all optional:

- **Hero facts** — properties surfaced inside the page header: a
  formatted date range ("1952 – 2001"), a tagline joined from value
  labels (occupations, preferred-rank first), an emblem image
  (signature).
- **Sections** — ordered, icon-headed regions that *claim* cards from
  the pool by source PID. First section wins; a card is never
  duplicated; sections that claim nothing collapse; whatever remains
  unclaimed renders in the overflow grid. This is deliberately a
  placement mechanism, not a data mechanism — it cannot lose content.
- **Timeline** — the one cross-property synthesis: a section may
  request a chronology built from its configured properties, merging
  direct Time values (birth, death, inception) with statements dated
  by start-time/point-in-time qualifiers (awards, positions,
  employers), each event carrying the referenced item's thumbnail.
  Dates display as years (qualifier precision is unavailable; full
  dates remain on the property cards). Below `min_events` the timeline
  does not render at all — the degradation contract's "no half-empty
  showpieces" rule.

The composed page is:

```
FactoidPage
├── header: label, description, hero image, hero facts
├── sections[]: icon + machine name + claimed cards (timeline first)
├── overflow[]: unclaimed standard cards (the bento grid)
└── footnotes[]: curation meta, collapsed
```

`all_cards()` iterates that in reading order; the textual backends and
the sprite builder walk it, so their output is a flattening of the
same editorial decisions.

Notably absent: a claim/ownership system between the timeline, hero
facts, and origin cards. The reference designs themselves repeat
information (dates in the hero *and* a born/died card; awards on the
timeline *and* as chips) — reinforcement, not redundancy — so the only
consumption rules are the two single-statement header cases above.

### 3.4 Presentation

**Factoid page** (`factoid.rs`, one Askama template, `factoid.css`):

- The layout is a fixed-track bento grid (6 columns, stepping to 4 and
  2 by container width); each card spans its `--cols`/`--rows` and
  `grid-auto-flow: dense` packs the holes. True masonry is a
  `@supports` progressive enhancement for browsers that ship it.
- Cards are `inline-size` containers; their internals (fact-row
  columns, stat font size) respond to the span the grid gave them, not
  the viewport, so the same card works at any size.
- The stylesheet keys card layout off `data-variant` (set from
  `plan.rs`), not off structural `:has()` guessing. Images do the
  explaining wherever possible: portrait cards bleed the picture to
  the card edges with the label beneath (a spouse's face, a school
  building) — encyclopedic, rabbit-hole navigation. Row-level chips
  keep compact round thumbnails.
- **One surface level per card**: chips are soft content tags
  (background, no border); the card border is the only box. This is a
  hard rule — no boxes inside boxes.
- Theming keys off `<body data-archetype="…">`: person and place get a
  serif display title; a place with a hero image gets a full-bleed
  backdrop with the title on a scrim — guarded by `:has()` so an item
  without an image keeps the compact header. Theme differentiation is
  intentionally limited to typography and hero treatment; accent-hue
  overrides were rejected because too many Codex token values derive
  from the progressive hue to override safely.
- Everything self-contained: Codex design tokens (light + dark) and
  the stylesheet are embedded; the icon sprite contains only the
  symbols the page uses; images come from Commons `Special:FilePath`
  thumbnails and map tiles from `maps.wikimedia.org`. No other hosts;
  the only script is the inline popover-morph enhancement.

**Textual backends** (`textual.rs`): four serializations of the same
IR walk — `# / ##` headings for text and markdown, `== … ==` for
wikitext, a plain semantic HTML fragment for embedding. They exist for
LLM/chat consumption, so qualifier notes and footnote content are
included (last), not stripped.

**Server** (`qrender-server`): axum; `/{lang}/{qid}` renders the
factoid page, `/api/{lang}/{qid}` serves the composed IR as JSON so
other frontends can build their own presentation over the same
editorial layer. Invalid input → 400; WDQS failure → 502.

## 4. Configuration model

Two files, both embedded at compile time (`include_str!` — edits
require a rebuild), both keeping the convention of a label comment on
every PID/QID:

**`groups.toml`** — property-level judgment, archetype-independent:

```toml
[groups.population]          # semantic grouping + shared icon/order
icon = "groups"
pids = ["P1082", ...]

[properties.P1082]           # per-PID overrides
icon = "groups"
sort = 100                   # page-order weight (sizes are not
                             # configurable - the variant owns them)
[properties.P31]
ignore = true                # structural: never a card
[properties.P5008]
footnote = true              # curation meta: collapsed, not deleted
[properties.P1081]
meter = { min = 0.0, max = 1.0, low = 0.5, high = 0.8, optimum = 0.95 }
```

**`archetypes.toml`** — item-level judgment:

```toml
[archetypes.person]
p31 = ["Q5"]                 # direct classification
signals = ["P569", ...]      # shape-scored fallback (threshold 4)

[archetypes.person.hero]
dates = ["P569", "P570"]     # → "1952 – 2001"
tagline = "P106"
emblem = "P109"

[[archetypes.person.sections]]
name = "career"              # machine name; icon carries the meaning
icon = "event"
pids = ["P106", "P69", ...]  # claims cards whose sources intersect

[archetypes.person.sections.timeline]
pids = ["P569", "P166", ...] # dated statements merged chronologically
min_events = 4               # below this, no timeline at all
```

Adding an archetype is a config exercise: classification, optional
hero, sections. No code changes unless it needs a new card kind.

## 5. Degradation contract

These invariants define "graceful" and are what tests pin:

1. An item with only a label and description renders a clean
   header-only page.
2. A featured element (timeline, full-bleed hero) renders only above
   its content threshold; there are no half-empty showpieces.
3. Composition never loses data: every non-ignored card appears in a
   section, the overflow grid, or the footnotes.
4. `generic` is the default product, not an error path — the hero,
   tiers, and bento layout apply to every item.
5. DOM order = reading order = screen-reader order, in every backend.

## 6. Testing conventions

All tests run offline against checked-in WDQS fixtures
(`qjson/tests/fixtures/*.sparql.json`: Q42 Douglas Adams — the person
ceiling; Q3870 Nairobi — a mid-density place). Layers are pinned
separately:

- transform tests (qjson) pin the typed model;
- synthesis/composition unit tests pin card derivation, tiers,
  archetype resolution, hero facts, timeline, and section claiming;
- `insta` golden snapshots pin every backend's full output
  (`ir_backends.rs` for the four textual formats, `factoid.rs` for the
  HTML body — the head is excluded because it embeds vendored token
  CSS).

After an intended output change: `INSTA_UPDATE=always cargo test`,
review the snapshot diff, commit it. The snapshot diff *is* the
review artifact — e.g. the composition refactor was validated by the
textual snapshots not changing at all.

To refresh a fixture, rebuild the query from `sparql.rs` against live
WDQS (see the fixtures' git history for the procedure).

## 7. Boundaries and non-goals

- **No prose.** Wikidata has no running text beyond the description;
  lead paragraphs would require Wikipedia extracts, which is a
  different data contract, deliberately out of scope.
- **No string parsing for visualization.** Data encoded in value
  labels (e.g. model sizes inside software-version strings) is not
  mined; only typed values drive card kinds.
- **No client-side rendering.** Content never depends on JavaScript;
  the one inline script is cosmetic (the popover view transition) and
  the page is complete without it.
- **No taxonomy walking.** Archetype resolution reads P31 literally
  plus data shape; following P279 subclass chains would cost extra
  queries for a presentation-only decision.
- **Section titles are not (yet) localized text.** The icon-only
  approach keeps the zero-i18n invariant; if named sections ever
  become necessary, the planned mechanism is naming sections by QID
  and resolving titles through the same label service — not a
  translation file.
