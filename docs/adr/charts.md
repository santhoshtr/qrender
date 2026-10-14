# Charts: dated measurements as ECharts line charts

Status: done (2026-10-14), with the changes of §6 and the notes of §7.

## 1. The problem

India (Q668) has many measurements dated by point in time (P585):

| Property | Points | Today |
|---|---|---|
| P1082 population | 60 | not in a spread (the country layout has no demography spread) |
| P1081 Human Development Index | 32 | row in "more" |
| P2250 life expectancy | 23 | row in "more" |
| P4841 total fertility rate | 17 | row in "more" |
| P11593 Inequality-adjusted HDI | 12 | row in "more" |
| P8476, P8477 BTI indexes | 9, 8 | row in "more" |
| P8763 birth rate | 4 | row in "more" |
| P10091 death rate | 4 | row in "more" |
| P6343, P6344, P2131, P8328, ... | 2 | row in "more" |

Four defects:

1. A series needs its property in the `series` role list (6 PIDs). A
   dated measurement of another property is a row, though its data has
   the shape of a series.
2. The values of a row are in the order of Wikidata (2021, 2019, 2022,
   2020), not in the order of time.
3. The numbers have no meaning without the property description. Birth
   rate has no unit; only the description says "per 1,000 population".
4. The series is CSS columns (`--f` fractions, labels on some points). It
   has no axis and no scale, so a small change looks large.

The country layout has no demography and no economy spread, so all of
these go to "more". That is an editorial gap; see §3.5.

## 2. Decisions

### 2.1 A series by the shape of the data, not by a list

The block rule `series` becomes:

```toml
[[block]]
id = "series"
when = { type = "Quantity", distinct_years = true }   # 3 or more distinct P585 years
make = "series"
```

A quantity dated by point in time is a measurement by the Wikidata model,
so the shape is enough (as a coordinate on the Earth is enough for a
map). The `series` role goes from `vocab.rs` and `layouts.toml`; the
scale spread uses the same predicate. Colliding years (one statement for
each sex, two sources) stay as rows, as now.

### 2.2 Fewer than 3 years: a row in the order of time

A row of dated values sorts them by year, the newest first: the reader
wants the current value first. Only dated values (P585) change order.

### 2.3 The property description says what the number is

For each series block and each row of dated quantities, the localize step
adds the description of the property to the view (the mechanism of the
`described` values). The page shows it under the title of the block; the
Markdown shows it after the property label:

> **birth rate** (the total number of live births per 1,000 population):
> 2019: 17.049, 2020: 16.572, ...

This is in the text layer (one language), so it needs no new fetch: the
fetcher gets the descriptions of all the entities already.

### 2.4 The chart: ECharts, progressive enhancement

The server renders the content; the script draws it.

- **HTML (layer 3, no JS):** a `<qr-chart>` element holds a real
  `<table>` (year, amount, edit and provenance marks) in a box of fixed
  block size that scrolls. This is the page without JavaScript, and the
  data for screen readers.
- **`assets/chart.js`** defines `qr-chart` (light DOM, as `qr-sounds`).
  When a chart comes near the viewport (`IntersectionObserver`), it loads
  ECharts once, reads the table, and draws a line chart in the same box.
  The table stays in the page, visually hidden. The script only reads
  what the server rendered: no composing, no choice of data.
- **No layout shift:** the box has the same block size with and without
  the chart.
- **Language:** numbers and years by `Intl` with the page locale (as
  `intl.js`); for `dir="rtl"` the time axis is inverted.
- **Design:** colours from the Codex tokens (read with
  `getComputedStyle`); the SVG renderer (sharp, prints, small DOM).
- **If ECharts does not load,** the table stays. The page is whole.

Not decided by the script: which properties are a series, the order, the
labels, the description. All of these are in layers 2 and 3.

### 2.5 Vendoring ECharts

- Pages refer only to Wikimedia hosts and the service. ECharts is
  vendored in `qrender/assets/vendor/` with its licence (Apache-2.0) and
  served by the existing versioned route:
  `/design/{version}/echarts.min.js`, with a long cache time.
- First step: the official minified build of the current release, with no
  build step. It is about 1 MB (about 350 KB with gzip). It loads only on
  a page that has a chart, after the chart comes near the viewport, so it
  does not block the render. A custom build (line chart, grid, tooltip,
  SVG renderer) can make it smaller later, if the size is a problem.
- `chart.js` gets the URL from an attribute that the server writes
  (`data-lib`), so the primitive declares only `JS` (the primitive-assets
  rule). A page made by the CLI has no server: it shows the tables.

### 2.6 The composition loses its geometry

`Point.fraction` and `Point.labelled` were for the CSS columns. The chart
makes its own scale, so they go. The composition schema changes
(`composition-0.4`).

## 3. Open questions

1. **Birth rate and death rate in one chart?** They have the same unit and
   the same scale; together they show the natural increase. This needs an
   editorial group in the configuration (properties that share a chart).
   Recommendation: one chart for each property first, then a
   `[roles.chart_together]` table as a second step.
2. **Line or columns?** Recommendation: a line for all series. Time is
   continuous, and a line shows a small change at its true size, with an
   axis that does not start at zero.
3. **Description only for measurements?** Recommendation: yes (series and
   dated rows). The descriptions of other properties ("country", "capital")
   add nothing.
4. **India fixture.** Recommendation: add Q668 (en, ml) with
   `qrender fetch`, as the validator of this work.
5. **Spreads for the country layout** (editorial, your decision):
   `demography` (scale kind: P1082, P1539, P1540, P6343, P6344, P8763,
   P10091, P4841, P2250, P6897, P6499, P6498) and `development`
   (facts: P2131, P2132, P1081, P11593, P1125, P8476, P8477, P8328,
   P11547). Without them, the charts are in "more", collapsed.

## 4. Changes

- `rules.toml`: the series rule without the role. `layouts.toml`, `vocab.rs`,
  `config`: remove the `series` role. `compose/spreads/scale.rs`: the
  predicate instead of the role.
- `primitives/fact.rs`: dated values by year, the newest first.
- `primitives/series.rs`: no `fraction`, no `labelled`; `JS` declared.
  `templates/primitives/series.html`: `<qr-chart>` with the table.
  `design/primitives/series.css`: the fixed box; the table visually
  hidden when the chart is drawn.
- `assets/chart.js`; `assets/vendor/echarts.min.js` and its licence.
- `localize`: the description of the property of each measurement block.
  `render/markdown`: the description after the label.
- `app/src/bin/qrender-server.rs`: serve the vendored file by the design
  route.
- Fixtures and snapshots: India; schemas regenerated.

## 5. Checks

- India: birth rate, death rate and the other dated measurements are
  charts, with the description under the title.
- Nairobi and Paris: population is a chart; the page heights change only
  by the series boxes.
- Without JavaScript: every chart is a table that scrolls in the same box.
- ECharts loads only on a page with a chart, and only once.
- `pages.rs`: a page has `chart.js` only when it has a series; a page
  refers to no host other than Wikimedia and the service.
- Arabic or Hebrew page: the time axis goes from right to left.

## 6. Approved, with changes

- **2 distinct years are a series**, not 3. A chart with 2 points shows
  that more data can come: it tells the reader that the data is scarce
  and invites an edit. The fallback table is then also clean.
- **No schema version change.** Nothing is delivered to users yet: the
  composition stays `composition-0.3`; the schema files are regenerated.
- **Birth rate and death rate:** two charts, side by side as siblings
  (next to each other in the same spread).
- **Line chart** for all series.
- **Description** only for measurements (series and dated rows).
- **India fixture:** Q668 in en and ml.
- **Country layout:** add the `demography` and `development` spreads of
  §3.5. The other items of "more" go to their own spreads in a later plan.

## 7. Implementation notes

- ECharts 6.1.0, the official "common" build (SVG renderer, line chart,
  tooltip, legend): 715 KB, 240 KB with gzip. The "simple" build has no
  tooltip and no SVG renderer.
- The page gives the design base on `<html data-design>` (a page that
  the server sends); `chart.js` loads `{base}/echarts.js`. A page from the
  CLI with inline styles has no base: it shows the tables.
- The table is in a wrapper (`.data`), and the wrapper is visually hidden
  when the chart is drawn. A table box keeps the height of its rows, so a
  hidden table took the place of the other blocks in a stack's columns.
- The chart SVG is `dir="ltr"`: the options turn the axes for a
  right-to-left page, and the page direction must not turn the text again.
- With 6 points or fewer, the time axis has a label at each year of the
  data, and each point has its number.
- A country is its own country (P17): the scale block does not compare
  an item with itself.
- The column header of the years is the label of "point in time" (P585):
  no new message.

Known issues, not in this change:

- The text cache key (`text/0.1:{qid}:{lang}`) does not change when the
  concepts of the configuration change. A cached text has no label for a
  new spread concept until the cache entry expires.
- The server does not compress responses.
- The hidden edit marks of the succession and set blocks are outside the
  page edge: the page scrolls horizontally by 6 px.
