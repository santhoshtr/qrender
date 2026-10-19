# Astronomical layout: a place for every property

Status: done (2026-10-19), with the notes of §9.

## 1. The problem

On the page of Earth (Q2), 34 properties were in "more" (collapsed,
"+34"). The `astronomical` layout listed only the physical and the orbit
properties; the relations spread took item properties whose values have
pictures. Every other statement went to "more".

"More" makes a plain row of each property. So it also lost the shape of
the data: the 43 dated populations were a list of 43 lines, not a chart.

The Moon (Q405, a fixture) had 12 properties in "more", and 5 of them
were the same as Earth's.

## 2. What was found in the code

1. **The history spread ignores its `pids`.** `spreads/history.rs` makes
   only the timeline (significant events). So "inception" (P571) has no
   place in any layout: it is in "more" for India, Paris and Nairobi too.
2. **An extreme point needs a map.** The property rule `extreme-point`
   skips P1332–P1335 and P5140: the map of P625 shows them as markers.
   Earth has no P625, so it has no map, and the skipped points go to
   "more". If the skip goes away, the block rule `coordinate` makes a
   map of each point (a map of the North Pole).
3. **A second scale spread repeats the peers.** `peers::build` does not
   depend on the spread and claims nothing. Population must go in a
   `facts` spread: there, the block rule `series` makes the chart.
4. **The topic "geography of topic" (P2633) wants the spread id
   `location`.** The astronomical layout has no such spread.
5. **A names spread takes many languages by itself.** In a spread of the
   `names` dimension, `names::at` makes one names block of all the
   properties whose values are monolingual texts (demonym, 51 values).

## 3. Where each property goes

Earth's 34, and the Moon's 12 (M). "New" is a new spread of the
astronomical layout.

| Property | Values (Earth) | Spread | Shown as |
| --- | --- | --- | --- |
| P2213 longitude of ascending node | 1 | orbit | row |
| P2248 argument of periapsis | 1 | orbit | row |
| P2325 mean anomaly | 1 | orbit | row |
| P11796 time of periapsis | 1 | orbit | row |
| P6259 epoch (J2000.0) | 1 | orbit | small value |
| P2052 speed ("orbit of Earth") | 1 | orbit | row |
| P2147 rotation period | 2 | physical | row |
| P2547 perimeter | 2 | physical | row |
| P2069 magnetic moment | 1 | physical | row |
| P1419 shape | 3 | physical | row |
| P6591 maximum temperature record | 1 | physical | row, next to temperature |
| P7422 minimum temperature record | 1 | physical | row |
| P1589 lowest point | 3 | location (new) | rows |
| P1332, P1333 northernmost, southernmost point | 1 each | location (new) | rows with 📍 (no map, §4.2) |
| P2633 geography of topic | 1 | location (new) | topic |
| P625 coordinate location (M) | — | location (new) | map on Earth; row on another globe |
| P276 location (M) | — | location (new) | — |
| also P610 highest point | — | location (new) | moves from relations |
| P1549 demonym | 51 | names (new) | names block |
| P138 named after | 4 | names (new) | names block (its values have languages) or row |
| P1814 name in kana | 1 | names (new) | row |
| P528 catalog code | 1 | names (new) | row (question 7.2) |
| P913 notation (Earth symbol) | 1 | names (new) | small value |
| P487 Unicode character | 9 | names (new) | row |
| P163 flag (flag of the Earth) | 1 | names (new) | picture |
| P1082 population | 43 | demography (new, facts) | series chart |
| P2573 number of out-of-school children | 17 | demography (new, facts) | series chart |
| P571 inception | 1 | history | row (§4.1) |
| P576 dissolved ("unknown value", future of Earth) | 1 | history | row |
| P575 time of discovery ("unknown value") | 1 | history | row |
| P2579 studied by (M) | 4 | relations | values |
| P1552 has characteristic (M) | 5 | relations | values |
| P461 opposite of | 2 | relations | values |
| P1557 manifestation of | 1 | relations | value |
| P366 has use, P1542 has effect, P1299 depicted by (M) | — | relations | values |
| P2888 exact match | 4 | evidence | curation |
| P6363 WordLift URL | 1 | evidence | curation |

Left in "more", on purpose: the Moon's P376 (located on astronomical
body: the Moon itself) and P3872 (patronage: 12, the people who walked
on it). They are odd data; "more" is the place for them.

The order of the spreads:

```
identity, physical, orbit, location (new), demography (new),
names (new), relations, history, media, web, more, evidence
```

A spread with no content is not on the page, so the Moon, Mars or a star
gets no empty demography spread.

## 4. Changes in the code

Two small changes. The rest is configuration.

### 4.1 The history spread shows its listed properties

`spreads/history.rs`: after the timeline, a row (`fact::build`) for each
listed property that has free statements, in the sheet slot, as the
scale spread does. A topic property (P2184) is not a row: the spread
takes it as its topic.

Then each layout lists the dates of its subject in its history spread:

- astronomical: P571, P575, P576;
- settlement and country: P571, P576 (India: inception 1947; Paris,
  Nairobi: their founding dates). These leave "more" too.

The person and film layouts have their dates in other spreads; no change.

### 4.2 An extreme point without a map is a row

- A property predicate `map`: the spread lists P625 and the item has a
  P625 on the Earth. (`PropertyFacts` gets the spread's properties.)
- `rules.toml`:

  ```toml
  [[property]]
  id = "extreme-point"
  when = { role = "extreme_point", map = true }   # markers on the map
  make = "skip"
  ```

  and a block rule before `coordinate`:

  ```toml
  [[block]]
  id = "extreme-point-row"
  when = { role = "extreme_point" }   # no map: a row, with its pin
  make = "fact"
  ```

The row has the 📍 pin of a coordinate, so the reader still gets a map
in a dialog (task 2).

### 4.3 Configuration

- `layouts.toml`, astronomical: the new spreads `location`, `demography`
  and `names`; the new properties of `physical`, `orbit`, `relations` and
  `history`.
- `[evidence] pids`: P2888 exact match, P6363 WordLift URL. This is for
  every layout: they are links to other databases, not facts for a
  reader.
- Settlement and country: P571, P576 in their history spreads.

## 5. What changes on other pages

- India, Paris, Nairobi, the village: inception (and dissolved, if any)
  moves from "more" to the history spread. A country or a settlement with
  P2888 or P6363 loses that row from "more".
- The Moon: 10 of its 12 properties leave "more".

## 6. Tests

- A fixture for Earth: `qrender fetch --qid Q2 --lang en --out fixtures`
  (and `ml`). Earth has things no fixture has: coordinate pins (the
  pin's map dialog of task 2 has no snapshot), a preferred 3D model, a
  video, many dated series. Add Q2 to `PAGES` in `pages.rs`.
- A unit test of the history rows: a listed property is a row; a topic
  property is not.
- A unit test of the `map` predicate: with P625 on the Earth, the points
  are skipped (markers); without it, they are rows.
- A test that Earth's "more" spread is empty, or holds only the
  properties named in §3 as left on purpose.
- Review the snapshot diffs of all pages (§5).

## 7. Decisions

7.1 **Order:** surface and people first: identity, physical, orbit,
location, demography, names, relations, history, media, web, more,
evidence.

7.2 **Catalog code (P528)** is in names: for a star, its catalog codes
are its designations.

7.3 **"Unknown value" dates** are rows in history, next to inception.

7.4 **Population of the Earth:** a demography spread (facts) in the
astronomical layout; empty, so not shown, for the other bodies.

## 8. Commits

1. History: a row for each property the layout lists (§4.1).
2. Rules: an extreme point without a map is a row (§4.2).
3. Layouts: a place for every property of an astronomical body (§4.3).
4. The Earth fixture (Q2, en and ml) and its tests (§6); this record.

## 9. Notes

- Earth has no "more" spread now; `compositions.rs` checks it. The Moon
  keeps P376 and P3872 in "more" (§3).
- The names spread lists pronunciation audio (P443), as the country and
  settlement layouts do. So the Moon's and the Earth's sounds block moved
  from the media spread to the names spread. It was not in the table of
  §3.
- The names spread has the prose concept "etymology" (Q35245): on Earth
  it shows the Etymology paragraph of the article.
- A lowest point that ended (Galathea Depth, 1952) is a "former" row,
  apart from the current ones: the rule `former-values`.
- `content/tests/fixtures.rs` forbade "unknown value" (SomeValue) in a
  fixture: the fixtures of the prototype had none. It is a value of the
  model, and Earth has two; the test now forbids only an unknown datatype
  (Other).
- Earth is a fixture for more than this work: the coordinate pins and
  their map dialogs, a preferred 3D model, a video, dated series.
- To do, outside this work:
  - the population chart of the Earth is from 4000 BCE to now on a linear
    axis: the curve is a line at the right edge (a log scale, or the
    years since 1800);
  - the year of inception reads "4540000000 BCE": a large year could
    read "4.54 billion years ago";
  - the person, film and generic layouts list no dates in their history
    spreads (a generic item with an inception still shows it in "more").
