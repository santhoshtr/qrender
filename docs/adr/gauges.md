# Gauges: a proportion or an index, in its row

Status: approved (2026-10-14).

## 1. The problem

A bounded number alone says little: "Gini coefficient 35.7", "Happy
Planet Index score 36.4", "unemployment rate 4 percent". The reader must
know the scale.

## 2. Decisions

### 2.1 Two kinds, two pictures

- **A proportion** is a part of a whole: literacy 74 %, unemployment 4 %,
  water 9.5 % of the area. A filled track from 0 to 100 %: the same
  language as the bars of a split (the value and the rest).
- **An index** is a position on a scale: Gini coefficient 35.7 of 0 to
  100, HDI 0.633 of 0 to 1. A marker on a track, with the ends of the
  scale. No fill: a fill says "an amount of"; for an index, the position
  is the meaning.
- No good or bad colours: a higher Gini is "worse", a higher HDI
  "better", but a colour for that is an editorial (for the Democracy
  Index, a political) judgment. The track has the colour of the
  dimension of the spread; the description of the property says what the
  number is.

### 2.2 Where the scale comes from

- **Wikidata.** The range constraint of the property (P2302 =
  Q21510860, minimum P2313, maximum P2312). The fetcher reads it for the
  quantity properties of the item: `Content.ranges`.
- **A proportion** needs no configuration: the unit percent (Q11229) and
  the range 0 to 100. Inflation and VAT rates are percentages without
  that range: they are rates, not parts, and have no gauge.
- **An index** is listed in the configuration (`[indexes]`), because a
  range constraint can be a sanity bound, not a scale (suicide rate 0 to
  400, total fertility rate 0 to 100). The range is the constraint of the
  property, or a range in the configuration for a property without one
  (Happy Planet Index: 0 to 100).

### 2.3 In the row

The gauge is under the value of its row: the number stays the content and
the sheet stays compact. Only a row of one value has a gauge. A value
outside its range has none. The series (HDI over time) stay line charts:
the change is what they show.

## 3. Changes

- `content`: `Range`, `Content.ranges`; `fetch/wikidata.rs`: `ranges`.
- `layouts.toml`: `[indexes]`; `config`: `IndexSpec` and its check.
- `wikidata.rs`: `PERCENT`.
- `primitives/fact.rs`: `Gauge`, `GaugeKind`, `gauge()`; the row
  template and `fact.css`; Markdown: an index gets "(scale 0–100)".
- Fixtures: `ranges` in the content fixtures (from a fresh fetch, only
  that key); the Nairobi responses recorded again.

## 4. Later

- Compare with other items: `.plans/future/contextualization.md`.
