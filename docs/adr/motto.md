# Motto: an item and its words are one value

Status: approved (2026-10-15).

## 1. The problem

Two properties give the motto of an item:

- **motto (P1546)**: an item. It is the motto as a topic, with its own
  articles and sometimes a picture.
- **motto text (P1451)**: the words in one language. It often has a
  translation (P2441), a role (P3831: national motto, tourism slogan) and
  dates.

Editors often add both for one motto. The page then shows the same motto
two times ("In God We Trust" on the United States). No statement links a
motto text to its motto item: the motto items do not have their words,
and the qualifiers of the two statements do not agree (India and Canada
have no qualifiers).

## 2. Decisions

- **The match.** A text matches an item when the text is a label or an
  alias of the item in the language of the text, or in `mul`. The
  comparison ignores case ("E Pluribus Unum" and "E pluribus unum"). The
  match does not depend on the page language. It finds the motto of the
  United States (two), India, Canada and France.
- **The configuration** names the pairs: `[roles.text]`, item property ->
  text property (`P1546 = "P1451"`). The fetcher and the composer read
  only this table, so other pairs can use it.
- **The fetcher** gets the labels and the aliases of the item values of
  each pair, in the languages of the texts and in `mul` (one query). It
  keeps only the result of the match: `Content.text_items`, text
  statement -> item. The content has no labels. `FETCH_VERSION` 7.
- **The composer** joins each matched text to the first item statement
  with that item. The text is claimed. The item value shows the words
  (with their language) and the qualifiers of the text (dates,
  translation, role) in its note. The provenance of the value is the best
  of the two statements. `StatementRef.texts` names the joined
  statements.
- **No match: no change.** A text without an item (a tourism slogan) and
  an item without a text stay as they are. A wrong pair stays visible: an
  editor can see it and correct it (South Africa: the item "Unity in
  Diversity" and the text "ǃke e: ǀxarra ǁke").
- **Tourism slogans** do not match a motto item, so they stay in the
  "motto text" row. No other rule is necessary.

## 3. Changes

- `content`: `Settings.texts`; `sparql::names`; `Content.text_items`.
- `config`: `[roles.text]`.
- `compose`: the join; `Composer::statement` adds the words.
- `schema`: `StatementRef.texts`.
- Tests: the match (case, `mul`, alias); the join; snapshots (India).
