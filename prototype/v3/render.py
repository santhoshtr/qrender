#!/usr/bin/env python3
"""The HTML renderer: localized view -> a static page.

Templates only. It reads data/view/<qid>.<lang>.json and nothing else: no
labels, no text file, no composition. Every string it prints was resolved and
formatted by localize.py, every link it prints was built there, and every
editorial decision was made by compose.py. The templates map a spread to a
section with its template class, a slot to an element with data-slot, and a
block to the markup of its primitive. They copy fractions and aspect ratios
into CSS variables; the design system (design/page.css) does the geometry.

One exception, noted where it is: a map. Its zoom and the place of its
markers depend on the frame this renderer draws, so they are computed here.

Usage: python3 prototype/v3/render.py Q3870 en
"""

import html
import itertools
import json
import math
import sys
import urllib.parse
from pathlib import Path

from primitives import credits, network, succession

HERE = Path(__file__).resolve().parent
STANDARD_WIDTHS = (330, 500, 960, 1280, 1920)
FILEPATH = "https://commons.wikimedia.org/wiki/Special:FilePath/"
WIKIPEDIA_LOGO = FILEPATH + "Wikipedia-logo-v2.svg?width=60"

UI = sys.modules[__name__]  # the template helpers, for the primitive modules
IDS = itertools.count(1)  # ids for popovers and dialogs
DIALOGS = []  # dialogs go at the end of the page
PROV = {}  # words for the provenance tags, from the view


def e(text):
    return html.escape(str(text), quote=True)


# ------------------------------------------------------------ small parts


def add_label(label):
    """The action to add a label in the page language, for a fallback."""
    if not (label and label.get("fallback") and label.get("add")):
        return ""
    return (
        f'<a class="add-label" href="{e(label["add"])}" title="+ {e(label["id"])}" '
        f'aria-label="Add label">+</a>'
    )


def t(label, add=True):
    """A resolved label. A fallback carries its language, a mark, and an
    action to add the label in the page language."""
    if not label:
        return ""
    if label.get("native"):
        return f'<span lang="{e(label["lang"])}">{e(label["text"])}</span> <span class="note">{e(label["lang"])}</span>'
    if label.get("fallback") and label.get("lang"):
        text = f'<span class="fb" lang="{e(label["lang"])}">{e(label["text"])}</span>'
        action = add_label(label) if add else ""
        return f'<span class="fb-group">{text}{action}</span>' if action else text
    return e(label["text"])


def part(p):
    """A display part: a label, a number with its unit, a link. The action
    to add a label goes after the link, not in it: a link in a link is not
    valid HTML, and the parser would break the markup around it."""
    inner = t(p, add=not p.get("href"))
    if p.get("unit"):
        inner += f' <span class="note">{t(p["unit"])}</span>'
    if not p.get("href"):
        return inner
    link = f'<a href="{e(p["href"])}">{inner}</a>'
    action = add_label(p)
    return f'<span class="fb-group">{link}{action}</span>' if action else link


def note(parts):
    return " · ".join(part(p) for p in parts or [])


def prov(kind):
    """Provenance, marked by exception: a sourced statement carries no mark;
    one whose only reference is a Wikipedia carries a worded tag; one with no
    reference carries a quiet mark."""
    if kind == "imported":
        label = PROV.get("imported", {}).get("text", "")
        return f'<span class="prov-imported" title="{e(label)}">W</span>'
    if kind == "none":
        return '<span class="prov-none" title="—"></span>'
    return ""


def edit(item):
    href = item.get("edit") if item else None
    return f'<a class="edit" href="{e(href)}" aria-label="Edit">✎</a>' if href else ""


def credit(c):
    """The author and licence of a picture, behind an (i) button."""
    text = " · ".join(
        x for x in ((c or {}).get("artist"), (c or {}).get("license")) if x
    )
    if not text:
        return ""
    n = next(IDS)
    return (
        f'<button class="info" type="button" popovertarget="p{n}" aria-label="Credit">i</button>'
        f'<span id="p{n}" popover class="pop">{e(text)}</span>'
    )


def dialog(title, body):
    """A dialog with the rest of a list, opened by an invoker button."""
    n = next(IDS)
    DIALOGS.append(
        f'<dialog id="d{n}" closedby="any"><header><h2>{title}</h2>'
        f'<button type="button" command="close" commandfor="d{n}" aria-label="Close">×</button>'
        f'</header><div class="dialog-body">{body}</div></dialog>'
    )
    return n


def srcset(name):
    base = FILEPATH + urllib.parse.quote(name)
    return (
        f'src="{e(base)}?width=960" srcset="'
        + ", ".join(f"{e(base)}?width={w} {w}w" for w in STANDARD_WIDTHS)
        + '"'
    )


def value_line(s):
    """One value on its own line: the value, its note, its edit action. A
    value looks the same in a row of one value and in a row of many."""
    n = note(s.get("note"))
    return (
        f'<span class="v">{part(s["display"])}'
        + (f' <span class="note">{n}</span>' if n else "")
        + edit(s)
        + "</span>"
    )


def value_lines(sts, title=""):
    """Values one per line: the ones the composer marked as shown, and all
    of them in a dialog."""
    shown = [s for s in sts if s.get("shown", True)]
    rest = [s for s in sts if not s.get("shown", True)]
    more = ""
    if rest:
        n = dialog(title, "".join(value_line(s) for s in sts))
        more = f'<button class="more" type="button" command="show-modal" commandfor="d{n}">+{len(rest)}</button>'
    return "".join(value_line(s) for s in shown) + more


# ------------------------------------------------------------- primitives


def p_title(b):
    tag = (
        f' <span class="note">{e(b["title"]["lang"])}</span>'
        if b["title"].get("fallback")
        else ""
    )
    desc = f"<p>{t(b['description'])}</p>" if b.get("description") else ""
    return f'<div class="title"><h1>{e(b["title"]["text"])}{tag}</h1>{desc}</div>'


def p_kind(b):
    def values(vs):
        shown = [v for v in vs if v.get("shown", True)]
        rest = len(vs) - len(shown)
        return ", ".join(part(v["display"]) for v in shown) + (
            f' <span class="note">+{rest}</span>' if rest else ""
        )

    return (
        '<div class="kind">'
        + "".join(
            f'<div><span class="k">{t(link["property"])}</span>{values(link["values"])}</div>'
            for link in b["chain"]
        )
        + "</div>"
    )


def p_prose(b):
    art = b.get("article")
    if not art:
        return f'<div class="lead"><p class="note">— {b["languages"]}</p></div>'
    more = ""
    if art["more"]:
        more = (
            f"<details><summary>¶ +{len(art['more'])}</summary>"
            + "".join(f"<p>{p}</p>" for p in art["more"])
            + "</details>"
        )
    return (
        f'<div class="lead" lang="{e(art["lang"])}">'
        + "".join(f"<p>{p}</p>" for p in art["paragraphs"])
        + more
        + f'<p class="source"><img src="{WIKIPEDIA_LOGO}" alt="" width="18"> '
        f'<a href="{e(art["url"])}">{e(art["host"])}</a> · {b["languages"]}</p></div>'
    )


def p_media(b):
    kind, name, aspect = b["kind"], b["file"], b.get("aspect", 4 / 3)
    label = f"<span>{t(b['property'])}</span>"
    star = (
        f'<span class="assessed" title="{e(", ".join(b["assessed"]))}">★</span>'
        if b.get("assessed")
        else ""
    )
    src = FILEPATH + urllib.parse.quote(name)
    tools = f'<span class="tools">{credit(b.get("credit"))}{edit(b)}</span>'
    if kind == "audio":
        n = note(b.get("note"))
        return (
            f'<div class="block"><h3>{t(b["property"])}{prov(b.get("provenance"))}{edit(b)}</h3>'
            f'<audio controls preload="none" src="{e(src)}"></audio>'
            + (f'<span class="note">{n}</span>' if n else "")
            + "</div>"
        )
    if kind == "video":
        size = (
            f'<span class="size">{b["bytes"] / 1e6:.0f} MB</span>'
            if b.get("bytes")
            else ""
        )
        return (
            f'<figure class="m k-video" style="--a:{aspect}">'
            f'<video controls preload="none" poster="{e(src)}?width=960" src="{e(b.get("url", src))}"></video>'
            f"<figcaption>{label}{prov(b.get('provenance'))}{size}</figcaption>{tools}</figure>"
        )
    sizes = (
        "100vw"
        if b["role"] in ("feature", "banner")
        else "(max-width: 760px) 100vw, 40vw"
    )
    page = f"https://commons.wikimedia.org/wiki/File:{urllib.parse.quote(name.replace(' ', '_'))}"
    caption = (
        ""
        if b["role"] in ("identity", "banner")
        else f"<figcaption>{label}{star}{prov(b.get('provenance'))}</figcaption>"
    )
    # The page head is always above the fold: its pictures load first.
    load = {"identity": 'fetchpriority="high"', "banner": ""}.get(
        b["role"], 'loading="lazy"'
    )
    return (
        f'<figure class="m k-{kind}" style="--a:{aspect}"><a href="{e(page)}">'
        f'<img {srcset(name)} sizes="{sizes}" alt="" {load}></a>{caption}{tools}</figure>'
    )


def item_figure(v, prop=None):
    """The picture of an item: its P18, or a page image (marked W), or an
    empty slot that asks for one."""
    image = v.get("image")
    head = f"<span>{t(prop)}</span>" if prop else ""
    name = f"<b>{part(v['display'])}</b>"
    n = note(v.get("note"))
    n = f"<span>{n}</span>" if n else ""
    if not image:
        return (
            f'<figure class="m k-empty" style="--a:1.333"><figcaption>{head}{name}{n}'
            f'{prov(v.get("provenance"))}</figcaption><span class="tools">{edit(v)}</span></figure>'
        )
    source = (
        f'<span class="from" title="{e(image["from"])}">W</span>'
        if image.get("from")
        else ""
    )
    return (
        f'<figure class="m k-{image["kind"]}" style="--a:{image.get("aspect", 1.333)}">'
        f'<img {srcset(image["file"])} sizes="(max-width: 760px) 50vw, 20vw" alt="" loading="lazy">'
        f"<figcaption>{head}{name}{source}{n}{prov(v.get('provenance'))}</figcaption>"
        f'<span class="tools">{credit(image.get("credit"))}{edit(v)}</span></figure>'
    )


def scroller(name, figures):
    """The values of one property, of equal importance: one row that
    scrolls. It takes the keyboard focus, so arrow keys scroll it."""
    return f'<div class="run scroll" tabindex="0" role="group" aria-label="{e(name)}">{figures}</div>'


def p_works(b):
    """Works by the item: pictures, or a list when most have no picture."""
    title = ", ".join(t(p) for p in b["properties"])
    values = [
        dict(v, note=[{"text": str(v["year"])}] if v.get("year") else None)
        for v in b["values"]
    ]
    shown = values[: b["shown_count"]]
    rest = values[b["shown_count"] :]
    more = ""
    if rest:
        n = dialog(title, "".join(value_line(v) for v in values))
        more = f'<button class="more" type="button" command="show-modal" commandfor="d{n}">+{len(rest)}</button>'
    if b["form"] == "list":
        body = f'<div class="list">{"".join(value_line(v) for v in shown)}{more}</div>'
    else:
        body = (
            scroller(
                ", ".join(p["text"] for p in b["properties"]),
                "".join(item_figure(v) for v in shown),
            )
            + more
        )
    return f'<div class="block set"><h3><span>{title}</span> <span class="note">{len(values)}</span></h3>{body}</div>'


def p_scale(b):
    rows = "".join(
        f'<div class="bar self"><span class="name">{t(m["property"])}</span>'
        f'<span class="track"><span style="--f:{min(m["ratio"], 1):.6f}"></span></span>'
        f'<span class="val">{e(m["percent"])}</span></div>'
        for m in b["measures"]
    )
    c = b["container"]
    return (
        f'<div class="block scale"><h3>{t(c["property"])}: {part(c["display"])}</h3>'
        f'<div class="bars">{rows}</div></div>'
    )


def p_peers(b):
    rows = "".join(
        f'<a class="bar{" self" if m["self"] else ""}" href="{e(m["display"]["href"])}">'
        f'<span class="name">{t(m["display"])}</span>'
        f'<span class="track"><span style="--f:{m["fraction"]}"></span></span>'
        f'<span class="val">{e(m["amount_display"])}</span></a>'
        for m in b["members"]
        if m["shown"]
    )
    where = f" · {t(b['container_label'])}" if b.get("container_label") else ""
    return (
        f'<div class="block"><h3>{t(b["property"])} <span class="note">{t(b["class_label"])}{where} · '
        f'{b["rank"]} / {b["of"]}</span></h3><div class="bars">{rows}</div></div>'
    )


def p_series(b):
    cols = "".join(
        f'<div title="{p["year"]}: {e(p["amount_display"])}"><span>{e(p["amount_display"]) if p.get("labelled", True) else ""}</span>'
        f'<span class="col" style="--f:{p["fraction"]}"></span>'
        f'<span class="note">{p["year"] if p.get("labelled", True) else ""}</span></div>'
        for p in b["points"]
    )
    source = ""
    if b.get("source"):
        page = f"https://commons.wikimedia.org/wiki/{urllib.parse.quote(b['source']['file'].replace(' ', '_'))}"
        source = f'<p class="note"><a href="{e(page)}">{e(b["source"]["file"])}</a></p>'
    return (
        f'<div class="block"><h3>{t(b["property"])}{prov(b.get("provenance"))}{edit(b)}</h3>'
        f'<div class="columns{" dense" if len(b["points"]) > 12 else ""}">{cols}</div>{source}</div>'
    )


def mercator(lat, lon, zoom):
    n = 256 * 2**zoom
    s = math.sin(math.radians(lat))
    return (lon + 180) / 360 * n, (
        0.5 - math.log((1 + s) / (1 - s)) / (4 * math.pi)
    ) * n


def p_map(b):
    # The exception to "no arithmetic": the zoom and the place of each marker
    # depend on the frame this renderer draws (800 × 500), which the composer
    # cannot know. Other renderers draw other frames.
    width, height, zoom = 800, 500, 11
    if b.get("extent_m"):
        # 156543 m is one pixel at zoom 0 on the equator.
        zoom = max(
            3,
            min(
                14,
                round(
                    math.log2(
                        width
                        * 156543
                        * math.cos(math.radians(b["lat"]))
                        / (3 * b["extent_m"])
                    )
                ),
            ),
        )
    cx, cy = mercator(b["lat"], b["lon"], zoom)
    markers = ""
    for m in b.get("markers", []):
        x, y = mercator(m["lat"], m["lon"], zoom)
        left, top = 50 + (x - cx) / width * 100, 50 + (y - cy) / height * 100
        if 0 <= left <= 100 and 0 <= top <= 100:
            markers += (
                f'<span class="marker" style="left:{left:.2f}%;top:{top:.2f}%" '
                f'title="{e(m["property"]["text"])}"></span>'
            )
    tile = f"https://maps.wikimedia.org/img/osm-intl,{zoom},{b['lat']},{b['lon']},{width}x{height}@2x.png"
    return (
        f'<figure class="map"><img src="{tile}" alt=""><span class="pin"></span>{markers}'
        f"<figcaption>{t(b['property'])} {b['lat']:g}, {b['lon']:g} {prov(b.get('provenance'))}"
        f'</figcaption><span class="tools">{edit(b)}</span></figure>'
    )


def fact_value(b):
    sts = b["statements"]
    if b["form"] == "range":
        lo, hi = sts
        return f"{e(lo['amount_display'])} – {part(hi['display'])}"
    if b["form"] == "variants":
        return "<br>".join(
            f'{part(s["display"])} <span class="note">{note(s.get("note"))}</span>'
            for s in sts
        )
    if len(sts) > 1 or b["form"] == "list":
        return value_lines(sts, t(b["property"]))
    n = note(sts[0].get("note"))
    age = (
        f' <span class="note">{t(b["age"]["property"])} {b["age"]["years"]}</span>'
        if b.get("age")
        else ""
    )
    return (
        part(sts[0]["display"]) + (f' <span class="note">{n}</span>' if n else "") + age
    )


def fact_row(b):
    first = b["statements"][0]
    single = len(b["statements"]) == 1 and b["form"] != "list"
    return (
        f'<div class="row r-{b["form"]}{" former" if b.get("former") else ""}">'
        f"<dt>{t(b['property'])}</dt><dd>{fact_value(b)}</dd>"
        f'<span class="tools">{prov(first.get("provenance")) if single else ""}{edit(first) if single else ""}</span></div>'
    )


def p_fact(b):
    return f'<dl class="sheet">{fact_row(b)}</dl>'


def p_absent(b):
    return (
        f'<div class="row absent"><dt>{t(b["property"])}</dt><dd>— {b["peers_with_it"]} / {b["peers"]} '
        f"{t(b['class_label'])}</dd></div>"
    )


def p_time(b):
    tracks = []
    for tr in b["tracks"]:
        marks, labels = [], []
        for m in tr["marks"]:
            what = part(m["display"]) if m.get("item") else ""
            if "year_pos" in m:
                marks.append(f'<span class="dot" style="--p:{m["year_pos"]}"></span>')
                labels.append(f"<span><b>{m['year']}</b> {what}</span>")
            else:
                # An unknown start or end is drawn as a fade to that edge of
                # the axis, not as a known date there.
                start, end = m.get("start_pos"), m.get("end_pos")
                ends = ("" if start is not None else " open-start") + (
                    "" if end is not None else " open"
                )
                marks.append(
                    f'<span class="span{ends}" '
                    f'style="--s:{start if start is not None else 0};--e:{end if end is not None else 1}"></span>'
                )
                labels.append(
                    f"<span><b>{m.get('start') or ''} – {m.get('end') or ''}</b> {what}</span>"
                )
        tracks.append(
            f'<div class="track"><span class="track-name">{t(tr["property"])}</span>'
            f'<div class="line">{"".join(marks)}</div><div class="marks">{"".join(labels)}</div></div>'
        )
    return (
        f'<div class="axis"><div class="ends"><span>{b["from"]}</span><span>{b["to"]}</span></div>'
        f"{''.join(tracks)}</div>"
    )


def p_provenance(b):
    segs = "".join(
        f'<span class="p-{l["kind"]}" style="--n:{l["count"]}"></span>'
        for l in b["legend"]
        if l["count"]
    )
    marks = {"sourced": "", "imported": prov("imported"), "none": prov("none")}
    legend = "".join(
        f"<li>{marks[l['kind']]} {' / '.join(t(x) for x in l['labels']) or '—'} <b>{l['count']}</b></li>"
        for l in b["legend"]
    )
    c = b["coverage"]
    return (
        f'<div class="block"><div class="stack-bar">{segs}</div><ul class="legend">{legend}</ul>'
        f'<p class="note">{e(c["lang"])}: {c["have"]} / {c["total"]}</p></div>'
    )


def p_sitelinks(b):
    return (
        '<div class="block"><div>'
        + "".join(
            f'<span class="chip">{e(f["name"])} <b>{f["count"]}</b>{" ✓" if f["local"] else ""}</span>'
            for f in b["families"]
        )
        + "</div></div>"
    )


def p_identifiers(b):
    rows = "".join(
        f"<div><span>{t(v['property'])}</span><span>{e(v['string'])}</span></div>"
        for v in sorted(b["values"], key=lambda v: v["property"]["text"].lower())
    )
    n = dialog(f"{len(b['values'])}", f'<div class="idlist">{rows}</div>')
    return f'<button class="more" type="button" command="show-modal" commandfor="d{n}"># {len(b["values"])}</button>'


PRIMITIVES = {
    "title": p_title,
    "kind": p_kind,
    "prose": p_prose,
    "media": p_media,
    "scale": p_scale,
    "peers": p_peers,
    "series": p_series,
    "map": p_map,
    "fact": p_fact,
    "network": lambda b: network.html(b, UI),
    "succession": lambda b: succession.html(b, UI),
    "credits": lambda b: credits.html(b, UI),
    "works": p_works,
    "time": p_time,
    "absent": p_absent,
    "provenance": p_provenance,
    "sitelinks": p_sitelinks,
    "identifiers": p_identifiers,
}


# ------------------------------------------------------------------ page


def slot_html(name, content):
    blocks = content if isinstance(content, list) else [content]
    if name == "gallery":
        # Single pictures of different properties, of the item and of its
        # values: one justified run.
        figures = "".join(
            PRIMITIVES["media"](b)
            if b["primitive"] == "media"
            else "".join(item_figure(v, v["property"]) for v in b["values"])
            for b in blocks
        )
        return f'<div data-slot="gallery"><div class="block set"><div class="run">{figures}</div></div></div>'
    if name in ("sheet", "absent", "web"):
        inner = "".join(
            fact_row(b) if b["primitive"] == "fact" else p_absent(b) for b in blocks
        )
        return f'<dl class="sheet" data-slot="{name}">{inner}</dl>'
    # Pictures next to each other in a slot form one justified run.
    parts, run = [], []
    for b in blocks:
        markup = PRIMITIVES[b["primitive"]](b)
        if markup.startswith('<figure class="m ') and name not in (
            "run",
            "feature",
            "band",
            "hero",
        ):
            run.append(markup)
            continue
        if run:
            parts.append(f'<div class="run small">{"".join(run)}</div>')
            run = []
        parts.append(markup)
    if run:
        parts.append(f'<div class="run small">{"".join(run)}</div>')
    inner = "".join(parts)
    if name in ("run", "feature", "band"):
        inner = f'<div class="{name}">{inner}</div>'
    return f'<div data-slot="{name}">{inner}</div>'


def prose_slot(sp):
    """The prose of a spread: the first paragraph of its article section."""
    pt = sp.get("prose_text")
    if not pt:
        return ""
    return (
        f'<div class="prose-slot" lang="{e(pt["lang"])}"><p>{pt["html"]}</p>'
        f'<a class="read" href="{e(pt["href"])}">{e(pt["title"])} ↗</a></div>'
    )


def spread_html(sp):
    title = (
        f"<h2>{t(sp['title'])}</h2>"
        if sp.get("title") and sp["id"] != "identity"
        else ""
    )
    slots = "".join(slot_html(n, c) for n, c in sp["slots"].items())
    links = " · ".join(
        part(l)
        for l in sp["prose_links"]
        if not sp.get("prose_text") or l.get("href") != sp["prose_text"]["href"]
    )
    prose = f'<p class="prose-links">{links}</p>' if links else ""
    if sp["template"] == "collapsed":
        rows = sum(len(c) if isinstance(c, list) else 1 for c in sp["slots"].values())
        return (
            f'<section class="spread t-collapsed" data-spread="{sp["id"]}"><details>'
            f"<summary>+{rows}</summary>{slots}</details></section>"
        )
    return (
        f'<section class="spread t-{sp["template"]}" data-spread="{sp["id"]}">{title}{prose_slot(sp)}'
        f"{slots}{prose}</section>"
    )


FALLBACK_JS = """
// Progressive enhancement only: opens and closes dialogs in browsers without
// the invoker commands API. The content does not need it.
if ( !( 'commandForElement' in HTMLButtonElement.prototype ) ) {
	document.addEventListener( 'click', function ( event ) {
		const button = event.target.closest( 'button[commandfor]' );
		if ( !button ) { return; }
		const target = document.getElementById( button.getAttribute( 'commandfor' ) );
		if ( !target ) { return; }
		if ( button.getAttribute( 'command' ) === 'close' ) { target.close(); } else { target.showModal(); }
	} );
}
"""


def render(qid, lang):
    global PROV
    view = json.loads(
        (HERE / "data" / "view" / f"{qid.lower()}.{lang}.json").read_text()
    )
    PROV = view.get("provenance_labels", {})
    DIALOGS.clear()
    spreads = "".join(spread_html(sp) for sp in view["spreads"])
    rest = view["unmatched_sections"]
    more = (
        (
            f'<section class="spread t-sheet" data-spread="evidence"><p class="prose-links">'
            f"{' · '.join(part(s) for s in rest)}</p></section>"
        )
        if rest
        else ""
    )
    footer = (
        f'<p class="provenance-note">Prototype. Composed by rule from Wikidata item '
        f'<a href="https://www.wikidata.org/wiki/{qid}">{qid}</a> ({e(view["composition"])}, '
        f"{e(view['schema'])}, {e(view['design'])}). The lead paragraph, the section paragraphs and the "
        f"section links come from Wikipedia. Pictures come from Wikimedia Commons; a W on a picture marks "
        f"one from an article, not from Wikidata; a W on a value marks a statement whose only reference is a "
        f"Wikipedia.</p>"
    )
    version = urllib.parse.quote(view["design"])
    page = f"""<!DOCTYPE html>
<html lang="{e(view["lang"])}" dir="{e(view["dir"])}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{e(view["page_title"])} — composed</title>
<link rel="stylesheet" href="design/tokens.css?v={version}">
<link rel="stylesheet" href="design/page.css?v={version}">
</head>
<body>
<header class="chrome"><div><span class="wordmark">Wikipedia</span><input type="search" placeholder="Search Wikipedia" aria-label="Search"></div></header>
<main class="page">
{spreads}{more}{footer}
</main>
{"".join(DIALOGS)}
<script>{FALLBACK_JS}</script>
</body>
</html>
"""
    out = HERE / f"{qid.lower()}-{lang}.html"
    out.write_text(page)
    print(f"wrote {out.name} ({len(page):,} B)")


if __name__ == "__main__":
    render(sys.argv[1], sys.argv[2])
