#!/usr/bin/env python3
"""The second-generation composer: one program, any item.

The first prototypes had one hand-written build script for each item. This
composer has no item-specific code. It reads the qjson data of an item, the
side data from fetch_v2.py, and the judgment in v2.toml and groups.toml, and
it decides everything else by rule:

1. Dimension. Each property goes to one region of the page: measure, space,
   time, names, relations, media or evidence. The datatype decides first, then
   the class of the property on Wikidata.
2. Block. What a statement becomes follows the shape of its data:
   - a visual, when it has a shape of its own: a picture, a video, a 3D model,
     a map, a drawing to scale, the picture of an item it points to;
   - a row of the fact sheet, when it is one fact: a number, a date, a name,
     one or two items, a range;
   - a panel, when it is text of its own length: a long list, a chart of
     counts, the peers as bars, the evidence.
3. Enrichment. Four rules add what one item cannot show by itself, and each
   one appears only above its threshold: scale against the container, the
   place among peers, the absences that peers make visible, and the
   provenance of each statement.
4. Layout. Each kind of block has the layout that suits it:
   - Visuals go into justified rows. Every visual keeps its own aspect ratio
     (from Commons), all visuals in a row share one height, and the row fills
     the width. Nothing is cropped and nothing is letterboxed. The best
     photograph of a region, by community assessment or by the kind of view,
     gets a feature row of its own.
   - Rows go into one fact sheet per region, which flows in columns.
   - Panels flow in columns at their own height.
   Nothing is stretched to match a neighbour, and the order in the page is
   the reading order.

Usage: python3 prototype/v2/compose_v2.py Q405 ig
"""

import json
import math
import re
import sys
import tomllib
import urllib.parse
from pathlib import Path

# The page kit and the qjson fixtures are those of the first generation.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "v1"))
import factoid_kit as kit  # noqa: E402
from factoid_kit import (commons, e, icon, label_of, num, qual, value_qid,  # noqa: E402
                         vmenu, wd, year_of, year_str)

HERE = Path(__file__).resolve().parent
V1 = HERE.parent / "v1"
ROOT = HERE.parent.parent
CONFIG = tomllib.loads((HERE / "v2.toml").read_text())
GROUPS_TOML = tomllib.loads((ROOT / "qrender" / "groups.toml").read_text())

FIXTURES = {
    ("Q405", "ig"): "q405-ig.sparql.json",
    ("Q3870", "gu"): "q3870-gu.sparql.json",
    ("Q41181", "hi"): "q41181-hi.sparql.json",
}

# Chrome words of each wiki, from its interface messages.
WORDS = {
    "ig": {"wordmark": "Wikipedia", "tagline": "njikotá édémédé nke onyobulạ",
           "search": "Chọọ Wikipedia", "donate": "Nye onyinye", "talk": "Ńkàtá"},
    "gu": {"wordmark": "વિકિપીડિયા",
           "tagline": '<img src="https://gu.wikipedia.org/static/images/mobile/copyright/wikipedia-tagline-gu.svg" alt="" height="14">',
           "search": "શોધો", "donate": "દાન આપો", "talk": "ચર્ચા"},
    "hi": {"wordmark": "विकिपीडिया",
           "tagline": '<img src="https://hi.wikipedia.org/static/images/mobile/copyright/wikipedia-tagline-hi.svg" alt="" height="14">',
           "search": "खोजें", "donate": "दान करें", "talk": "संवाद"},
}

DIMENSIONS = {d["id"]: d for d in CONFIG["dimensions"]}
DIM_ORDER = [d["id"] for d in CONFIG["dimensions"]]
COMPARE = CONFIG["compare"]["pids"]
RANGE_ENDS = CONFIG["range_ends"]
MEDIA = CONFIG["media"]

# groups.toml: which properties belong together, and which are footnotes.
GROUP_OF = {}
FOOTNOTE = set()
for _name, _g in GROUPS_TOML.get("groups", {}).items():
    for _p in _g.get("pids", []):
        if re.fullmatch(r"P\d+", _p):
            GROUP_OF.setdefault(_p, _name)
            if _g.get("footnote"):
                FOOTNOTE.add(_p)
FOOTNOTE |= {p for p, c in GROUPS_TOML.get("properties", {}).items() if c.get("footnote")}

# Structural properties that never make a block. P31 goes into the header.
IGNORED = {"P31", "P279", "P7452"}

AUDIO = (".ogg", ".oga", ".wav", ".mp3", ".flac", ".opus")
VIDEO = (".webm", ".ogv", ".mp4")

# References that only point back into Wikimedia, or at a guess, are not
# sources. A statement whose references are all of this kind is "imported".
NOT_A_SOURCE = {"P143", "P813", "P4656", "P887", "P3452"}

# The widths that the Wikimedia thumbnail servers keep. Any other width makes
# a new thumbnail on the first request.
STANDARD_WIDTHS = (120, 250, 330, 500, 960, 1280, 1920, 3840)

# The geometry of the justified rows, at the widest page. CSS keeps the
# proportions at every other width.
PAGE_WIDTH = 1248
GAP = 8
ROW_HEIGHT = 230          # visual rows in a region
SET_HEIGHT = 170          # the pictured values of one property
FEATURE_MAX = 560         # a feature row is never taller than this


class Page:
    """Everything the rules read, loaded once."""

    def __init__(self, qid, lang):
        self.qid, self.lang = qid, lang
        kit.load(V1 / "data" / FIXTURES[(qid, lang)])
        self.item = kit.item()
        self.sts = [s for s in kit.statements() if s["rank"] != "Deprecated"]
        self.bundle = json.loads((HERE / "data" / f"{qid.lower()}-{lang}.json").read_text())
        self.by_pid = {}
        for s in self.sts:
            self.by_pid.setdefault(s["pid"], []).append(s)
        peers = self.bundle.get("peers") or {}
        self.peer_count = len(peers.get("members", []))
        self.peer_freq = peers.get("frequency", {})


P = None  # the page being composed


# ------------------------------------------------------------ language marks

def t(text, lang):
    """A string with its language. Fallback strings carry a lang attribute,
    for screen readers and for the quiet mark in the stylesheet. A "mul"
    label is the same in every language, so it is not a fallback."""
    if lang and lang not in (P.lang, "mul"):
        return f'<span lang="{e(lang)}">{e(text)}</span>'
    return e(text)


def plain(html_text):
    return re.sub(r"<[^>]+>", "", html_text)


def pl(st):
    return t(st["plabel"], st["plabel_lang"])


def vl(st):
    return t(label_of(st), st.get("vlabel_lang"))


def unit(st):
    if not st.get("unit") or st["unit"] in ("1", "otu"):
        return ""
    return t(st["unit"], st.get("unit_lang"))


def amount(st):
    u = unit(st)
    return f'{num(st["value"])}' + (f' <span class="u">{u}</span>' if u else "")


def qnote(st, skip=()):
    """The qualifiers of a statement as a short note. Temporal qualifiers
    become a span of years; a "some value" qualifier is not shown."""
    bits = []
    span = kit.timespan(st, prefix=False)
    if span:
        bits.append(e(span))
    elif qual(st, "P585"):
        bits.append(e(year_str(qual(st, "P585"))))
    for q in st["quals"]:
        if q["pid"] in ("P580", "P582", "P585") or q["pid"] in skip:
            continue
        if q["value"].startswith("http://www.wikidata.org/.well-known/genid"):
            continue
        if re.match(r"^-?\d{4,}-\d\d-\d\dT", q["value"]):
            bits.append(e(year_str(q["value"])))
            continue
        bits.append(t(q["value"], q["label_lang"] or q.get("lang")))
    return " · ".join(bits)


SUPERSCRIPT = str.maketrans("-0123456789", "⁻⁰¹²³⁴⁵⁶⁷⁸⁹")


def sci(v, digits=3):
    """A number a reader can take in: grouped digits in the usual range,
    powers of ten outside it. Digits and the times sign need no
    translation."""
    if v == 0:
        return "0"
    exp = math.floor(math.log10(abs(v)))
    if -3 <= exp < 7:
        return num(round(v, max(0, digits - 1 - exp)))
    mant = v / 10 ** exp
    return f"{mant:.{digits - 1}g} × 10{str(exp).translate(SUPERSCRIPT)}"


def concept(qid):
    c = P.bundle["concepts"].get(qid) or {"label": qid, "lang": "en"}
    return t(c["label"], c["lang"])


def item_link(st, text=None):
    q = value_qid(st)
    inner = text or vl(st)
    return f'<a href="{wd(q)}">{inner}</a>' if q else inner


def file_of(url):
    return urllib.parse.unquote(url.rsplit("/", 1)[-1])


def info_of(name):
    return P.bundle["commons"].get(name) or {}


def thumb(url, width):
    """A Commons thumbnail at the smallest standard width that is enough."""
    w = next((s for s in STANDARD_WIDTHS if s >= width), STANDARD_WIDTHS[-1])
    return commons(url, w)


def aspect_of(name, default=4 / 3):
    info = info_of(name)
    if info.get("width") and info.get("height"):
        return info["width"] / info["height"]
    return default


def credit(name):
    info = info_of(name)
    bits = [x for x in (info.get("artist"), info.get("license")) if x]
    return f'<span class="credit">{e(" · ".join(bits))}</span>' if bits else ""


# ---------------------------------------------------------------- provenance

def provenance(sts):
    """Count statements by the kind of their references."""
    counts = {"sourced": 0, "imported": 0, "none": 0}
    refs = P.bundle["references"]
    for s in sts:
        sets = refs.get(s["id"], [])
        if not sets:
            counts["none"] += 1
        elif any(set(r) - NOT_A_SOURCE for r in sets):
            counts["sourced"] += 1
        else:
            counts["imported"] += 1
    return counts


def prov_label(kind):
    c = P.bundle["concepts"]
    if kind == "sourced":
        return " / ".join(t(c[p]["label"], c[p]["lang"]) for p in ("P248", "P854") if p in c)
    if kind == "imported":
        return t(c["P143"]["label"], c["P143"]["lang"]) if "P143" in c else "P143"
    return "—"


def prov_mark(sts):
    """One glyph: filled when every statement has a source, half when some
    have, dotted when the only references point back into a Wikipedia, a ring
    when there is no reference at all."""
    if not sts:
        return ""
    counts = provenance(sts)
    total = sum(counts.values())
    if counts["sourced"] == total:
        state = "sourced"
    elif counts["sourced"]:
        state = "partial"
    elif counts["imported"]:
        state = "imported"
    else:
        state = "none"
    tip = " · ".join(f'{plain(prov_label(k))}: {n}' for k, n in counts.items() if n)
    return f'<span class="prov prov-{state}" title="{e(tip)}"></span>'


def importance(sts, extra=0.0):
    """How much a block matters, from the data: how common its property is
    among peers, how well it is sourced, how many values it has."""
    s = extra
    pids = {x["pid"] for x in sts}
    if P.peer_count and pids:
        s += 2 * max(P.peer_freq.get(p, 0) for p in pids) / P.peer_count
    s += math.log2(1 + provenance(sts)["sourced"]) if sts else 0
    s += 0.5 * math.log2(1 + len(sts))
    return s


def dimension(pid, ptype):
    if ptype in ("ExternalId", "Url") or pid in FOOTNOTE:
        return "evidence"
    if ptype == "CommonsMedia":
        return "media"
    if ptype == "Quantity":
        return "measure"
    if ptype == "Time":
        return "time"
    if ptype == "GlobeCoordinate":
        return "space"
    if ptype in ("Monolingualtext", "String"):
        return "names"
    classes = P.bundle["property_classes"].get(pid, [])
    for dim in DIM_ORDER:
        if any(CONFIG["classes"].get(c) == dim for c in classes):
            return dim
    return "relations"


# ------------------------------------------------------------------ blocks

class Visual:
    """A block with a shape of its own. The aspect ratio is width / height
    of the picture, the video or the drawing."""

    def __init__(self, kind, sts, aspect, inner, caption, score=0.0, feature=None):
        self.kind, self.sts, self.aspect = kind, sts, max(0.3, min(aspect, 5.0))
        self.inner, self.caption, self.score = inner, caption, score
        self.feature = feature      # a feature score, or None when it cannot be one

    def html(self, px_width):
        inner = self.inner(px_width) if callable(self.inner) else self.inner
        menu = vmenu(plain(self.caption)) if self.sts else ""
        mark = prov_mark(self.sts)
        return (f'<figure class="v k-{self.kind}" style="--a:{self.aspect:.4f}">{inner}{menu}'
                f'<figcaption>{self.caption}{mark}</figcaption></figure>')


class Row:
    """One fact in the fact sheet."""

    def __init__(self, label, value, sts, score=0.0, group=None, kind="fact"):
        self.label, self.value, self.sts = label, value, sts
        self.score, self.group, self.kind = score, group, kind

    def html(self):
        return (f'<div class="row r-{self.kind}"><dt>{self.label}</dt><dd>{self.value}</dd>'
                f'{prov_mark(self.sts)}{vmenu(plain(self.label)) if self.sts else ""}</div>')


class Panel:
    """A block of text or a chart, at the height of its content."""

    def __init__(self, kind, sts, title, body, score=0.0, wide=False):
        self.kind, self.sts, self.title, self.body = kind, sts, title, body
        self.score, self.wide = score, wide

    def html(self):
        head = (f'<header><h3>{self.title}</h3>{prov_mark(self.sts)}'
                f'{vmenu(plain(self.title)) if self.sts else ""}</header>') if self.title else ""
        return f'<article class="panel2 k-{self.kind}{" wide" if self.wide else ""}">{head}<div class="body">{self.body}</div></article>'


class Region:
    def __init__(self, dim):
        self.dim = dim
        self.visuals, self.sets, self.rows, self.panels = [], [], [], []

    def statements(self):
        out = [s for v in self.visuals for s in v.sts] + [s for r in self.rows for s in r.sts]
        out += [s for p in self.panels for s in p.sts] + [s for _, vs, _ in self.sets for v in vs for s in v.sts]
        return {id(s) for s in out}

    def small(self):
        return not self.visuals and not self.sets and len(self.rows) <= 6 and \
            len([p for p in self.panels if not p.wide]) <= 1 and not any(p.wide for p in self.panels)


# ------------------------------------------------------- visuals and media

def image_visual(st, kind="image"):
    """The item's own picture: a photograph fills its box, a drawing (SVG)
    is shown whole on white. The feature score comes from the community's
    assessment on Commons and from the kind of view."""
    name = file_of(st["value"])
    info = info_of(name)
    svg = name.lower().endswith(".svg")
    kind = "drawing" if svg else kind
    feature = None
    if not svg and info.get("width"):
        mp = info["width"] * info["height"] / 1e6
        views = MEDIA["views"]
        assessed = sum(MEDIA["assessments"].get(a, 0) for a in info.get("assessments", []))
        if mp >= MEDIA["min_megapixels"] and (assessed or st["pid"] in views):
            pref = (len(views) - views.index(st["pid"])) / len(views) if st["pid"] in views else 0
            feature = 2 * assessed + 2 * pref + 0.5 * math.log2(mp)
    star = ""
    if info.get("assessments"):
        star = f'<span class="assessed" title="{e(", ".join(info["assessments"]))}">★</span>'
    def inner(px):
        return (f'<a href="https://commons.wikimedia.org/wiki/File:{e(name.replace(" ", "_"))}">'
                f'<img src="{e(thumb(st["value"], px * 1.5))}" alt="" loading="lazy"></a>')
    caption = f'<span class="vp">{pl(st)}{star}</span>{credit(name)}'
    return Visual(kind, [st], aspect_of(name), inner, caption,
                  score=importance([st], 1 + (feature or 0)), feature=feature)


def item_picture(st, with_property=True):
    """The picture of an item that a statement points to: its P18, or else
    the page image of its article, which is not Wikidata and says so."""
    fallback = None
    if st.get("vimg"):
        url, name = st["vimg"], file_of(st["vimg"])
    else:
        fallback = P.bundle.get("page_images", {}).get(value_qid(st) or "")
        if not fallback:
            return None
        name = fallback["file"]
        url = "http://commons.wikimedia.org/wiki/Special:FilePath/" + urllib.parse.quote(name)
    svg = name.lower().endswith(".svg")
    source = (f'<span class="from" title="{e(fallback["source"])}">W</span>' if fallback else "")
    note = qnote(st)
    def inner(px):
        return f'<img src="{e(thumb(url, px * 1.5))}" alt="" loading="lazy">'
    caption = ((f'<span class="vp">{pl(st)}</span>' if with_property else "")
               + f'<b>{item_link(st)}</b>{source}'
               + (f'<span class="vn">{note}</span>' if note else ""))
    lead = st["pid"] in MEDIA["lead_portraits"] and not svg
    return Visual("drawing" if svg else "portrait", [st], aspect_of(name, 3 / 4), inner, caption,
                  score=importance([st], 3 if lead else 1.5), feature=5.0 if lead else None)


def placeholder(st):
    caption = f'<b>{item_link(st)}</b>'
    inner = f'<button class="add-image" type="button">{icon("add-photo")}Add image</button>'
    return Visual("empty", [st], 4 / 3, inner, caption, score=importance([st]))


def media_blocks(sts, region):
    audio = []
    for st in sts:
        name = file_of(st["value"])
        low = name.lower()
        info = info_of(name)
        if low.endswith(AUDIO):
            audio.append(st)
        elif low.endswith(VIDEO):
            size = f' · {info["size"] / 1e6:.0f} MB' if info.get("size") else ""
            inner = (f'<video controls preload="none" poster="{e(thumb(st["value"], 960))}" '
                     f'src="{e(commons(st["value"]))}"></video>')
            caption = f'<span class="vp">{pl(st)}</span>{credit(name)}<span class="credit">{e(size)}</span>'
            region.visuals.append(Visual("video", [st], aspect_of(name, 16 / 9), inner, caption,
                                         score=importance([st], 3)))
        elif low.endswith(".stl"):
            size = f'{info["size"] / 1e6:.1f} MB' if info.get("size") else ""
            inner = (f'<div class="model3d" data-src="{e(info.get("url", ""))}">'
                     f'<div class="model3d-stage"><img src="{e(thumb(st["value"], 960))}" alt="{e(name)}" loading="lazy"></div>'
                     f'<button class="model3d-load" type="button" hidden>{icon("3d_rotation")}3D · {e(size)}</button></div>')
            caption = (f'<span class="vp">{pl(st)}</span><span class="vn">{e(info.get("description", "")[:80])}</span>'
                       f'{credit(name)}')
            region.visuals.append(Visual("model3d", [st], aspect_of(name, 16 / 9), inner, caption,
                                         score=importance([st], 3)))
        else:
            region.visuals.append(image_visual(st))
    if audio:
        body = '<div class="audios">' + "".join(
            f'<div class="audio-row"><span>{qnote(a) or pl(a)}</span>'
            f'<audio controls preload="none" src="{e(commons(a["value"]))}"></audio></div>'
            for a in audio) + "</div>"
        region.panels.append(Panel("audio", audio, pl(audio[0]), body, importance(audio, 1)))


def map_block(st, region):
    m = re.search(r"(?:entity/(Q\d+)> )?Point\(([-\d.]+) ([-\d.]+)\)", st["value"])
    globe, lon, lat = m.group(1), float(m.group(2)), float(m.group(3))
    where = f'{lat:g}, {lon:g}'
    if globe and globe != "Q2":
        # Not on the Earth. The map is the item's own equirectangular
        # texture when it has one; otherwise there is no map, only numbers.
        texture = (P.by_pid.get("P4640") or [None])[0]
        if globe != P.qid or not texture:
            region.rows.append(Row(pl(st), where, [st], importance([st])))
            return
        x, y = (lon + 180) / 360 * 100, (90 - lat) / 180 * 100
        def inner(px):
            return (f'<div class="frame"><img src="{e(thumb(texture["value"], px * 1.5))}" alt="">'
                    f'<span class="pin" style="left:{x:.2f}%;top:{y:.2f}%"></span></div>')
        region.visuals.append(Visual("map", [st], 2.0, inner,
                                     f'<span class="vp">{pl(st)}</span><span class="vn">{where} · {pl(texture)}</span>',
                                     score=importance([st], 4)))
        return
    # On the Earth: a static tile, zoomed so that the area of the item, when
    # known, fills about a third of the frame.
    width, height = 640, 400
    area = P.bundle["quantities"].get(P.qid, {}).get("values", {}).get("P2046")
    zoom = 11
    if area:
        # 156543 m is one pixel at zoom 0 on the equator.
        zoom = round(math.log2(width * 156543 * math.cos(math.radians(lat)) / (3 * math.sqrt(area["amount"]))))
        zoom = max(3, min(14, zoom))
    tile = f"https://maps.wikimedia.org/img/osm-intl,{zoom},{lat},{lon},{width}x{height}@2x.png"
    inner = (f'<div class="frame"><img src="{tile}" alt="">'
             f'<span class="pin" style="left:50%;top:50%"></span></div>')
    region.visuals.append(Visual("map", [st], width / height, inner,
                                 f'<span class="vp">{pl(st)}</span><span class="vn">{where}</span>',
                                 score=importance([st], 4)))


def orbit_block(sts, region):
    """Role renderer: an orbit drawn to scale from periapsis and apoapsis."""
    peri = next(s for s in sts if s["pid"] == "P2244")
    apo = next(s for s in sts if s["pid"] == "P2243")
    q, Q = float(peri["value"]), float(apo["value"])
    a, c = (q + Q) / 2, (Q - q) / 2
    b = math.sqrt(a * a - c * c)
    w, h, pad = 440, 300, 30
    # Fit both axes: this orbit is almost a circle, so height limits it.
    s = min((w - 2 * pad) / (2 * a), (h - 2 * pad) / (2 * b))
    cx, cy = w / 2, h / 2
    fx = cx - c * s
    parent = (P.by_pid.get("P397") or [None])[0]
    parent_name = e(label_of(parent)) if parent else ""
    inner = f"""<svg class="orbit" viewBox="0 0 {w} {h}" role="img" aria-label="{e(plain(pl(peri)))} {num(q)}, {e(plain(pl(apo)))} {num(Q)}">
  <ellipse cx="{cx:.1f}" cy="{cy:.1f}" rx="{a * s:.1f}" ry="{b * s:.1f}" class="path"/>
  <line x1="{cx - a * s:.1f}" y1="{cy}" x2="{cx + a * s:.1f}" y2="{cy}" class="axis"/>
  <circle cx="{fx:.1f}" cy="{cy}" r="8" class="parent"/>
  <text x="{fx:.1f}" y="{cy + 24}" text-anchor="middle" class="lbl">{parent_name}</text>
  <circle cx="{cx - a * s:.1f}" cy="{cy}" r="5" class="body"/><circle cx="{cx + a * s:.1f}" cy="{cy}" r="5" class="body"/>
  <text x="{cx - a * s + 10:.1f}" y="{cy - 10}" class="lbl">{num(q)}</text>
  <text x="{cx + a * s - 10:.1f}" y="{cy - 10}" text-anchor="end" class="lbl">{num(Q)}</text>
</svg>"""
    caption = f'<span class="vp">{pl(peri)} · {pl(apo)}</span><span class="vn">{unit(peri)}</span>'
    region.visuals.append(Visual("chart", sts, w / h, inner, caption, score=importance(sts, 4)))


# ------------------------------------------------------------ facts, lists

def is_range(sts):
    ends = {RANGE_ENDS.get(q.get("qid")) for s in sts for q in s["quals"]}
    return len(sts) == 2 and {"min", "max"} <= ends


def is_series(sts):
    years = [year_of(qual(s, "P585")) for s in sts if qual(s, "P585")]
    return len(sts) >= 3 and len(years) == len(sts) and len(set(years)) == len(years)


def fmt_time(s):
    y = year_of(s["value"])
    # Deep time: a signed year needs no era word in any language.
    if y is not None and abs(y) >= 100000:
        return ("−" if y < 0 else "") + num(abs(y))
    return e(kit.fmt_date(s["value"], s.get("precision")))


def chip(st):
    img = f'<img src="{e(thumb(st["vimg"], 60))}" alt="" loading="lazy">' if st.get("vimg") else ""
    if st["ptype"] == "Monolingualtext":
        tag = f'<span class="note">{e(st["vlang"])}</span>' if st.get("vlang") else ""
        return f'<span class="chip native" lang="{e(st.get("vlang") or "")}">{e(st["value"])}{tag}</span>'
    note = qnote(st)
    label = item_link(st) if value_qid(st) else (
        f'<a href="{e(st["value"])}">{e(st["value"])}</a>' if st["ptype"] == "Url" else e(label_of(st)))
    return (f'<span class="chip">{img}<span>{label}</span>'
            + (f'<span class="note">{note}</span>' if note else "") + "</span>")


def quantity_blocks(pid, sts, region):
    first = sts[0]
    score = importance(sts)
    group = GROUP_OF.get(pid)
    if len(sts) == 1:
        note = qnote(first)
        value = amount(first) + (f' <span class="note">{note}</span>' if note else "")
        region.rows.append(Row(pl(first), value, sts, score, group, "stat"))
    elif is_range(sts):
        lo = next(s for s in sts if any(RANGE_ENDS.get(q.get("qid")) == "min" for q in s["quals"]))
        hi = next(s for s in sts if s is not lo)
        top = float(hi["value"]) * 1.15
        a, b = float(lo["value"]) / top * 100, float(hi["value"]) / top * 100
        value = (f'{num(lo["value"])} – {amount(hi)}<span class="mini-range"><span style="inset-inline-start:{a:.1f}%;'
                 f'inline-size:{b - a:.1f}%"></span></span>')
        region.rows.append(Row(pl(first), value, sts, score, group, "range"))
    elif is_series(sts):
        pts = sorted(((year_of(qual(s, "P585")), float(s["value"])) for s in sts))
        top = max(v for _, v in pts)
        bars = "".join(
            f'<div class="bar-col"><span class="bar-value">{sci(v)}</span>'
            f'<span class="bar" style="block-size:{v / top * 100:.1f}%"></span>'
            f'<span class="bar-year">{y}</span></div>' for y, v in pts)
        region.panels.append(Panel("series", sts, pl(first), f'<div class="bars">{bars}</div>', score + 2))
    else:
        ordered = sorted(sts, key=lambda s: float(s["value"]))
        value = "<br>".join(
            amount(s) + f' <span class="note">{qnote(s)}{" ★" if s["rank"] == "Preferred" else ""}</span>'
            for s in ordered)
        region.rows.append(Row(pl(first), value, sts, score, group, "variants"))


def item_blocks(pid, sts, region):
    """Item and text values. Pictured values become visuals: alone when there
    are one or two, as a set with its own heading when there are 3 to 12. A
    longer enumeration is a list of chips, and a list is not pictures."""
    first = sts[0]
    score = importance(sts)
    items = first["ptype"] == "WikibaseItem"
    # Curation data (evidence) is listed, not pictured.
    pictures = [item_picture(s) if items and region.dim != "evidence" else None for s in sts]
    pictured = [p for p in pictures if p]
    if items and len(sts) <= 2 and len(pictured) == len(sts):
        region.visuals.extend(pictured)
        return
    if items and 3 <= len(sts) <= 12 and len(pictured) * 2 >= len(sts) and region.dim != "evidence":
        visuals = [item_picture(s, with_property=False) or placeholder(s) for s in sts]
        title = f'{pl(first)} <span class="count">{len(sts)}</span>'
        region.sets.append((title, visuals, score + 1))
        return
    labels = sum(len(label_of(s)) for s in sts)
    if len(sts) <= 6 and labels <= 70:
        value = " ".join(chip(s) for s in sts) if len(sts) > 1 else chip(sts[0])
        region.rows.append(Row(pl(first), value, sts, score, GROUP_OF.get(pid), "items"))
        return
    shown = sts[:24]
    body = '<div class="chips">' + "".join(chip(s) for s in shown)
    if len(sts) > len(shown):
        did = f"dlg-{pid}"
        kit.add_dialog(did, plain(first["plabel"]), '<div class="chips">' + "".join(chip(s) for s in sts) + "</div>")
        body += kit.more_button(did, f"+{len(sts) - len(shown)}", "info")
    body += "</div>"
    region.panels.append(Panel("chips", sts, f'{pl(first)} <span class="count">{len(sts)}</span>', body, score))


# ------------------------------------------------------------- enrichment

def peer_blocks(regions):
    """The item among its peers, on the two comparable properties that the
    most peers have. A property with a shape in v2.toml (discs for a radius)
    comes first, because its picture says more than a bar."""
    peers = P.bundle.get("peers")
    if not peers:
        return
    values = peers.get("quantities", {})
    def count(p):
        return sum(1 for r in values.values() if p in r["values"])
    candidates = [p for p in COMPARE if p in P.by_pid and count(p) >= 5]
    candidates.sort(key=lambda p: (p not in CONFIG["peer_shape"], -count(p)))
    chosen = []
    for p in candidates:
        # A radius and a diameter show the same thing; keep one of them.
        if not ({p, *chosen} >= {"P2120", "P2386"}):
            chosen.append(p)
    for pid in chosen[:2]:
        panel = peer_panel(pid, peers, values)
        if panel:
            regions.setdefault("measure", Region("measure")).panels.append(panel)


def peer_panel(pid, peers, values):
    own = P.bundle["quantities"].get(P.qid, {}).get("values", {}).get(pid)
    if not own:
        return None
    members = [(q, r["values"][pid]["amount"], r) for q, r in values.items() if pid in r["values"]]
    if P.qid not in {q for q, _, _ in members}:
        members.append((P.qid, own["amount"], {"label": P.item["label"], "label_lang": P.item["label_lang"],
                                                 "image": (P.by_pid.get("P18") or [{}])[0].get("value")}))
    members.sort(key=lambda m: -m[1])
    rank = [q for q, _, _ in members].index(P.qid) + 1
    top = members[0][1]
    cls = peers.get("class_label") or {"label": peers["class"], "lang": "en"}
    first = P.by_pid[pid][0]
    title = (f'{pl(first)} · <a href="{wd(peers["class"])}">{t(cls["label"], cls["lang"])}</a>'
             f' <span class="count">{rank} / {len(members)}</span>')
    shown = members[:9]
    if P.qid not in {q for q, _, _ in shown}:
        shown = members[:8] + [m for m in members if m[0] == P.qid]
    if CONFIG["peer_shape"].get(pid) == "disc":
        discs = []
        for q, v, r in shown:
            d = max(3, v / top * 110)
            me = " me" if q == P.qid else ""
            img = (f'<img src="{e(thumb(r["image"], 250))}" alt="">' if r.get("image") and d >= 22 else "")
            discs.append(f'<a class="disc{me}" href="{wd(q)}" title="{e(r["label"])}: {sci(v)}">'
                         f'<span class="ball" style="inline-size:{d:.1f}px">{img}</span>'
                         f'<span class="name">{t(r["label"], r.get("label_lang"))}</span></a>')
        return Panel("peers", P.by_pid[pid], title, f'<div class="discs">{"".join(discs)}</div>', 10, wide=True)
    bars = "".join(
        f'<a class="peer{" me" if q == P.qid else ""}" href="{wd(q)}">'
        f'<span class="name">{t(r["label"], r.get("label_lang"))}</span>'
        f'<span class="track"><span style="inline-size:{v / top * 100:.1f}%"></span></span>'
        f'<span class="val">{sci(v)}</span></a>' for q, v, r in shown)
    return Panel("peers", P.by_pid[pid], title, f'<div class="peerbars">{bars}</div>', 8)


def compare_panel():
    """The item against the item that contains it, on the comparable
    properties both have: side by side for "scale", as a part of the whole
    for "share"."""
    cont = P.bundle.get("container") or {}
    mode = CONFIG["containers"].get(cont.get("pid") or "")
    qs = P.bundle["quantities"]
    if not mode or cont.get("qid") not in qs or P.qid not in qs:
        return None
    mine, theirs = qs[P.qid]["values"], qs[cont["qid"]]["values"]
    shared = [p for p in COMPARE if p in mine and p in theirs and theirs[p]["amount"]]
    if not shared:
        return None
    them = qs[cont["qid"]]
    them_name = t(them["label"], them.get("label_lang"))
    me_name = e(P.item["label"])
    parts = []
    radius = next((p for p in ("P2120", "P2386") if p in shared), None)
    if mode == "scale" and radius:
        r1, r2 = mine[radius]["amount"], theirs[radius]["amount"]
        big = max(r1, r2)
        my_img = (P.by_pid.get("P18") or [{}])[0].get("value")
        def ball(r, img, name, q):
            d = r / big * 120
            pic = f'<img src="{e(thumb(img, 250))}" alt="">' if img else ""
            return (f'<a class="disc" href="{wd(q)}"><span class="ball" style="inline-size:{d:.1f}px">{pic}</span>'
                    f'<span class="name">{name}</span></a>')
        parts.append(f'<div class="discs pair">{ball(r1, my_img, me_name, P.qid)}{ball(r2, them.get("image"), them_name, cont["qid"])}</div>')
    for p in shared:
        if p == radius:
            continue
        pct = mine[p]["amount"] / theirs[p]["amount"] * 100
        st = P.by_pid[p][0]
        parts.append(f'<div class="share" title="{sci(mine[p]["amount"])} / {sci(theirs[p]["amount"])} {e(mine[p]["unit"])}">'
                     f'<span class="name">{pl(st)}</span><span class="track"><span style="inline-size:{min(pct, 100):.2f}%"></span></span>'
                     f'<span class="val">{sci(pct, 2)} %</span></div>')
    rel = P.by_pid[cont["pid"]][0]
    title = f'{me_name} · {pl(rel)}: <a href="{wd(cont["qid"])}">{them_name}</a>'
    return Panel("compare", [P.by_pid[p][0] for p in shared], title, "".join(parts))


def ghost_rows(regions):
    """Properties that most peers have and this item lacks. An absence is a
    fact about the data, so it is shown, with the count that makes it one."""
    peers = P.bundle.get("peers")
    if not peers:
        return
    cls = peers.get("class_label") or {"label": peers["class"], "lang": "en"}
    types = {"wikibase-item": "WikibaseItem", "time": "Time", "quantity": "Quantity",
             "string": "String", "commonsMedia": "CommonsMedia", "external-id": "ExternalId"}
    for pid, lab in sorted(peers.get("missing_labels", {}).items(), key=lambda kv: -peers["frequency"][kv[0]]):
        n = peers["frequency"][pid]
        dim = dimension(pid, types.get(peers.get("missing_types", {}).get(pid), "WikibaseItem"))
        value = (f'<span class="ghost">—</span> <span class="note">{n} / {P.peer_count} '
                 f'{t(cls["label"], cls["lang"])}</span>'
                 f'<button class="add" type="button" aria-label="Add">{icon("edit")}</button>')
        regions.setdefault(dim, Region(dim)).rows.append(
            Row(t(lab["label"], lab["lang"]), value, [], -5, None, "ghost"))


# ------------------------------------------------------------ composition

def build_regions():
    hero_image = next((s for s in P.by_pid.get("P18", []) if s["rank"] == "Preferred"), None) or \
        (P.by_pid.get("P18") or [None])[0]
    consumed = {id(hero_image)} if hero_image else set()
    consumed |= {id(s) for s in P.by_pid.get("P948", [])}
    regions = {}
    def region(dim):
        return regions.setdefault(dim, Region(dim))

    if "P2243" in P.by_pid and "P2244" in P.by_pid:
        sts = P.by_pid["P2243"][:1] + P.by_pid["P2244"][:1]
        orbit_block(sts, region("measure"))
        consumed |= {id(s) for s in sts}

    media = []
    for pid, sts in P.by_pid.items():
        live = [s for s in sts if id(s) not in consumed]
        if pid in IGNORED or not live:
            continue
        ptype = live[0]["ptype"]
        dim = dimension(pid, ptype)
        if ptype == "CommonsMedia":
            media.extend(live)
        elif ptype == "ExternalId":
            continue
        elif ptype == "Quantity":
            quantity_blocks(pid, live, region(dim))
        elif ptype == "Time":
            region(dim).rows.append(Row(pl(live[0]), "<br>".join(
                fmt_time(s) + (f' <span class="note">{qnote(s)}</span>' if qnote(s) else "") for s in live),
                live, importance(live), GROUP_OF.get(pid), "time"))
        elif ptype == "GlobeCoordinate":
            map_block(live[0], region(dim))
        else:
            item_blocks(pid, live, region(dim))
    media_blocks(media, region("media"))
    peer_blocks(regions)
    ghost_rows(regions)
    return hero_image, regions


# ------------------------------------------------------------------ layout

def justify(visuals, target, width=PAGE_WIDTH):
    """Break a list of visuals into rows whose height is close to a target.
    Each row fills the width exactly when the aspects are added up; the
    last row keeps the target height and does not stretch."""
    rows, row = [], []
    for v in visuals:
        if row:
            now = (width - GAP * (len(row) - 1)) / sum(x.aspect for x in row)
            nxt = (width - GAP * len(row)) / (sum(x.aspect for x in row) + v.aspect)
            if now <= target * 1.3 and abs(now - target) <= abs(nxt - target):
                rows.append((row, True))
                row = []
        row.append(v)
    if row:
        full = (width - GAP * (len(row) - 1)) / sum(x.aspect for x in row)
        rows.append((row, full <= target * 1.3))
    return rows


def row_html(row, filled, target, width=PAGE_WIDTH):
    total = sum(v.aspect for v in row)
    height = (width - GAP * (len(row) - 1)) / total if filled else target
    cap = "" if filled else f' style="max-inline-size:{total * target + GAP * (len(row) - 1):.0f}px"'
    return (f'<div class="jrow"{cap}>'
            + "".join(v.html(v.aspect * height) for v in row) + "</div>")


def feature_row(feature, others):
    """The best visual of a region, big and whole. Beside it, visuals stack
    in pairs (a pair of aspects a and b has the aspect 1 / (1/a + 1/b)) or
    stand alone, until the row fills the width at a height no taller than
    FEATURE_MAX. Every visual keeps its own shape."""
    units, rest = [], list(others)
    total = feature.aspect
    def height(total, n):
        return (PAGE_WIDTH - GAP * n) / total
    while rest and height(total, len(units)) > FEATURE_MAX:
        if len(rest) >= 2 and len(units) == 0:
            pair = rest[:2]
            aspect = 1 / (1 / pair[0].aspect + 1 / pair[1].aspect)
            units.append(("stack", pair, aspect))
            rest = rest[2:]
        else:
            units.append(("one", [rest[0]], rest[0].aspect))
            rest = rest[1:]
        total += units[-1][2]
    h = min(height(total, len(units)), FEATURE_MAX)
    cap = "" if h < FEATURE_MAX else f' style="max-inline-size:{total * FEATURE_MAX + GAP * len(units):.0f}px"'
    html = feature.html(feature.aspect * h)
    for kind, vs, aspect in units:
        if kind == "one":
            html += vs[0].html(aspect * h)
        else:
            html += (f'<div class="stack" style="--a:{aspect:.4f}">'
                     + "".join(f'<div class="cell" style="--g:{1 / v.aspect:.4f}">{v.html(aspect * h)}</div>'
                               for v in vs) + "</div>")
    solo = " solo" if not units else ""
    return f'<div class="jrow feature{solo}"{cap}>{html}</div>', rest


LEAD_HEIGHT = 440          # a lead visual beside the rest of its region


def visuals_html(reg, width=PAGE_WIDTH, rest_only=None):
    """Justified rows, after the feature row when the region has one."""
    ordered = rest_only if rest_only is not None else sorted(reg.visuals, key=lambda v: -v.score)
    out = []
    if rest_only is None:
        feature, companions = choose_feature(ordered)
        if feature and companions:
            html, ordered = feature_row(feature, [v for v in ordered if v is not feature])
            out.append(html)
    out += [row_html(r, f, ROW_HEIGHT, width) for r, f in justify(ordered, ROW_HEIGHT, width)]
    return '<div class="visuals">' + "".join(out) + "</div>" if out else ""


def choose_feature(ordered):
    """The visual with the best feature score, and whether the visuals left
    are worthy companions in its row. In the media region every picture is a
    view, so they are. Elsewhere a lead portrait stands beside the text of
    its region instead, so that the size itself says which one leads."""
    candidates = [v for v in ordered if v.feature is not None]
    if not candidates:
        return None, False
    feature = max(candidates, key=lambda v: v.feature)
    others = [v for v in ordered if v is not feature]
    worthy = feature.kind != "portrait" and len(others) >= 1
    return feature, worthy


def sets_html(reg, width=PAGE_WIDTH):
    out = []
    for title, visuals, _ in sorted(reg.sets, key=lambda s: -s[2]):
        rows = "".join(row_html(r, f, SET_HEIGHT, width) for r, f in justify(visuals, SET_HEIGHT, width))
        out.append(f'<div class="vset"><h3>{title}</h3>{rows}</div>')
    return "".join(out)


def sheet_html(rows):
    """One fact sheet. Rows of one groups.toml group stay together; groups
    and single rows are ordered by their best score; absences come last."""
    if not rows:
        return ""
    groups = {}
    for r in rows:
        groups.setdefault(r.group or id(r), []).append(r)
    ordered = sorted(groups.values(), key=lambda g: -max(r.score for r in g))
    return '<dl class="sheet">' + "".join(r.html() for g in ordered for r in g) + "</dl>"


def region_html(reg):
    d = DIMENSIONS[reg.dim]
    count = len(reg.statements())
    head = (f'<h2>{icon(d["icon"])}<a href="{wd(d["concept"])}">{concept(d["concept"])}</a>'
            + (f'<span class="count">{count}</span>' if count else "") + "</h2>")
    wides = "".join(p.html() for p in sorted(reg.panels, key=lambda p: -p.score) if p.wide)
    flow = sheet_html(reg.rows) + "".join(p.html() for p in sorted(reg.panels, key=lambda p: -p.score) if not p.wide)
    flow = f'<div class="flow">{flow}</div>' if flow else ""
    ordered = sorted(reg.visuals, key=lambda v: -v.score)
    feature, worthy = choose_feature(ordered)
    if feature and not worthy:
        # The lead layout: the feature at the start, the rest of the region
        # in the column beside it.
        lead_w = min(feature.aspect * LEAD_HEIGHT, PAGE_WIDTH * 0.45)
        side_w = PAGE_WIDTH - lead_w - 2 * GAP
        rest = [v for v in ordered if v is not feature]
        side = visuals_html(reg, side_w, rest_only=rest) + sets_html(reg, side_w) + wides + flow
        return (f'<section class="region t-{d["tint"]}">{head}<div class="split">'
                f'<div class="lead-visual" style="inline-size:{lead_w:.0f}px">{feature.html(lead_w)}</div>'
                f'<div class="split-rest">{side}</div></div></section>')
    return (f'<section class="region t-{d["tint"]}">{head}{visuals_html(reg)}{sets_html(reg)}{wides}{flow}</section>')


# -------------------------------------------------------------- rendering

def hero_html(hero_image):
    kinds = P.by_pid.get("P31", [])
    cont = P.bundle.get("container") or {}
    chips = [f'<a class="qid" href="{wd(P.qid)}">{P.qid}</a>']
    if kinds:
        chips.append(f'<span class="hchip"><span class="k">{pl(kinds[0])}</span>'
                     + ", ".join(item_link(k) for k in kinds) + "</span>")
    if cont.get("pid") and cont["pid"] in P.by_pid:
        st = P.by_pid[cont["pid"]][0]
        chips.append(f'<span class="hchip"><span class="k">{pl(st)}</span>{item_link(st)}</span>')
    banner = (P.by_pid.get("P948") or [None])[0]
    backdrop = banner or hero_image
    back = (f'<img class="backdrop{" banner" if banner else ""}" src="{e(thumb(backdrop["value"], 1920))}" alt="">'
            if backdrop else "")
    figure = ""
    if hero_image:
        name = file_of(hero_image["value"])
        figure = (f'<figure class="hero-img">{vmenu(hero_image["plabel"])}'
                  f'<img src="{e(thumb(hero_image["value"], 500))}" alt="">'
                  f'<figcaption>{credit(name)}</figcaption></figure>')
    label_tag = f'<span class="lang-tag">{e(P.item["label_lang"])}</span>' if P.item["label_lang"] != P.lang else ""
    desc_lang = P.item["description_lang"]
    desc_tag = f'<span class="lang-tag">{e(desc_lang)}</span>' if desc_lang != P.lang else ""
    return f"""<header class="hero2">{back}
  {figure}
  <div class="hero-text">
    <h1>{e(P.item["label"])} {label_tag}</h1>
    <p class="desc" lang="{e(desc_lang)}">{e(P.item["description"])} {desc_tag}</p>
    <div class="hchips">{"".join(chips)}</div>
  </div>
</header>"""


ALLOWED = {"a", "b", "i", "span", "ruby", "rt"}


def rewrite_lead(paragraph, links, source_host):
    """Keep the prose markup, drop the rest, and send each link where the
    reader can follow it in their own language: to the article in the page
    language when it exists, otherwise to the item, whose composed page is
    the destination in production. Where the item has a label in the page
    language, it goes above the word as a ruby gloss."""
    def link(m):
        attrs, text = m.group(1), m.group(2)
        href = re.search(r'href="([^"]+)"', attrs)
        href = href.group(1) if href else ""
        if not href.startswith("/wiki/"):
            return text
        title = urllib.parse.unquote(href[6:]).replace("_", " ").split("#")[0]
        qid = links["titles"].get(title)
        target = links["items"].get(qid, {}) if qid else {}
        if target.get("local"):
            a = (f'<a class="to-local" href="https://{P.lang}.wikipedia.org/wiki/'
                 f'{e(target["local"].replace(" ", "_"))}">{text}</a>')
        elif qid:
            a = f'<a class="to-item" href="{wd(qid)}">{text}</a>'
        else:
            a = f'<a class="to-source" href="https://{source_host}{e(href)}">{text}</a>'
        gloss = target.get("label")
        if gloss and gloss.casefold() != plain(text).casefold() and source_host.split(".")[0] != P.lang:
            return f'<ruby>{a}<rt lang="{P.lang}">{e(gloss)}</rt></ruby>'
        return a
    paragraph = re.sub(r'<img[^>]*\balt="([^"]*)"[^>]*>', r"\1", paragraph)
    p = re.sub(r"<a\s([^>]*)>(.*?)</a>", link, paragraph, flags=re.S)
    p = re.sub(r"</?(\w+)[^>]*>", lambda m: m.group(0) if m.group(1).lower() in ALLOWED else "", p)
    p = re.sub(r"<(b|i)\s[^>]*>", r"<\1>", p)
    p = re.sub(r"<span\b([^>]*)>", lambda m: (lambda g: f'<span lang="{g.group(1)}">' if g else "<span>")(
        re.search(r'lang="([^"]+)"', m.group(1))), p)
    return p.strip()


def lead_html():
    leads = P.bundle["leads"]
    blocks = []
    local = leads.get(P.lang)
    english = leads.get("en") if P.lang != "en" else None
    def text_len(lead):
        return sum(len(plain(p)) for p in lead["paragraphs"])
    def block(code, lead, keep=1):
        host = f"{code}.wikipedia.org"
        paras = [rewrite_lead(p, lead["links"], host) for p in lead["paragraphs"]]
        shown = "".join(f"<p>{p}</p>" for p in paras[:keep])
        more = ""
        if len(paras) > keep:
            more = (f'<details><summary>¶ +{len(paras) - keep}</summary>'
                    + "".join(f"<p>{p}</p>" for p in paras[keep:]) + "</details>")
        source = (f'<footer><img src="{kit.WIKIPEDIA_LOGO}" alt=""><a href="https://{host}/wiki/'
                  f'{e(lead["title"].replace(" ", "_"))}">{code}.wikipedia.org</a>'
                  f'<span class="note">{e(lead["title"])}</span></footer>')
        return f'<div class="lead-block" lang="{code}">{shown}{more}{source}</div>'
    if local:
        blocks.append(block(P.lang, local))
    # A short local article, or none, is followed by the English lead.
    if english and (not local or text_len(local) < 400):
        blocks.append(block("en", english))
    legend = (f'<p class="lead-legend"><span class="to-local">{e(P.lang)}.wikipedia.org</span>'
              f'<span class="to-item">Wikidata</span></p>') if english else ""
    menu = (f'<button class="value-action" popovertarget="lead-menu" aria-label="Lead">{icon("more-vert")}</button>'
            f'<div id="lead-menu" popover class="card-menu"><button class="card-menu-item" type="button">'
            f'{icon("edit")}Edit</button><button class="card-menu-item" type="button">{icon("language")}Translate</button></div>')
    return f'<section class="lead2">{menu}{"".join(blocks)}{legend}</section>'


def evidence_panels(reg):
    shown = [s for s in P.sts if s["ptype"] != "ExternalId"]
    counts = provenance(shown)
    segs = "".join(f'<span class="seg seg-{k}" style="flex:{n}" title="{n}"></span>' for k, n in counts.items() if n)
    legend = "".join(f'<li><span class="prov prov-{k}"></span>{prov_label(k)} <b>{n}</b></li>'
                     for k, n in counts.items())
    ids = [s for s in P.sts if s["ptype"] == "ExternalId"]
    ids_button = ""
    if ids:
        rows_html = "".join(f'<div><span class="k">{pl(s)}</span><span class="v">{e(s["value"])}</span></div>'
                            for s in sorted(ids, key=lambda s: s["plabel"].lower()))
        kit.add_dialog("dlg-ids", f"{len(ids)}", f'<div class="idlist">{rows_html}</div>')
        ids_button = f'<div class="chips">{kit.more_button("dlg-ids", f"{len(ids)}", "tag")}</div>'
    reg.panels.append(Panel("provenance", [], "", f'<div class="stack-bar">{segs}</div>'
                            f'<ul class="legend">{legend}</ul>{ids_button}', 30))
    fam = {}
    for site in P.bundle["sitelinks"]:
        m = re.match(r"^(.+?)(wiki|wikiquote|wikivoyage|wikinews|wikisource|wikibooks|wikiversity|wiktionary)$", site)
        if m and site not in ("commonswiki", "specieswiki", "wikidatawiki", "metawiki", "mediawikiwiki"):
            fam.setdefault("wikipedia" if m.group(2) == "wiki" else m.group(2), []).append(m.group(1))
    chips = "".join(f'<span class="chip{" has" if P.lang in l else ""}">{e(f)} <b>{len(l)}</b>'
                    + (f'<span class="note">{e(P.lang)}</span>' if P.lang in l else "") + "</span>"
                    for f, l in sorted(fam.items(), key=lambda kv: -len(kv[1])))
    reg.panels.append(Panel("sitelinks", [], f'{concept("Q14827288")} <span class="count">{sum(len(v) for v in fam.values())}</span>',
                            f'<div class="chips">{chips}</div>', 20))
    covered = [s for s in shown if s["pid"] not in FOOTNOTE]
    props = {s["pid"]: s["plabel_lang"] for s in covered}
    values = {value_qid(s): (label_of(s), s.get("vlabel_lang")) for s in covered if value_qid(s)}
    ok = (P.lang, "mul")
    def bar(label, have, total):
        return (f'<div class="share"><span class="name">{label}</span><span class="track"><span style="inline-size:'
                f'{have / max(total, 1) * 100:.1f}%"></span></span><span class="val">{have} / {total}</span></div>')
    missing = sorted(((q, lab) for q, (lab, l) in values.items() if l not in ok), key=lambda x: int(x[0][1:]))
    kit.add_dialog("dlg-missing", f"{len(missing)}", '<div class="chips">' + "".join(
        f'<span class="chip"><a href="{wd(q)}">{e(lab)}</a><span class="note">{q}</span></span>'
        for q, lab in missing) + "</div>")
    body = (bar(concept("Q18616576"), sum(1 for l in props.values() if l in ok), len(props))
            + bar(concept("Q16222597"), sum(1 for _, l in values.values() if l in ok), len(values))
            + f'<div class="chips">{kit.more_button("dlg-missing", f"+{len(missing)}", "language")}</div>')
    reg.panels.append(Panel("coverage", [], f'{icon("language")} {e(P.lang)}', body, 25))


def compose(qid, lang):
    global P
    P = Page(qid, lang)
    hero_image, regions = build_regions()
    evidence_panels(regions.setdefault("evidence", Region("evidence")))
    compare = compare_panel()

    # At a glance: the best-scored single measurements, beside the lead.
    singles = sorted((r for r in regions.get("measure", Region("measure")).rows if r.kind == "stat"),
                     key=lambda r: -r.score)[:5]
    glance = (f'<article class="panel2 k-glance"><dl class="sheet">{"".join(r.html() for r in singles)}</dl></article>'
              if singles else "")
    side = f'<aside class="glance t-blue">{glance}{compare.html() if compare else ""}</aside>' if (glance or compare) else ""
    orient = f'<div class="orient{" with-side" if side else ""}">{lead_html()}{side}</div>'

    # The dimension with the most statements first; media and evidence close
    # the page. Small regions (a few facts and no pictures) sit side by side.
    order = sorted((d for d in regions if d not in ("media", "evidence")),
                   key=lambda d: -len(regions[d].statements()))
    order += [d for d in ("media", "evidence") if d in regions]
    parts, band = [], []
    for dim in order:
        reg = regions[dim]
        if reg.small() and dim != "evidence":
            band.append(region_html(reg))
            continue
        if band:
            parts.append(f'<div class="band">{"".join(band)}</div>')
            band = []
        parts.append(region_html(reg))
    if band:
        parts.append(f'<div class="band">{"".join(band)}</div>')

    footer = (f'<footer class="provenance">Prototype, composed by rule from the Wikidata item '
              f'<a href="{wd(qid)}">{qid}</a> (label language "{lang}, mul, en") and '
              f'<code>prototype/v2/data/{qid.lower()}-{lang}.json</code>: property classes, references, the '
              f'container and peer quantities, Commons sizes, attribution and assessments, page images '
              f'from wiki-display-image (marked W), and the Wikipedia leads. The lead paragraphs come from '
              f'Wikipedia and say so. Words that are not data are English page chrome.</footer>')
    words = dict(WORDS[lang], user="Sthottingal")
    body = "\n".join([kit.chrome(words), '<main class="page2">', hero_html(hero_image), orient,
                      *parts, footer, "</main>", kit.VALUE_MENU, "".join(kit.DIALOGS), kit.sprite()])
    viewer = VIEWER_JS if 'class="model3d"' in body else ""
    out = HERE / f"{qid.lower()}-{lang}-v2.html"
    out.write_text(f"""<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{e(P.item["label"])} ({qid}) — composed</title>
<link rel="stylesheet" href="../v1/codex-tokens-light.css">
<link rel="stylesheet" href="factoid-v2.css">
</head>
<body>
{body}
<script>{kit.FALLBACK_JS}</script>
{f"<script>{viewer}</script>" if viewer else ""}
</body>
</html>
""")
    counts = {d: (len(r.visuals), len(r.sets), len(r.rows), len(r.panels)) for d, r in regions.items()}
    print(f"wrote {out.name}: visuals/sets/rows/panels {counts}")


VIEWER_JS = (V1 / "viewer3d.js").read_text()

if __name__ == "__main__":
    compose(sys.argv[1], sys.argv[2])
