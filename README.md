# QRender

Render Wikidata information in various formats:

- Text
- Markdown
- HTML
- Wikitext
- Factoid — a visual card page

The text and markdown formats are designed to be human-readable and are
useful when integrating Wikidata information into tool-calling LLMs. For an
example LLM based chat bot that uses this format internally, see
<https://wq42.toolforge.org/>

The factoid format renders an item as a page of fact cards — images,
stats with history charts, maps, links — using only Wikipedia and sister
project resources (Wikidata, Wikimedia Commons, Wikimedia Maps, Codex
design tokens). It is meant as a visual complement to article prose: a
quick glance at the structured facts of a topic.

## Installation

To install QRender, you need to have Rust and Cargo installed on your
system. Once you have them, you can build and install QRender using the
following commands:

```bash
git clone https://github.com/santhoshtr/qrender.git
cd qrender
cargo install --path qrender
```

This installs the `qrender` CLI and the `qrender-server` web service to
your Cargo bin directory (usually `~/.cargo/bin`).

## Usage

```
Wikidata Renderer

Usage: qrender [OPTIONS]

Options:
  -q, --qid <QID>            The QID of the Wikidata item to render [default: Q405]
  -l, --language <LANGUAGE>  The language to use [default: en]
  -f, --format <FORMAT>      Render format [default: text] [possible values: text, html, markdown, wikitext, factoid]
  -i, --ignore-ids           Ignore IDs in the output
  -h, --help                 Print help
  -V, --version              Print version
```

## Web interface

`qrender-server` serves factoid pages over HTTP:

```bash
qrender-server            # listens on PORT (default 8000)
```

Example URL paths:

- `/en/Q3870` — factoid page for Nairobi, in English
- `/ml/Q405` — factoid page for the Moon, in Malayalam
- `/api/en/Q3870` — the underlying card data as JSON, for other frontends
- `/healthz` — liveness probe

Set `REDIS_URL` (plain `host:port` or a `redis://` URL) to cache Wikidata
query results for a week; without it every request queries the Wikidata
Query Service directly. Both variables can also be provided via a `.env`
file.
