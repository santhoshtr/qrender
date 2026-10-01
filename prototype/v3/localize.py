#!/usr/bin/env python3
"""Localize: composition + text of one language -> a localized view.

The step between layer 2 and the renderers. It holds all the work that
depends on the language and none of the editorial decisions, which the
composition already made:

- every identifier gets its label, with the language the label came from and
  a fallback flag (a mul label is the same in every language, so it is not a
  fallback);
- every value gets a display form: numbers, dates, percentages in the page
  language, qualifiers as a note made of parts;
- the lead: the article in the page language, or the fallback article, with
  its links rewritten and glosses where the text is not in the page language;
- the article sections of each spread, the topics as links, the sections no
  spread claims;
- page images for items without P18;
- label coverage of the page in this language.

The output has the shape of the composition, with display fields added. It is
data, not HTML (the article paragraphs are the only markup, and they are prose
from Wikipedia). Any renderer can read it: HTML, an app, print, text for an
LLM. None of them needs language logic of its own.

Reads data/composition/<qid>.json and data/text/<qid>.<lang>.json.
Writes data/view/<qid>.<lang>.json.

Usage: python3 prototype/v3/localize.py Q3870 en
"""

import html
import json
import math
import re
import sys
import urllib.parse
from pathlib import Path

HERE = Path(__file__).resolve().parent
WIKIDATA = "https://www.wikidata.org/wiki/"
RTL = {
    "ar",
    "arz",
    "azb",
    "ckb",
    "dv",
    "fa",
    "he",
    "ks",
    "mzn",
    "pnb",
    "ps",
    "sd",
    "ug",
    "ur",
    "yi",
}
VALUE_KEYS = ("item", "amount", "time", "text", "url", "lat", "string")
NOT_VALUES = (
    "scale",
    "peers",
    "series",
    "time",
    "media",
    "map",
    "title",
    "prose",
    "kind",
)


class Localizer:
    def __init__(self, composition, text):
        self.comp = composition
        self.t = text
        self.lang = text["lang"]
        self.labels = text["labels"]

    # ----------------------------------------------------------- labels

    def label(self, qid):
        """A label with its identifier, its language and a fallback flag. A
        fallback carries the address where a reader can add the label in the
        page language: one small edit that every page using it gains."""
        rec = self.labels.get(qid)
        out = {"id": qid}
        if not rec or "label" not in rec:
            out.update(text=qid, lang=None, fallback=True)
        else:
            out.update(
                text=rec["label"],
                lang=rec["lang"],
                fallback=rec["lang"] not in (self.lang, "mul"),
            )
        if out["fallback"]:
            out["add"] = f"{WIKIDATA}Special:SetLabel/{self.lang}/{qid}"
        return out

    def description(self, qid):
        rec = self.labels.get(qid, {})
        if "description" not in rec:
            return None
        return {
            "text": rec["description"],
            "lang": rec["description_lang"],
            "fallback": rec["description_lang"] not in (self.lang, "mul"),
        }

    def concept(self, qid):
        c = self.t["concepts"].get(qid, {})
        return {
            "text": c.get("label", qid),
            "lang": c.get("lang"),
            "fallback": c.get("lang") not in (self.lang, "mul"),
        }

    # ----------------------------------------------------------- values

    def number(self, x):
        """English grouping. Another language needs its own digits and
        separators; this is the one place to add them."""
        if abs(x) >= 1e9 or (x and abs(x) < 1e-3):
            exp = math.floor(math.log10(abs(x)))
            sup = str(exp).translate(str.maketrans("-0123456789", "⁻⁰¹²³⁴⁵⁶⁷⁸⁹"))
            return f"{x / 10**exp:.3g} × 10{sup}"
        if x == int(x):
            return f"{int(x):,}"
        return f"{x:,.6g}"

    def percent(self, ratio):
        pct = ratio * 100
        return f"{pct:.2g} %" if pct < 10 else f"{pct:.0f} %"

    def time(self, iso, precision):
        m = re.match(r"^([+-]?)0*(\d+)-(\d\d)-(\d\d)", iso)
        year = int(m.group(2)) * (-1 if m.group(1) == "-" else 1)
        if precision is None or precision <= 9:
            return f"{abs(year)} BCE" if year < 0 else str(year)
        if precision == 10:
            return f"{year}-{m.group(3)}"
        return f"{year}-{m.group(3)}-{m.group(4)}"

    def part(self, v):
        """One value as a display part: text, its language, a fallback flag,
        and a link where the value is an item or a URL."""
        if v.get("item"):
            return {**self.label(v["item"]), "href": WIKIDATA + v["item"]}
        if "amount" in v:
            out = {"text": self.number(v["amount"])}
            if v.get("unit"):
                out["unit"] = self.label(v["unit"])
            return out
        if "time" in v:
            return {"text": self.time(v["time"], v.get("precision"))}
        if "year" in v:
            return {"text": str(v["year"])}
        if "text" in v:
            return {"text": v["text"], "lang": v["lang"], "native": True}
        if "url" in v:
            return {"text": v["url"], "href": v["url"]}
        if "lat" in v:
            return {"text": f"{v['lat']:g}, {v['lon']:g}"}
        return {"text": v.get("string", v.get("file", ""))}

    def note(self, st):
        """The qualifiers of a statement as parts. A span of years reads as
        "1920 – 1963"; every other qualifier is one part."""
        qs = st.get("qualifiers", [])
        start = next(
            (q["year"] for q in qs if q["pid"] == "P580" and "year" in q), None
        )
        end = next((q["year"] for q in qs if q["pid"] == "P582" and "year" in q), None)
        parts = []
        if start or end:
            parts.append({"text": f"{start or ''} – {end or ''}".strip()})
        parts += [self.part(q) for q in qs if q["pid"] not in ("P580", "P582")]
        return parts

    # ------------------------------------------------------------- walk

    def walk(self, x):
        """Copy the composition, adding a display field next to every
        identifier and value. The shape stays the same."""
        if isinstance(x, list):
            return [self.walk(v) for v in x]
        if not isinstance(x, dict):
            return x
        out = {k: self.walk(v) for k, v in x.items()}
        if isinstance(x.get("pid"), str):
            out["property"] = self.label(x["pid"])
        if isinstance(x.get("class"), str):
            out["class_label"] = self.label(x["class"])
        if isinstance(x.get("container"), str):
            out["container_label"] = self.label(x["container"])
        # A value holder gets a display form. Blocks whose keys mean something
        # else (a map's coordinates, a scale's container) do not.
        if any(k in x for k in VALUE_KEYS) and x.get("primitive") not in NOT_VALUES:
            out["display"] = self.part(x)
        # A statement can be edited on Wikidata, at its property.
        if isinstance(x.get("statement"), str) and isinstance(x.get("pid"), str):
            out["edit"] = f"{WIKIDATA}{self.comp['qid']}#{x['pid']}"
        if isinstance(x.get("via"), list):
            out["via_labels"] = [self.label(p) for p in x["via"]]
        if isinstance(x.get("pids"), list):
            out["properties"] = [self.label(p) for p in x["pids"]]
        # An item with an amount (a peer and its population) needs both.
        if isinstance(x.get("amount"), (int, float)):
            out["amount_display"] = self.number(x["amount"])
        if "ratio" in x:
            out["percent"] = self.percent(x["ratio"])
        if "qualifiers" in x:
            out["note"] = self.note(x)
        if "item" in x and "image" in x and not x["image"] and not x.get("class"):
            page = self.t.get("page_images", {}).get(x["item"])
            if page:
                # A picture from the item's article, not from Wikidata.
                out["image"] = {
                    "file": page["file"],
                    "aspect": round(page["width"] / page["height"], 4),
                    "kind": "drawing"
                    if page["file"].lower().endswith(".svg")
                    else "photo",
                    "credit": {
                        "artist": page.get("artist", ""),
                        "license": page.get("license", ""),
                    },
                    "from": page["source"],
                }
        return out

    # ------------------------------------------------------------ prose

    def article(self):
        art = self.t.get("article")
        if art:
            return art, self.lang
        fb = self.t.get("fallback_article")
        return (fb, "en") if fb else (None, None)

    def rewrite(self, paragraph, links, source_lang):
        """Clean the article markup, then rewrite its links: to the article in
        the page language when the text is in the page language, otherwise to
        the item, with its label in the page language as a gloss."""
        p = re.sub(r'<img[^>]*\balt="([^"]*)"[^>]*>', r"\1", paragraph)
        p = re.sub(
            r"</?(\w+)[^>]*>",
            lambda m: m.group(0) if m.group(1).lower() in ("a", "b", "i") else "",
            p,
        )
        p = re.sub(r"<(b|i)\s[^>]*>", r"<\1>", p)

        def link(m):
            href = re.search(r'href="/wiki/([^"#]+)', m.group(1))
            text = m.group(2)
            if not href:
                return text
            if source_lang == self.lang:
                return f'<a href="https://{self.lang}.wikipedia.org/wiki/{html.escape(href.group(1))}">{text}</a>'
            qid = links.get(urllib.parse.unquote(href.group(1)).replace("_", " "))
            if not qid:
                return text
            a = f'<a href="{WIKIDATA}{qid}">{text}</a>'
            gloss = self.labels.get(qid, {})
            if gloss.get("lang") == self.lang:
                return f"<ruby>{a}<rt><span>{html.escape(gloss['label'])}</span></rt></ruby>"
            return a

        return re.sub(r"<a\s([^>]*)>(.*?)</a>", link, p, flags=re.S).strip()

    def lead(self, block):
        art, source_lang = self.article()
        if not art:
            return {**block, "article": None}
        paras = [self.rewrite(p, art["links"], source_lang) for p in art["paragraphs"]]
        host = f"{source_lang}.wikipedia.org"
        return {
            **block,
            "article": {
                "lang": source_lang,
                "local": source_lang == self.lang,
                "host": host,
                "url": f"https://{host}/wiki/{urllib.parse.quote(art['title'].replace(' ', '_'))}",
                "paragraphs": paras[:1],
                "more": paras[1:],
            },
        }

    def section_prose(self, concepts):
        """The first paragraph of the article section that belongs to a
        spread, with its source. Without a matched section, nothing."""
        art, source_lang = self.article()
        if not art:
            return None
        titles = {s["anchor"]: s["title"] for s in art["sections"]}
        for q in concepts:
            for anchor in art.get("concept_sections", {}).get(q, []):
                text = art.get("section_text", {}).get(anchor)
                if text:
                    base = f"https://{source_lang}.wikipedia.org/wiki/{urllib.parse.quote(art['title'].replace(' ', '_'))}"
                    return {
                        "html": self.rewrite(text, art["links"], source_lang),
                        "lang": source_lang,
                        "local": source_lang == self.lang,
                        "title": titles.get(anchor, anchor),
                        "href": f"{base}#{anchor}",
                    }
        return None

    def section_links(self, concepts):
        art, source_lang = self.article()
        if not art:
            return []
        titles = {s["anchor"]: s["title"] for s in art["sections"]}
        base = f"https://{source_lang}.wikipedia.org/wiki/{urllib.parse.quote(art['title'].replace(' ', '_'))}"
        return [
            {"text": titles.get(a, a), "lang": source_lang, "href": f"{base}#{a}"}
            for q in concepts
            for a in art.get("concept_sections", {}).get(q, [])
        ]

    def unmatched_sections(self):
        art, source_lang = self.article()
        if not art:
            return []
        used = {
            a for anchors in art.get("concept_sections", {}).values() for a in anchors
        }
        base = f"https://{source_lang}.wikipedia.org/wiki/{urllib.parse.quote(art['title'].replace(' ', '_'))}"
        return [
            {"text": s["title"], "lang": source_lang, "href": f"{base}#{s['anchor']}"}
            for s in art["sections"]
            if s["anchor"] not in used
        ]

    # --------------------------------------------------------- coverage

    def coverage(self):
        """Label coverage in this language, over the items and properties a
        reader sees (everything but the evidence spread)."""
        ids = set()

        def collect(x):
            if isinstance(x, dict):
                for k, v in x.items():
                    if (
                        k in ("item", "pid", "unit", "class")
                        and isinstance(v, str)
                        and re.fullmatch(r"[PQ]\d+", v)
                    ):
                        ids.add(v)
                    else:
                        collect(v)
            elif isinstance(x, list):
                for v in x:
                    collect(v)

        for sp in self.comp["spreads"]:
            if sp["id"] != "evidence":
                collect(sp["slots"])
        have = sum(
            1 for i in ids if self.labels.get(i, {}).get("lang") in (self.lang, "mul")
        )
        return {"have": have, "total": len(ids)}

    # ------------------------------------------------------------- page

    def spread(self, sp):
        out = self.walk(sp)
        out["title"] = self.concept(sp["concept"]) if sp.get("concept") else None
        out["prose_text"] = self.section_prose(sp.get("prose", {}).get("concepts", []))
        links = self.section_links(sp.get("prose", {}).get("concepts", []))
        links += [
            {**self.label(t["item"]), "href": WIKIDATA + t["item"]}
            for t in sp.get("topics", [])
        ]
        out["prose_links"] = links
        for name, content in out["slots"].items():
            blocks = content if isinstance(content, list) else [content]
            for i, b in enumerate(blocks):
                if b.get("primitive") == "prose":
                    blocks[i] = self.lead(b)
                elif b.get("primitive") == "title":
                    b["title"] = self.label(b["item"])
                    b["description"] = self.description(b["item"])
                elif b.get("primitive") == "provenance":
                    names = {
                        "sourced": [self.label("P248"), self.label("P854")],
                        "imported": [self.label("P143")],
                        "none": [],
                    }
                    b["legend"] = [
                        {"kind": k, "labels": names[k], "count": b[k]}
                        for k in ("sourced", "imported", "none")
                    ]
                    b["coverage"] = {"lang": self.lang, **self.coverage()}
                elif b.get("primitive") == "sitelinks":
                    b["families"] = [
                        {"name": f, "count": len(l), "local": self.lang in l}
                        for f, l in sorted(
                            b["families"].items(), key=lambda kv: -len(kv[1])
                        )
                    ]
            if not isinstance(content, list):
                out["slots"][name] = blocks[0]
        return out

    def view(self):
        qid = self.comp["qid"]
        return {
            "schema": "view/0.1",
            "composition": self.comp["schema"],
            "design": self.comp["design"],
            "qid": qid,
            "lang": self.lang,
            "dir": "rtl" if self.lang in RTL else "ltr",
            "page_title": self.label(qid)["text"],
            # Words for the provenance tags, from Wikidata's reference properties.
            "provenance_labels": {
                "sourced": self.label("P248"),
                "imported": self.label("P143"),
            },
            "spreads": [
                self.spread(sp)
                for sp in self.comp["spreads"]
                if sp["slots"] or sp.get("topics")
            ],
            "unmatched_sections": self.unmatched_sections(),
        }


def main(qid, lang):
    comp = json.loads(
        (HERE / "data" / "composition" / f"{qid.lower()}.json").read_text()
    )
    text = json.loads(
        (HERE / "data" / "text" / f"{qid.lower()}.{lang}.json").read_text()
    )
    view = Localizer(comp, text).view()
    out = HERE / "data" / "view" / f"{qid.lower()}.{lang}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(view, ensure_ascii=False, indent=1))
    print(f"{out.name}: {len(view['spreads'])} spreads ({out.stat().st_size:,} B)")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
