# The images of the article

Status: done (2026-10-22).

## 1. The problem

The page shows the sections of the article (in the page language, or
the fallback article) with their headings, paragraphs and lists. The
parser dropped the images. But the images are content: their details
add context to the text.

## 2. Decisions

- **Full-width blocks, not thumbnails.** An image is a block at the full
  width of the text column, with its aspect ratio, in its place among
  the paragraphs. The thumbnail beside the text is an idea of Wikipedia
  from twenty years ago: the reader must click to see the details. On
  the modern web an image is a first-class item. The mobile interface of
  Wikipedia does this. A tall image is at most 80% of the window high,
  so it is narrower and keeps its aspect ratio.
- **What.** A thumbnail (`figure`), the images of a multiple-image
  template (`div.tmulti`), and the images of a gallery (`ul.gallery`).
  In a group, each image has its own caption under it, and the group can
  have a caption after its images.
- **Commons only.** A local file of the wiki is often not free (fair
  use): its licence is for that article only. It is also not on Commons,
  so the Commons URL that the page uses cannot show it. The parser skips
  it, and a figure without an image goes away.
- **The link.** An image links to its file page on Commons, which has
  the author and the licence. The file name is the name of the link for
  a screen reader; the caption is the text next to it, so the image has
  an empty `alt`.
- **Not in this iteration.** The images of the lead (the lead is beside
  the head picture of the page). The translation of a caption.

## 3. Design

- **Content.** `OutlineSection.blocks: Vec<OutlineBlock>`, an untagged
  enum: `Text(String)` or `Figure(ArticleFigure)`. A string is a text
  block, so the text fixtures of before stay valid.

  ```rust
  pub struct ArticleFigure {
      pub images: Vec<ArticleImage>,
      /// The caption of the whole figure, as HTML.
      pub caption: String,
  }
  pub struct ArticleImage {
      /// The name of the file on Commons, without "File:".
      pub file: String,
      pub width: u32,
      pub height: u32,
      /// The caption of this image in a group, as HTML.
      pub caption: String,
  }
  ```

- **Parse.** The file name and "is it on Commons" come from the `src` of
  `img.mw-file-element`: `/wikipedia/commons/[thumb/]a/ab/NAME`. The
  URL does not depend on the language of the wiki, as the "File:"
  namespace does. The size comes from `data-file-width` and
  `data-file-height`. A caption is cleaned as a paragraph. The captions
  go with the texts whose links and notes the fetcher reads.
  `FETCH_VERSION` 16 → 17.
- **Localize.** `ProseSection.blocks: Vec<ProseBlock>`: `Text`, or
  `Figure` with the aspect ratio of each image. The captions are
  rewritten as the text (links, note marks). The view schema changes.
- **Render.** The part `parts/article_blocks.html` (the blocks in order,
  for a section and a subsection) and the part
  `parts/article_figure.html`. The `img` has the standard widths of the
  page (`srcset`) and loads when it comes near the view (`lazy`).
- **Translation.** `translate.js` reads only the text blocks; it does
  not see a figure. A saved translation keeps the figures as they are.
  A known gap, not for now: the editor dialog shows no images and no
  references (`translate.js` also removes the note marks from a block
  before the machine translation). See
  `fallback-article-translation.md` §7.
- **Markdown.** No change: a section gives its first text block.

## 4. Notes

- Nairobi: 28 figures in its sections. Germany: 26 thumbnails and 2
  multiple-image templates. India: 19 thumbnails and 16 galleries; 14 of
  its files are local and are skipped.
- The fixtures `q3870.en` (`article`) and `q3870.gu`
  (`fallback_article`) were fetched again. The replay test reads the
  recorded parse, so it has the figures with no new record.
- During this work `wiki-display-image.toolforge.org` (the page images)
  did not answer. The fetcher continues without a page image, but only
  after its timeout, so a text fetch took about 70 s in place of 7 s.
