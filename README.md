# QRender

QRender makes a page for a Wikidata item, in any language. It composes the
page by rule from the statements of the item: which sections the page has,
which value leads, what is a picture, a list, a map, a time axis or a
comparison with the item's peers. The same composition serves every
language; only the labels change.

The page uses only Wikimedia resources: Wikidata, Wikipedia (the lead and
the section text), Wikimedia Commons (pictures) and Wikimedia Maps. Every
statement links to Wikidata for editing, and a label that is not in the
page language links to the place where a reader can add it.

The same page is also available as Markdown, for readers that read text,
such as large language models. It keeps all the values, the qualifiers,
the sources and the language of each label.

## Build

You need Rust and Cargo.

```bash
git clone https://github.com/santhoshtr/qrender.git
cd qrender
cargo build --release
```

## Command line

```bash
qrender page --qid Q3870 --lang en > nairobi.html
qrender page --qid Q3870 --lang en --format markdown
```

Each step can also run on its own, with the files of one step as the
input of the next:

```bash
qrender fetch --qid Q3870 --lang ml --out data      # content and text
qrender compose --content data/content/q3870.json   # the composition
qrender view --content data/content/q3870.json --text data/text/q3870.ml.json
```

## Web service

```bash
qrender-server     # listens on PORT (default 4243)
```

- `/en/Q3870`: the page of Nairobi, in English
- `/ml/Q405.md`: the page of the Moon, in Malayalam, as Markdown
- `/api/composition/Q3870`: the composition, as JSON
- `/api/view/ml/Q405`: the page data in one language, as JSON
- `/api/content/Q3870`: the data from Wikidata, as JSON
- `/healthz`

Set `REDIS_URL` (a `redis://` URL or `host:port`) to keep fetched data for
a day; without it, every request fetches again. On a shared Redis, set
`REDIS_PREFIX` to a long random secret: it goes before every key.
`BASE_PATH` is for a service under a subpath (for example `/qrender`), and
`USER_AGENT` names the operator for the Wikimedia services. These can also
be in a `.env` file.

## How it works

Three layers, each with its own data:

1. **Content**: the statements of the item, with identifiers and typed
   values, and the side data (references, the container, peers, Commons
   metadata). The text of one language (labels, the article) is apart.
2. **Composition**: the editorial decisions, from rule tables
   (`qrender/rules.toml`) and class layouts (`qrender/layouts.toml`). It
   has no text and no geometry, so it is the same for every language.
3. **Layout**: the design system, in CSS. A localize step joins the
   composition and the text of one language into a view; the renderers
   read only the view.

The rules and the layouts are data: a community can change how a kind of
item looks without a change to the code.
