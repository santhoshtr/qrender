# Factoid icons by configuration

Status: done (2026-10-26).

## 1. The problem

Each composed item (a fact row, a box, a map card, a timeline, ...) has
an icon at the start of its label. Every item showed the same icon
(`puzzle`). The icon of an item must be a decision in a configuration
file, as the other decisions of the page are.

Codex does not have an icon for many properties (most measures: mass,
temperature, elevation). The page must supply them.

Earlier work:

- The `wx` prototype: `config.yaml` has `icon:` for about 45 PIDs;
  `icons.go` has 51 Material Symbols (24px) as SVG strings. An unknown
  name falls through as text.
- The first Rust code (commit `5750b97`) had 53 Material Symbols, a
  per-PID table and a sprite in each page. Default icons by kind of card
  gave many identical icons; they were removed.

## 2. Decisions

- **Codex first.** A Material Symbol (Apache-2.0) only where Codex has no
  icon. Of the `wx` icons, those that Codex has change to Codex (person
  → `userAvatar`, website → `browser`, open data portal → `database`).
- **One style of Material Symbols:** Outlined, fill 0, weight 600,
  optical size 20. Weight 400 had thinner lines than the Codex icons;
  weight 600 has approximately the same.
- **The same size as Codex.** A Material Symbol has an empty margin of
  2px at 20px; a Codex icon has almost none. The `viewBox` of each
  Material file is `48 -912 864 864`: 1px less on each side. No icon has
  a part in that 1px. `qrender/assets/icons/material/README.md` tells
  how to add an icon.
- **A closed vocabulary.** The `icons!` table in
  `qrender/src/render/icons.rs` names each icon and its file. A name in
  the configuration that is not in the table is a load error.
- **The configuration: `qrender/design/icons.toml`.** The icon of an
  item is the icon of its property; else the icon of the datatype of its
  value; else the fallback. A concept item that titles a block (a split)
  has a table of its own.

  ```toml
  fallback = "puzzle"

  [datatypes]
  Time = "calendar"
  GlobeCoordinate = "mapPin"
  Url = "link"

  [properties]
  P2067 = "weight"                  # mass

  [concepts]
  Q161078 = "location_city"         # urbanization
  ```

- **A default by datatype** only where the icon is correct for every
  property of the datatype: a time, a coordinate, a URL. The view has no
  datatype; the form of a value (`ValueShown`) gives it. A string can be
  an identifier, a string or a tabular data page: no default.
- **The fallback stays `puzzle`.** It shows that the item is a composed
  fact. Look at it again when more properties have icons.
- **A design decision, not a composition decision.** The file is in
  `design/`, and only the HTML renderer reads it. The composition and
  the Markdown do not change.

## 3. Changes

- `qrender/src/render/icons.rs`: the `Icon` enum, the types of
  `icons.toml`, the lookup, and the CSS of the icons.
- The stylesheet has one rule for each icon that the configuration
  uses: `.i-calendar { --icon: url("data:image/svg+xml,...") }`. It is
  cached by every page; a page has only the class names. A Material icon
  is approximately 0.6 KB of the stylesheet.
- `base.css`: `mask: var( --icon )`. The variable `--icon-puzzle` is
  removed: the fallback is an icon of the configuration.
- `Ui`: `icon(statement)` (property, then datatype), `property_icon(pid)`
  and `item_icon(qid)` give the class. Each label in the icon selector of
  `base.css` has its class (also the fallback): `fact-row`, `gallery`,
  `links`, `absent`, `map`, `succession`, `timeline`, `earlier`, `media`,
  `sounds`, `scale`, `split`, `series`, `network`, `works`. A block with
  more than one PID (succession, works) uses the icon of its first PID.
- Tests: the configuration loads; an unknown name is an error; each file
  in `assets/icons/material/` is in the table; on every fixture page,
  each label in the icon selector has an icon class (without it, the
  mask is a filled square).
- The first configuration: the 53 property rows of `wx/config.yaml`
  (the dates and coordinates come from the datatype), and three concepts
  (sex, urbanization, literacy).

## 4. Alternatives not chosen

- **The icon in the composition** (`Fact.icon`). Other front ends would
  get it from the JSON. But it puts a design decision in layer 2 and
  changes every composition snapshot. A front end can map PIDs to its
  own icons.
- **A sprite in each page** (`<svg><use href="#i-event">`). It costs
  bytes in each page. The stylesheet is cached once for all pages, and
  the mask gives the colour of the dimension.
- **Default icons by dimension or by kind of block.** The colour shows
  the dimension. Defaults by kind gave many identical icons before.
- **Material Symbols only.** One look for the factoid icons, but the
  page has Codex icons anyway (the actions), and the design system is
  Codex.
- **A `build.rs` that reads the icon folder.** The table is short, and a
  change to it is a reviewed change.
