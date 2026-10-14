# Splits: n parts of a total, as a stacked bar or one chart

Status: done (2026-10-14), with the notes of §7.

## 1. The problem

Some properties divide one whole: female and male population, literate
and illiterate population, urban and rural population. Today each part is
a row of its own, and the reader must add and divide to see the shares.

India (Q668):

| Split | Year | Parts | Sum | P1082 that year |
|---|---|---|---|---|
| sex | 2021 | 662,380,000 + 700,620,000 | 1,363,000,000 | none |
| literacy | 2011 | 763,638,812 + 447,216,165 | 1,210,854,977 | 1,221,156,319 |
| urbanization | 2011, 2021 | 377,106,125 + 833,748,852; 469,340,000 + 893,660,000 | 1,210,854,977; 1,363,000,000 | 1,221,156,319; none |

The parts add up to the census total. P1082 is from another source.

## 2. Decisions

### 2.1 A split is declared in the configuration

Wikidata has no relation that says "these properties add up to one
whole". That is an editorial fact, so it is data, in `layouts.toml`:

```toml
# Properties whose values in one year divide one whole: a split. The
# concept titles the bar.
[[splits]]
id = "sex"
concept = "Q290"       # sex
parts = [
  "P1539",  # female population
  "P1540",  # male population
]

[[splits]]
id = "literacy"
concept = "Q8236"      # literacy
parts = [
  "P6499",  # literate population
  "P6498",  # illiterate population
]

[[splits]]
id = "urbanization"
concept = "Q161078"    # urbanization
parts = [
  "P6343",  # urban population
  "P6344",  # rural population
]
```

A new split is a new table entry; no code changes. A load check: a part
is in one split only, and a split has two parts or more.

### 2.2 The total is the sum of the parts

The parts divide the whole by their definition, so the total is their
sum, and the shares add up to 100 %. P1082 is not used: in India it is
from another source, and it makes a difference of 10 million that does
not exist.

### 2.3 Which statements make a split

- One bar for each year in which every part has a value (P585).
- A year with two values for a part takes its preferred value, by the
  rule of series (`series_points`); without one preferred value, that
  year makes no bar.
- **Lines when every part is a series.** When the values of every part
  make a series (`series_points`: 2 years or more), the split is one line
  chart with one line for each part: a line shows the change better than
  bars. Urban and rural population of India are one chart with two
  lines. A part can have a year that the other part has not: each line
  has its own points.
- **Bars otherwise.** One bar for each year in which every part has a
  value. Statements that make no bar stay for the series and the rows of
  their property (claim once, nothing lost).

### 2.4 The bar

- A horizontal bar of 100 %, one segment for each part, in the order of
  `parts`. Under it, for each part: the colour key, the label, the share
  and the number ("female population 48.6 % · 662,380,000").
- HTML and CSS from the server: a segment has its share as its inline
  size. No JavaScript.
- With more than one year: one bar for each year, the year at the start
  of the bar, all on one 0–100 % scale.
- Colours from the Codex categorical family, by the position of the part,
  not by its meaning (no pink and blue for female and male). The
  saturated grade (`--border-color-option-*`): blue, orange, purple,
  yellow, pink, green. No red or maroon: they are the destructive
  colours. The labels are always shown, because in dark mode colour
  alone does not separate the parts.

### 2.4a The chart with lines

- The `series` primitive gets lines: one line for each property. A series
  of one property is a series of one line, as now.
- The table has a column for each line: year, urban population, rural
  population. A cell is empty when the line has no point in that year.
- `chart.js` draws one line for each column, with a legend. One line has
  the colour of the dimension (`--ink`), as now; several lines have the
  categorical colours of §2.4, in the same order as the bars, and the
  legend and the tooltip name each line.
- The title is the concept of the split ("urbanization"); under it, the
  description of each part property.
- The other values of a year (preferred rank rule) stay marks of their
  line.

### 2.5 The composition

A new primitive, `split`:

```rust
pub struct Split {
    pub id: String,
    pub concept: Qid,
    pub parts: Vec<Pid>,
    /// One bar for each year, the oldest first.
    pub bars: Vec<Bar>,
}
pub struct Bar {
    pub year: i64,
    pub total: f64,
    /// In the order of `parts`.
    pub values: Vec<Part>,
}
pub struct Part {
    pub amount: f64,
    /// amount / total, rounded to 4 places: data, as the ratio of a scale.
    pub share: f64,
    pub statement: StatementId,
    pub provenance: Provenance,
}
```

Markdown: "sex (2021): female population 662,380,000 (48.6 %); male
population 700,620,000 (51.4 %)".

The series changes from one property to lines:

```rust
pub struct Series {
    /// The split whose parts are the lines; None for one property.
    pub concept: Option<Qid>,
    pub lines: Vec<Line>,
    // statement, provenance, source, rule: as now
}
pub struct Line {
    pub pid: Pid,
    pub points: Vec<Point>,
}
```

Markdown for a chart with lines: one item for each line, under the
concept.

### 2.6 Where the composer makes it

A split crosses properties, so a block rule (one property) cannot make
it. A composer step, `splits(pids)`, runs in each spread builder (facts
and scale) after the series and before the rows: for each split whose
parts are all in the spread's `pids`, it builds the bars and claims the
statements. The block goes in the stack slot.

## 3. Not in this plan

- One chart for related series that are not parts of one whole (birth
  and death rates): a later table, when it is needed.
- Cross-tables: urban population 2011 has qualifiers for its female,
  male, literate and illiterate parts. They could become a second level
  of the bar.
- A stacked area chart (ECharts) for a split with many years: no fixture
  has one.

## 4. Before this: qualifier numbers without a label (done)

A quantity qualifier shows only its number: literate population shows
"763,638,812 2011 · 328,875,190 · 434,763,622". The reader cannot know
that the numbers are the female and male literate population. A
quantity qualifier must show the label of its property: "female
population: 328,875,190". This is a separate commit.

## 5. Changes

- `layouts.toml`: `[[splits]]`; `config`: the type and the load checks.
- `primitives/series.rs`: lines; its template, CSS, `chart.js` and
  Markdown.
- `primitives/split.rs`, `templates/primitives/split.html`,
  `design/primitives/split.css`; `Primitive::Split` in `vocab.rs`
  (`ALL`), `Block::Split`, the design asset table.
- `compose`: the `splits` step; the facts and scale spread builders.
- Tests: the split of India (sex, literacy: bars; urbanization: lines); a
  year without every part makes no bar; a part with one point and a part
  with two make bars only for the common year; config checks.

## 6. Checks

- India: two bars in demography (sex 2021, literacy 2011) and one chart
  with two lines (urbanization); `unshown` is empty.
- Paris: a sex bar (2017).
- The bar without JavaScript is the same as with it.
- 1280 px and 390 px; a right-to-left page (the bar starts at the right).

## 7. Implementation notes

- A share keeps the qualifiers of its statement but the point in time:
  the female and male parts of the literate population show under it.
  Without them, the bar hid data that the row showed.
- The split block is made at the place of its first part in the spread;
  the property then goes on with its statements that the split did not
  take.
- The colours of the parts are CSS tokens (`--part-1` to `--part-6` in
  `base.css`); `chart.js` reads them, so the colours are in one place.
- The tooltip of a chart with lines escapes the line names (labels from
  Wikidata): ECharts writes the tooltip as HTML.
- The new concepts are in the text fixtures of India and Paris (copied
  from a fresh fetch; the rest of the fixtures is not changed). The
  Nairobi responses are recorded again: the concept request changed.
- The cache of a text does not change with the concepts of the
  configuration (known issue): a cached text shows the id of a new
  concept until the entry expires.
