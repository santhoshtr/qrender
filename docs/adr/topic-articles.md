# Topics: Wikipedia article cards

Status: done (2026-10-20).

## 1. The problem

The "... of topic" properties (P2184 history of topic, P2633 geography
of topic, P9241 demographics of topic, P8744 economy of topic) point to
an item about one part of the subject: "geography of Germany" (Q154124)
for Germany. Each one is the topic of its spread (`[topics]` in
`layouts.toml`).

The page showed a topic as a link to its item on Wikidata, at the end of
its spread. But the item is a Wikipedia article in many languages, with
a title, a description and a picture. The link did not show this.

## 2. Decisions

- **A card.** A topic that has an article is a card at the end of its
  spread: its picture, the Wikipedia logo and the host of the wiki, the
  article title, and the description of the item.
- **The article.** In the page language, else in English. This is the
  chain of the fallback article. The host ("en.wikipedia.org") and the
  `lang` of the title show the language.
- **No article.** A topic without an article in the two wikis stays a
  link to its item on Wikidata.
- **The link.** Only the title is a link. The credit of the picture is a
  button, and a button cannot be in a link, so the card is not a link.
- **Markdown.** A topic is a link to its article and the description:
  `[Geography of Germany](…) — geography of a country`.

## 3. Design

- **Fetch.** `Settings.topics`: the topic properties. `Text.articles`:
  the article of each topic item, by QID, as a `LangText` (the title,
  and the language of its wiki). One `wbgetentities` request
  (`props=sitelinks`) for 50 items; the fetcher selects the wiki of the
  page language, else `enwiki`. `FETCH_VERSION` 15 → 16.
- **Compose.** A topic is `described`: its label gets the description.
  A topic is not a class value, so it keeps its picture (P18). A topic
  without P18 is "unpictured", and gets a page image from its article
  (an existing step).
- **Localize.** `View.articles`: a language table, `Qid → Link` (the
  title, the language, the URL), for the topics of the composition.
- **Render.** `Ui::topic_articles(spread)`: the topics that have an
  article, each with the value as shown (picture, description). The
  part `parts/topic_article.html`, rendered by `Ui::topic_article`.
  `prose_links` keeps only the topics without an article.

## 4. Notes

- A request with `sitefilter` needs a new parameter of `entities`. A
  page has four topics or fewer, so the fetcher gets all their sitelinks
  and selects the two wikis.
- Most topic items are classes ("history of Germany" is a class of
  events). The composer shows no picture for a class value, because the
  picture shows only an example. A topic is an article, so the composer
  clears `class` for it.
- The fingerprint of the default settings changed (the new `topics`
  field); its test has the new value.
- The replay test needs one new response: the sitelinks of Q7805834
  (timeline of Nairobi history). It was recorded to a temporary
  directory, and only that file was copied, so the other responses stay
  the same.
- Fixtures: `articles` added to the 11 text fixtures that have topics.
  `ml/Q668` has three articles in Malayalam and one in English.
- Paris (Q90): its economy topic (P8744) is a row of the sheet of the
  economy spread, not a topic: a block claims it first. That was so
  before this change.
