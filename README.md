# QRender

Render Wikidata information in various formats:

* Text
* Markdown
* HTML
* Wikitext


The text and makrdown formats are designed to be human-readable, while the HTML and Wikitext formats are designed for use in web pages and wikis, respectively.

The human-readable formats are useful when integrating wikidata information to a tool calling LLMs. For an example LLM based chat bot that uses this format internally, see https://wq42.toolforge.org/

## Installation

To install QRender, you need to have Rust and Cargo installed on your system. Once you have them, you can build and install QRender using the following commands:

```bash
git clone https://github.com/your-repo/qrender.git
cd qrender
cargo install --path .
```

This will compile the project and install the `qrender` binary to your Cargo bin directory (usually `~/.cargo/bin`).

## Usage

```
Wikidata Renderer

Usage: qrender [OPTIONS]

Options:
  -q, --qid <QID>            The QID of the Wikidata item to render [default: Q405]
  -l, --language <LANGUAGE>  The language to use [default: en]
  -f, --format <FORMAT>      Render format (text or html) [default: text] [possible values: text, html, markdown, wikitext]
  -i, --ignore-ids           Ignore IDs in the output
  -h, --help                 Print help
  -V, --version              Print version                                                                                                                                                                                             /0.5s
```

