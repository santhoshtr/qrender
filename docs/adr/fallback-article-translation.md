# The fallback article, and its translation section by section

Status: first iteration done (2026-10-19): commits ef53dc6, 64c2034,
0319715, 0350717; notes in §8. Iteration 2 (the editor in a dialog, §6)
done (7b48eed); notes in §9. Iteration 3 (the article in the page
language, §10) done.

## 1. The problem

Nairobi has no article in Gujarati. The page `gu/Q3870` shows the facts,
the lead of the English article, and in a spread whose concept matches a
section, the first paragraph of that section. The other sections are
links at the end of the page.

So the page shows only pieces of the article, and it does not show how a
person writes the article in Gujarati. The prototype must show both:

- the whole English article, collapsed so that the facts stay first;
- for each section, a way to translate it: machine translation (MinT),
  which a person corrects and saves.

## 2. Decisions

- **Placement.** A section that a spread matches stays in its spread
  (Geography with the map): its lead paragraphs, then its subsections,
  collapsed, by their titles. The sections that no spread matches are in
  one area at the end, before the references: each collapsed to its title;
  opened, its lead paragraphs and its subsections, collapsed. Notes,
  references and links (the `apparatus` titles) are left out, as now.
- **Only for a fallback article.** When the page language has its own
  article, nothing changes in this iteration. (Changed in iteration 3,
  §10: the page shows the whole article in the page language too.)
- **Translate, for each top-level section and for the lead.** A button at
  the end of the section heading: the Codex "language" icon
  (`assets/icons/language.svg`, inline, `currentColor`), no visible words,
  so there is nothing more to localize for now. Its name for screen
  readers is the existing message "Translate to ગુજરાતી", visually hidden
  (the language name from `intl.js`). One button opens a new translation
  and a saved one: no "Edit the translation".
- **The editor: side by side, in a dialog** (iteration 2). The English
  source, and the editable machine translation; stacked on a narrow
  screen. "Save" and "Discard". A dialog, not the page: the layout of the
  page does not limit the editor, and a dialog has room for more tools
  and information. **Every edit on this page is in a dialog.**
- **Save mimics a published edit** (iteration 2). The section on the page
  shows the translation in place of the English text (its title and the
  titles of its subsections too), marked as translated, with a note that
  it is not published. Nothing goes to a server, and a reload shows the
  English again. The same button opens the dialog again with the
  English source and the saved translation, not a new machine
  translation.
- **The machine translation: the browser calls cxserver.**
  `POST https://cxserver.wikimedia.org/v2/translate/{from}/{to}/MinT`,
  body `{"html": "…"}`, answer `{"contents": "…"}`. It allows every
  origin (checked), and it is a Wikimedia host.

## 3. What MinT does with HTML (checked, en → gu)

- It translates only the first top-level element: two `<p>` give one
  translated `<p>`. Each request is one element, so the script sends one
  block (a paragraph, a list, a heading) at a time, wrapped in
  `<section>`.
- A small paragraph takes about 1.5 s. The script sends the blocks of a
  section a few at a time (4) and fills the editor as they come.
- Links can be lost ("capital" lost its link). That is for the person to
  correct.

## 4. Design

### 4.1 Content (fetch)

- The article body from `action=parse&prop=text&parsoid=1`: JSON, so the
  HTTP client, its cache and record/replay do not change. Parsoid wraps
  each section in `<section>`, nested by level, with its heading in
  `div.mw-heading > h2…h6` (Nairobi: 53 sections, 980 KB).
- A new field of `Article`:

  ```rust
  /// The sections of the article, with their content: only for the
  /// fallback article of a page whose language has none.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub outline: Vec<OutlineSection>,

  pub struct OutlineSection {
      pub title: String,
      pub anchor: String,
      /// 2 for a section, 3 for its subsection, ...
      pub level: u8,
      /// The blocks before the first subsection: `<p>`, `<ul>`, `<ol>`,
      /// `<blockquote>`, as HTML without references and attributes but
      /// `href`.
      pub blocks: Vec<String>,
      pub sections: Vec<OutlineSection>,
  }
  ```

- Parsed with `scraper` (a dependency of the `fetch` feature): a tree is
  safer than regular expressions. Tables, figures, navigation boxes,
  styles and references are not kept.
- The items of all the links of the blocks, by the existing `links`
  (Nairobi: 851 titles, 18 requests of 50).
- Fetched only when the page language has no article: the cost is for
  the pages that show it.
- `FETCH_VERSION` 9 → 10.

### 4.2 Localize

- `article::rewrite` also keeps `p`, `ul`, `ol`, `li`, `blockquote`
  (without attributes). The links are rewritten as for the lead: to the
  item on Wikidata, with its label in the page language as a gloss.
- `SpreadView` gets `sections: Vec<ProseSection>`: the outline sections
  of its concepts. When it has them, the template shows them in place of
  `prose_text`.
- `View` gets `more_sections: Vec<ProseSection>`: the outline sections
  that no spread shows and that are not apparatus. When it has them, the
  page shows them in place of the `unmatched_sections` links.
- `ProseSection { title, anchor, href, lang, blocks, sections }`, the
  blocks cleaned.
- The view schema changes: regenerate the JSON Schema.

### 4.3 Render

- A part `parts/article_section.html`, rendered by a `Ui` method, which
  calls itself for the subsections (as the parts do; not a macro):

  ```html
  <qr-section class="article-section" data-from="en" data-to="gu">
    <div class="head">
      <h3 lang="en">Geography</h3>
      <button class="translate" type="button">Translate to
        <span data-lang-name="gu">gu</span></button>
    </div>
    <div class="source" lang="en">…blocks…
      <details><summary><h4>Climate</h4></summary>…</details>
    </div>
    <div class="translation" hidden>
      <div class="labels">English · <span data-lang-name="gu">gu</span>
        (machine translation: correct it)</div>
      <div class="target" lang="gu" contenteditable="true"></div>
      <div class="actions"><button>Discard</button><button>Save</button>
        <p class="saved" hidden>Saved on this page only: a prototype, not
        published.</p></div>
    </div>
  </qr-section>
  ```

  In the area at the end, the section is in a `<details>` whose summary
  is the head (the button is in the summary: to check that a click on it
  does not open the section).
- The server writes every word of the editor (messages in
  `i18n/en.json` and `qqq.json`). The script only fills the target and
  shows or hides the parts.
- The lead gets the same button and editor.
- Markdown: no change in this iteration.

### 4.4 Script: `assets/translate.js`

The `qr-section` element, light DOM, a page script:

- On "Translate": the blocks of the source (headings of subsections,
  paragraphs, lists), each cloned without the glosses (`rt`) and the
  `ruby` wrappers; a placeholder for each in the target; then the
  requests, 4 at a time, each block wrapped in `<section>`, and the
  answer in the place of its placeholder. A failed block keeps its source
  text, marked.
- The source and the translation side by side.
- "Discard": the editor closes, the target is emptied. "Save": the note.
- This is a new use of JavaScript (CLAUDE.md: reviewed case by case): it
  is an action of the reader, not a layer's work; the server renders all
  the content.

## 5. Steps

1. Fetch: the outline (§4.1), its unit test on a small Parsoid sample;
   `FETCH_VERSION`.
2. Localize and render: `ProseSection`, the sections in their spreads
   and at the end, the CSS; no script yet (the button does nothing).
3. The script and the editor; the messages.
4. Fixtures: `q3870.gu` text again (fetched to a temporary directory: the
   content fixture does not change); the HTTP record of Q3870 again;
   snapshots; a check in a browser.

Each step is one commit.

## 6. Iteration 2: the editor in a dialog

- **One dialog for the page**, `parts/translator.html`, rendered by
  `Ui::translator` once, in `page.html`, when the page has a text to
  translate (a fallback lead or fallback sections). The server writes
  every word of it in the page language: the title ("Translate to
  ગુજરાતી"), the column labels, the buttons, the messages. A section
  does not repeat the editor.
- **Wide:** `inline-size: min(1400px, 100vw - 2 × gutter)`, `max-block-size:
  90vh`; the header (the title and the section title, ×) and the actions
  stay in view while the rows scroll.
- **Open:** the button of a section (the script handles its click; there
  is no invoker: the dialog is empty without the script). The script
  writes the section title in the dialog title, makes the rows (the
  section title first, then the blocks and the subsection titles), and
  translates them as before; then `showModal()`.
- **Close:** ×, "Discard" or Esc: the dialog closes, the page does not
  change, and the requests still running stop (`AbortController`).
- **Save:** each block on the page takes its translation in place (the
  same element, its children replaced, `lang` and `dir` of the page
  language), so the structure stays and a second edit maps row to block
  again. The section gets the class `translated` (a mark at its side) and
  shows its note "Saved on this page only. This is a prototype: nothing
  is published." The same button opens the saved translation.
- **Edit again:** the script keeps the English blocks of its first open
  and the saved translations; the rows show them, with no request.
- The editor part in each section (`parts/translation.html`) goes away.

## 7. Questions for the next iteration

- The size of a long article in the page (India); a section could load
  its content when it opens.
- A translation per subsection, not only per section.
- The same for a page whose article exists but is short (a stub).
- Where a saved translation goes (Content Translation, a draft).

## 8. Notes of the first iteration

- `action=parse` puts the Parsoid sections in a `div.mw-parser-output`;
  the REST endpoint does not. The parser takes both.
- The coordinate template writes four forms of a coordinate; the CSS of
  the wiki hides three (`geo-nondefault`, `geo-multi-punct`). The
  serializer drops them, so "1°09′S 36°39′E" is there once (the first
  paragraph in the Markdown changed so).
- The editor classes are `.original` and `.translated`: `.from` was
  already the "W" mark of a page image.
- A button in a `summary`: the script cancels the click, so the section
  does not close, and opens it.
- The Markdown is as before: its prose comes from the sections when the
  page has them.
- The fixture `q3870.gu` has a new `fallback_article` only; its labels are
  as before.
- A link has a gloss in the page language only when the text fetch has
  the item's label: the items of the content. Most links of the body
  have none.
- Size: the HTML of `gu/Q3870` grew from 23 KB to 55 KB (gzip): the whole
  article is in the page, collapsed. A section could load its content
  when it opens (§7).

## 9. Notes of the second iteration

- Checked in a browser (Geography of `gu/Q3870`): 18 requests on the first
  open (the title and 17 blocks); after an edit and "Save", the section
  on the page is in Gujarati (`lang="gu"`), with the edit, its
  subsection titles translated, the mark and the note, and the button
  "Edit the translation"; a second open shows the saved rows with no
  request; "Discard" leaves the page as it is.
- A custom element has the `title` property of every element (the
  tooltip): the script keeps the heading of a section in `heading`.
- A row that the service did not translate keeps its source language
  when it is saved: the block on the page has `lang="en"`.
- In a page stylesheet, `:scope` is the root element: the mark of a saved
  section uses child selectors.
- The button is an icon now (§2); it keeps its size in the head of a
  section (`flex: none`).
- A media query, not the page container query, stacks the columns of the
  dialog: the dialog is in the top layer and follows the window.

## 10. Iteration 3: the article in the page language

- **The problem.** A page whose language has an article (`en/Q183`,
  `ig/Q30`) showed the first paragraph of a matched section, and the
  titles of the other sections as links. A page with a fallback article
  showed the whole article. The two pages must be the same.
- **Decision.** The page shows the whole article that it shows the lead
  of: the article in the page language, or else the fallback article.
  The placement is the same (§2). A section in the page language has no
  translate button and no saved note, and it is a `div`, not a
  `qr-section`. The page has the translator dialog only when it has a
  text in another language.
- **Fetch.** The article in the page language keeps its outline too. The
  parse request does not change; the links of the outline need more
  requests. `FETCH_VERSION` 14 → 15.
- **View.** `ProseSection` gets `local` (as `Article` and
  `SectionProse`). The view schema changes.
- **Fixtures.** The `article` of `q3870.en` is fetched again (the labels
  are as before). The replay test needs no new record: the en text uses
  the same English article as the gu text.
- **Notes.**
  - Size: the HTML of `en/Q3870` grew from 27 KB to 97 KB (gzip), the
    same as `gu/Q3870` (101 KB). `en/Q183` is 250 KB.
  - The apparatus titles (`layouts.toml`) are listed for `en` and `ml`
    only. In another wiki, "See also" and "External links" are now
    sections with content at the end of the page (`ig/Q30`: "Lee nke
    ozo", "Jikodo na ga ezí"). Before, they were only titles.
  - When every article has an outline, `unmatched_sections`,
    `prose_text` (`section_prose`) and `Article.section_text` are not
    used. The text fixtures that were fetched before this change still
    use them.
