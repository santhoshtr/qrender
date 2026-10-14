# Names: the names of an item in many languages, one choice of language

Status: approved (2026-10-14).

## 1. The problem

India has names in many languages: official name (7 languages), short
name (5), name (13), native label (9), demonym (34, with gender and
number). Each property is a long list of rows in "more". A pronunciation
qualifier (P443) shows as a raw Commons URL.

## 2. Decisions

### 2.1 One block, one choice

The monolingual-text properties of a names spread are one block, `names`,
with one choice of language for all of its rows: "India in language X".
The block is made at the place of its first such property in the spread
(as a split is), and claims the text statements of those properties.

### 2.2 The groups of languages

The renderer (it knows the page language; the composition does not)
makes one group for each language of the values:

- Order: the page language, `mul`, `en`, then the other languages, the
  language with the most properties first, then by code.
- A row of a group: the values of the property in that language. Without
  one: the values by the chain "the language of the group, the page
  language, `mul`, `en`", marked as a fallback, with the language and the
  action to add the value. Without any: the row is not in the group.
- The page language always has a group, the first, also when all its
  rows are fallbacks.

### 2.3 With and without JavaScript

- The server renders all groups. Without JavaScript the groups are in a
  box that scrolls; the fallback rows of all groups but the first are
  hidden (`qr-names:not(:defined)`): the full list has each value once.
- `names.js` defines `qr-names`: it shows the choice (a `<select>`) and
  the group of the chosen language, the first at the start. It decides
  nothing: the groups and their rows come from the server.
- Language names: the server writes the code; `intl.js` writes the name
  of the language in the page locale (`Intl.DisplayNames`). Without
  JavaScript, or for a code that the browser does not know, the code
  stays. The language note of every monolingual text gets the same.

### 2.4 The pronunciation qualifier: a play button

A string qualifier that is a Commons file (`Special:FilePath`) is a file
qualifier. An audio file is a play button: a link to the file (without
JavaScript the browser plays it), and `play.js` (a page script) plays it
in the page. Other files are links to their Commons page.

### 2.5 The spread

The country layout gets a `names` spread with the properties of the
settlement's, and "name" (P2561).

## 3. Changes

- `schema/value.rs`: `QualifierPart::File`; `localize/format.rs`:
  `Part::Audio`, `Part::File`; their renderings; `assets/play.js`.
- `primitives/names.rs`, its template, CSS and `assets/names.js`;
  `Primitive::Names`; the facts spread builder; `Ui::name_groups`.
- `assets/intl.js`: language names.
- `layouts.toml`: the country `names` spread.
- Tests: the groups (order, fallbacks, no row without a value in the
  chain); the file qualifier; snapshots.
