#!/usr/bin/env python3
"""Build the static Moon (Q405) prototype page, in Igbo.

The astronomical object archetype, and the case that the strategy document
names: the Igbo article about the Moon has one line. Wikidata has 103
statements about the Moon that are not external identifiers. Igbo has a label
for 57 of the 64 properties that carry them, but for only 2 of the 55 items
they point to. The page reads in Igbo where the property labels are, and in
English for almost every value. It marks each string that did not arrive in
Igbo and counts them at the end, as a finite list of work.

An astronomical object has no people in it, no boundary and no census. What
it has is an orbit that can be drawn to scale, a body that can be measured,
an appearance from Earth, a surface with a 3D model, and a short history of
visits.

Usage: python3 prototype/v1/build_q405.py
"""

import json
import math
import re
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from factoid_kit import *  # noqa: F403

QID = "Q405"
LANG = "ig"
DATA = ROOT / "prototype" / "v1" / "data"
OUT = ROOT / "prototype" / "v1" / "q405-ig.html"
FIXTURE = DATA / "q405-ig.sparql.json"

load(FIXTURE)

# Side tables, each from one API call. None of them is in the qjson query.
SITELINKS = json.loads((DATA / "q405-sitelinks.json").read_text())["entities"][QID]["sitelinks"]
IGWIKI = json.loads((DATA / "q405-igwiki.json").read_text())["query"]["pages"][0]
COMMONS = {p["title"]: p["imageinfo"][0]
           for p in json.loads((DATA / "q405-commons.json").read_text())["query"]["pages"]}
CONCEPTS = json.loads((DATA / "q405-section-labels.json").read_text())["entities"]

# Curation metadata, as the renderer's own config marks it: categories,
# templates, focus lists, described-by-source. Real data, but not what a
# reader reads, so it is not what a translator should do first.
_GROUPS = tomllib.loads((ROOT / "qrender" / "groups.toml").read_text())
FOOTNOTE_PIDS = {pid for pid, cfg in _GROUPS.get("properties", {}).items() if cfg.get("footnote")} | {
    pid for cfg in _GROUPS.get("groups", {}).values() if cfg.get("footnote") for pid in cfg.get("pids", [])}

WORDS = {
    # From the Igbo Wikipedia interface messages.
    "wordmark": "Wikipedia",
    "tagline": "njikotá édémédé nke onyobulạ",
    "search": "Chọọ Wikipedia",
    "donate": "Nye onyinye",
    "talk": "Ńkàtá",
    "user": "Sthottingal",
}

# ------------------------------------------------------------ language marks

def t(text, lang):
    """A string with its language. A string that did not come back in the
    page language carries a lang attribute, so a screen reader reads it in
    the right voice and the stylesheet can mark it. A "mul" label is a name
    that is the same in every language, such as a spacecraft or a crater: it
    is not a fallback, so it is not marked."""
    if lang and lang not in (LANG, "mul"):
        return f'<span lang="{e(lang)}">{e(text)}</span>'
    return e(text)


def lang_tag(value_lang):
    return (f'<span class="lang-tag">{e(value_lang)}</span>'
            if value_lang and value_lang != LANG else "")


def pl(st):
    """The property label of a statement or a qualifier."""
    return t(st["plabel"], st["plabel_lang"])


def vl(st):
    """The value label of an item statement."""
    return t(label_of(st), st["vlabel_lang"])


def unit(st):
    """No unit for a dimensionless quantity. Q199, the number 1, is how
    Wikidata writes "no unit", and its Igbo label is "otu"."""
    if not st["unit"] or st["unit"] in ("1", "otu"):
        return ""
    return t(st["unit"], st["unit_lang"])


def amount(st):
    u = unit(st)
    return f'{num(st["value"])}' + (f" {u}" if u else "")


def qv(st, pid):
    """A qualifier value with its language, or None. A qualifier whose value
    is "some value" arrives as a blank node URI; that is not a thing to show."""
    for q in st["quals"]:
        if q["pid"] == pid and not q["value"].startswith("http://www.wikidata.org/.well-known/genid"):
            return t(q["value"], q["label_lang"])
    return None


def ql(st, pid):
    """The label of a qualifier property."""
    for q in st["quals"]:
        if q["pid"] == pid:
            return pl(q)
    return ""


def item_link(st):
    qid = value_qid(st)
    return f'<a href="{wd(qid)}">{vl(st)}</a>' if qid else vl(st)


def item_chip(st, note=None):
    """A chip whose label keeps its language. The kit chip escapes its label,
    so it cannot carry a lang attribute."""
    img = f'<img src="{e(commons(st["vimg"], 48))}" alt="" loading="lazy">' if st["vimg"] else ""
    note_html = f'<span class="note">{note}</span>' if note else ""
    return f'<span class="chip">{img}<span>{item_link(st)}</span>{note_html}{vmenu(label_of(st))}</span>'


def section_title(qid):
    """Section names are Wikidata items, resolved through the same label
    chain as the data. Where the item has no Igbo label, the English one is
    shown and marked, like any other fallback."""
    labels = CONCEPTS[qid]["labels"]
    if "ig" in labels:
        return labels["ig"]["value"], None
    return labels["en"]["value"], "en"


def panel(span, icon_name, qid, count, body, tint=None):
    """The kit section, with a title from a Wikidata item."""
    title, lang = section_title(qid)
    html = section(span, icon_name, title, count, body, tint)
    if lang:
        html = html.replace(f"<h2>{e(title)}</h2>",
                            f'<h2 lang="{lang}"><a href="{wd(qid)}">{e(title)}</a></h2>', 1)
    else:
        html = html.replace(f"<h2>{e(title)}</h2>",
                            f'<h2><a href="{wd(qid)}">{e(title)}</a></h2>', 1)
    return html


def facts_rows(rows):
    """Fact rows whose keys are already marked HTML."""
    body = "".join(
        f'<div><span class="k">{k}</span><span class="v">{v}</span>'
        f'{vmenu(re.sub(r"<[^>]+>", "", k))}</div>'
        for k, v in rows if v
    )
    return f'<div class="facts">{body}</div>'


def stat_tile(icon_name, st, value, note=None):
    """The kit stat tile, with a label that keeps its language."""
    u = unit(st)
    unit_html = f'<span class="stat-unit"> {u}</span>' if u else ""
    note_html = f'<span class="stat-note">{note}</span>' if note else ""
    return (f'<div class="stat {next_tint()}">{vmenu(st["plabel"])}'
            f'<span class="stat-label">{icon(icon_name)}{pl(st)}</span>'
            f'<span class="stat-value">{e(value)}{unit_html}</span>{note_html}</div>')


def signed_year(st):
    """A year as a signed number. The kit date formatter has no words for
    geological precision (4.5 billion years at a precision of a million
    years), and a signed number needs no era word in any language."""
    y = year_of(st["value"])
    return ("\u2212" if y < 0 else "") + num(abs(y))


def media_url(st):
    return commons(st["value"])


def file_name(st):
    from urllib.parse import unquote
    return unquote(local_name(st["value"]))


def megabytes(size):
    return f"{size / 1_000_000:.1f} MB"


# ---------------------------------------------------------------- sections

def section_hero():
    banner = one("P948")
    photo = one("P18")
    kinds = P("P31")
    glyphs = P("P487")

    legends = [q for q in photo["quals"] if q["pid"] == "P2096"]
    legend = next((q for q in legends if q["lang"] == LANG), None) or \
        next((q for q in legends if q["lang"] == "en"), None)
    caption = pl(photo) + (f" · {t(legend['value'], legend['lang'])}" if legend else "")

    meta = [f'<a class="qid-badge" href="{wd(QID)}">{QID}</a>']
    meta.append(f'<span class="hero-chip"><span class="lang">{pl(kinds[0])}</span>'
                + "<span>" + ", ".join(vl(k) for k in kinds) + "</span></span>")
    meta.append(f'<span class="hero-chip"><span class="lang">{pl(glyphs[0])}</span>'
                f'<span class="glyphs">{" ".join(e(g["value"]) for g in glyphs)}</span></span>')
    return f"""<section class="hero moon span-12">
  <img class="hero-banner" src="{e(commons(banner["value"], 2000))}" alt="">
  <div class="hero-body">
    <figure class="hero-disc">
      {vmenu(photo["plabel"])}
      <img src="{e(commons(photo["value"], 600))}" alt="{e(item()["label"])}">
      <figcaption>{caption}</figcaption>
    </figure>
    <div>
      <h1>{e(item()["label"])} {lang_tag(item()["label_lang"])}</h1>
      <p class="hero-desc" lang="{e(item()["description_lang"])}">{e(item()["description"])} {lang_tag(item()["description_lang"])}</p>
      <div class="hero-meta">{"".join(meta)}</div>
    </div>
  </div>
  <p class="banner-credit">{pl(banner)} · {e(file_name(banner))}</p>
</section>"""


def igbo_article_html(wikitext):
    """The Igbo article, rendered from its wikitext. It is short enough that
    a few rules cover all of it: the infobox template and the lead picture
    are not prose, so they go; a symbol picture keeps its alt text."""
    text = re.sub(r"\{\{[^}]*\}\}", "", wikitext)
    text = re.sub(r"\[\[file:[^\]|]*\|thumb\|[^\]]*\]\]", "", text, flags=re.I)
    text = re.sub(r"\[\[file:[^\]]*\|([^\]|]*)\]\]", r"\1", text, flags=re.I)
    text = e(text.strip())
    text = re.sub(r"(?:&#x27;){3}(.+?)(?:&#x27;){3}", r"<b>\1</b>", text)
    return re.sub(
        r"\[\[([^\]|]+)\|([^\]]+)\]\]",
        lambda m: f'<a href="https://ig.wikipedia.org/wiki/{e(m.group(1).replace(" ", "_"))}">{m.group(2)}</a>',
        text)


def section_lead():
    """Two blocks of prose, and neither is Wikidata. The Igbo article comes
    first because it exists and a person wrote it. It is one line, so the
    English lead follows it, marked as English, with Translate on it."""
    wikitext = IGWIKI["revisions"][0]["slots"]["main"]["content"]
    title = IGWIKI["title"]
    return f"""<section class="lead span-12">
  <button class="value-action" popovertarget="lead-menu"
          aria-label="Actions on the lead section">{icon("more-vert")}</button>
  <div id="lead-menu" popover class="card-menu">
    <button class="card-menu-item" type="button">{icon("edit")}Edit</button>
    <button class="card-menu-item" type="button">{icon("language")}Translate</button>
  </div>
  <div class="lead-local">
    <p>{igbo_article_html(wikitext)}</p>
    <footer>
      <img src="{WIKIPEDIA_LOGO}" alt="">
      <a href="https://ig.wikipedia.org/wiki/{e(title)}">Wikipedia Igbo</a>
      <span class="note">{e(title)} · the whole article, {IGWIKI["length"]} bytes</span>
    </footer>
  </div>
  <p class="lead-missing">The Igbo article is one line and an infobox. The
     paragraph below is the lead of the English article.</p>
  <p lang="en">{e(LEAD["text"])}</p>
  <footer>
    <img src="{WIKIPEDIA_LOGO}" alt="">
    <a href="{e(LEAD["url"])}">{e(LEAD["project"])}</a>
    <span class="note">{e(LEAD["title"])} · lead section, not from Wikidata</span>
  </footer>
</section>"""


def section_keyfacts():
    """The six numbers a reader asks about first: how far, how big, how
    heavy, how strong its pull, and how long its month is, in the two
    senses of month."""
    distance = one("P2583")
    diameter = one("P2386")
    mass = one("P2067")
    gravity = one("P7015")
    sidereal = one("P2146")
    synodic = one("P4341")
    tiles = [
        stat_tile("distance", distance, num(distance["value"]),
                  note=f'{ql(distance, "P4934")}: {qv(distance, "P4934")}' if qv(distance, "P4934") else None),
        stat_tile("straighten", diameter, num(diameter["value"]),
                  note=qv(diameter, "P518")),
        stat_tile("weight", mass, num(mass["value"])),
        stat_tile("arrow_cool_down", gravity, num(gravity["value"])),
        stat_tile("globe", sidereal, num(sidereal["value"])),
        stat_tile("calendar_month", synodic, num(synodic["value"])),
    ]
    return '<section class="keyfacts span-12">' + "".join(tiles) + "</section>"


# ------------------------------------------------------------------ orbit

def orbit_diagram():
    """The orbit drawn to scale from four statements: periapsis and apoapsis
    give the ellipse, and the parent body sits at one focus. At this scale the
    ellipse is almost a circle and the Earth is visibly off centre; both are
    true, and both are the point."""
    peri = float(one("P2244")["value"])
    apo = float(one("P2243")["value"])
    parent = one("P397")
    a = (peri + apo) / 2
    c = (apo - peri) / 2
    b = math.sqrt(a * a - c * c)
    w, h, pad = 520, 440, 64
    scale = (w - 2 * pad) / (2 * a)
    cx, cy = w / 2, h / 2
    rx, ry = a * scale, b * scale
    fx = cx - c * scale           # the occupied focus
    px, ax = cx - rx, cx + rx     # periapsis on the focus side
    return f"""<svg class="chart orbit" viewBox="0 0 {w} {h}" role="img"
     aria-label="{e(one("P2244")["plabel"])} {num(peri)}, {e(one("P2243")["plabel"])} {num(apo)}">
  <ellipse cx="{cx:.1f}" cy="{cy:.1f}" rx="{rx:.1f}" ry="{ry:.1f}" class="orbit-path"/>
  <line x1="{px:.1f}" y1="{cy:.1f}" x2="{ax:.1f}" y2="{cy:.1f}" class="grid-line"/>
  <circle cx="{cx:.1f}" cy="{cy:.1f}" r="2.5" class="orbit-centre"/>
  <circle cx="{fx:.1f}" cy="{cy:.1f}" r="9" class="orbit-parent"><title>{e(label_of(parent))}</title></circle>
  <text x="{fx:.1f}" y="{cy + 28:.1f}" text-anchor="middle" class="point-label">{e(label_of(parent))}</text>
  <circle cx="{px:.1f}" cy="{cy:.1f}" r="6" class="orbit-moon"/>
  <circle cx="{ax:.1f}" cy="{cy:.1f}" r="6" class="orbit-moon"/>
  <text x="{px + 12:.1f}" y="{cy - 12:.1f}" class="axis-text">{e(one("P2244")["plabel"])}</text>
  <text x="{px + 12:.1f}" y="{cy + 20:.1f}" class="point-label">{num(peri)}</text>
  <text x="{ax - 12:.1f}" y="{cy - 12:.1f}" text-anchor="end" class="axis-text">{e(one("P2243")["plabel"])}</text>
  <text x="{ax - 12:.1f}" y="{cy + 20:.1f}" text-anchor="end" class="point-label">{num(apo)}</text>
  <text x="{cx:.1f}" y="{h - 12}" text-anchor="middle" class="axis-text">{e(one("P2233")["plabel"])} {num(a)} {e(one("P2233")["unit"])}</text>
</svg>"""


def inclination_diagram():
    """The tilt of the orbit against the reference plane, at its true angle."""
    inc = one("P2045")
    ref = qv(inc, "P2210") or ""
    deg = float(inc["value"])
    w, h, x0 = 300, 90, 30
    y0 = 60
    length = w - 2 * x0
    x1 = x0 + length * math.cos(math.radians(deg))
    y1 = y0 - length * math.sin(math.radians(deg))
    return f"""<svg class="chart" viewBox="0 0 {w} {h}" role="img" aria-label="{e(inc["plabel"])} {num(deg)}">
  <line x1="{x0}" y1="{y0}" x2="{x0 + length}" y2="{y0}" class="grid-line"/>
  <line x1="{x0}" y1="{y0}" x2="{x1:.1f}" y2="{y1:.1f}" class="series-line"/>
  <text x="{x0 + length}" y="{y0 + 18}" text-anchor="end" class="axis-text">{re.sub(r"<[^>]+>", "", ref)}</text>
  <text x="{x0 + 70}" y="{y0 - 14}" class="point-label">{num(deg)}°</text>
</svg>"""


def section_orbit():
    semi = one("P2233")
    peri, apo = one("P2244"), one("P2243")
    ecc = P("P1096")
    inc = one("P2045")
    kind = one("P522")
    effect = one("P1542")
    distances = P("P2583")
    sidereal, synodic = one("P2146"), one("P4341")

    ecc_value = one("P1096")
    ecc_other = [s for s in ecc if s is not ecc_value]
    ecc_note = " · ".join(
        f'{num(s["value"])} <span class="note">{ql(s, "P585")}: {year_str(qual(s, "P585") or "")}</span>'
        for s in ecc_other)

    def distance_row(st):
        when = qual(st, "P585")
        bits = [x for x in [qv(st, "P4934"),
                            fmt_date(when) if when else None] if x]
        return (f'{amount(st)} <span class="note">{" · ".join(bits)}'
                + (" · preferred" if st["rank"] == "Preferred" else "") + "</span>")

    top = max(float(sidereal["value"]), float(synodic["value"]))
    bars = "".join(
        f'<div class="amount-row"><span class="amount-label">{pl(st)}</span>'
        f'<span class="amount-track"><span class="amount-fill" '
        f'style="inline-size: {float(st["value"]) / top * 100:.1f}%"></span></span>'
        f'<span class="amount-value">{num(st["value"])} {unit(st)}</span>{vmenu(st["plabel"])}</div>'
        for st in (sidereal, synodic)
    )

    # Up the containment ladder: where the Moon is, what it is part of, what
    # it goes round. Three relations, one direction: outwards.
    ladder = "".join(
        f'<li><span class="note">{pl(st)}</span>{item_chip(st)}</li>'
        for st in (one("P276"), one("P361"), one("P397"))
    )
    body = f"""
<div class="cols-map">
  <div class="tile">
    <h3>{pl(peri)} · {pl(apo)} <span class="note">{unit(peri)}</span>{vmenu(peri["plabel"])}</h3>
    {orbit_diagram()}
    <p class="order-note">Distances to scale. The two bodies are markers, not to scale.</p>
  </div>
  <div class="stack">
    <div class="tile">
      {facts_rows([
          (pl(semi), amount(semi)),
          (pl(peri), amount(peri)),
          (pl(apo), amount(apo)),
          (pl(ecc_value), f'{num(ecc_value["value"])}' + (f"<br>{ecc_note}" if ecc_note else "")),
          (pl(kind), f'{item_link(kind)} <span class="note">{qv(kind, "P805") or ""}</span>'),
          (pl(effect), item_link(effect)),
      ])}
    </div>
    <div class="tile">
      <h3>{pl(inc)}{vmenu(inc["plabel"])}</h3>
      {inclination_diagram()}
    </div>
  </div>
</div>
<div class="cols">
  <div class="tile">
    <h3>{pl(distances[0])} ({len(distances)}){vmenu(distances[0]["plabel"])}</h3>
    {facts_rows([(qv(d, "P1013") or pl(d), distance_row(d)) for d in
                 sorted(distances, key=lambda d: float(d["value"]))])}
  </div>
  <div class="tile">
    <h3>{pl(sidereal)} · {pl(synodic)}{vmenu(sidereal["plabel"])}</h3>
    <div class="amounts">{bars}</div>
  </div>
  <div class="tile">
    <h3>{pl(one("P276"))} · {pl(one("P361"))} · {pl(one("P397"))}</h3>
    <ol class="ladder">{ladder}</ol>
  </div>
</div>
"""
    return panel(12, "globe", "Q4130", f'{num(semi["value"])} {semi["unit"]}', body)


# ------------------------------------------------------------------- body

def section_body():
    radii = P("P2120")
    diameter = one("P2386")
    flat = one("P1102")
    area = one("P2046")
    volume = one("P2234")
    mass = one("P2067")
    density = one("P2054")
    gravity = one("P7015")
    temps = P("P2076")

    # A temperature range on a scale that starts at absolute zero, so the
    # bar is as long as the number says.
    lo = min(temps, key=lambda s: float(s["value"]))
    hi = max(temps, key=lambda s: float(s["value"]))
    top = math.ceil(float(hi["value"]) / 100) * 100
    left = float(lo["value"]) / top * 100
    width = (float(hi["value"]) - float(lo["value"])) / top * 100
    temp_html = f"""
<div class="range">
  <div class="range-track"><span class="range-fill" style="inset-inline-start: {left:.1f}%; inline-size: {width:.1f}%"></span></div>
  <div class="range-ends">
    <span><b>{num(lo["value"])}</b> {unit(lo)} <span class="note">{qv(lo, "P1480") or ""}</span></span>
    <span><b>{num(hi["value"])}</b> {unit(hi)} <span class="note">{qv(hi, "P1480") or ""}</span></span>
  </div>
  <div class="band-axis"><span>0</span><span>{top} {unit(hi)}</span></div>
</div>"""

    radius_rows = "<br>".join(
        f'{amount(r)} <span class="note">{qv(r, "P518") or ""}</span>'
        for r in sorted(radii, key=lambda r: float(r["value"])))
    body = f"""
<div class="tile">
  {facts_rows([
      (pl(radii[0]), radius_rows),
      (pl(diameter), f'{amount(diameter)} <span class="note">{qv(diameter, "P518") or ""}</span>'),
      (pl(flat), amount(flat)),
      (pl(area), f'{amount(area)} <span class="note">{qv(area, "P459") or ""}</span>'),
      (pl(volume), amount(volume)),
      (pl(mass), amount(mass)),
      (pl(density), f'{amount(density)} <span class="note">{ql(density, "P515")}: {qv(density, "P515") or ""}</span>'),
      (pl(gravity), amount(gravity)),
  ])}
</div>
<div class="tile">
  <h3>{pl(temps[0])}{vmenu(temps[0]["plabel"])}</h3>
  {temp_html}
</div>
"""
    return panel(6, "straighten", "Q4373292", None, body)


# ------------------------------------------------------------- from Earth

def section_sky():
    angular = P("P5348")
    photo = one("P18")
    magnitude = one("P1215")
    albedo = one("P4501")
    symbol = one("P367")
    night = one("P3451")
    synodic = one("P4341")

    # The same photograph at the two sizes the Moon has in the sky, to
    # scale with each other.
    big = max(float(s["value"]) for s in angular)
    discs = "".join(
        f'<figure class="disc">{vmenu(s["plabel"])}'
        f'<img src="{e(commons(photo["value"], 300))}" alt="" '
        f'style="inline-size: {float(s["value"]) / big * 150:.1f}px">'
        f'<figcaption><b>{num(s["value"])}′</b> <span class="note">{qv(s, "P1480") or ""}</span></figcaption></figure>'
        for s in sorted(angular, key=lambda s: float(s["value"]))
    )
    body = f"""
<div class="tile">
  <h3>{pl(angular[0])} <span class="note">{unit(angular[0])}</span>{vmenu(angular[0]["plabel"])}</h3>
  <div class="discs">{discs}</div>
</div>
<div class="tile">
  {facts_rows([
      (pl(magnitude), amount(magnitude)),
      (pl(albedo), f'{amount(albedo)} <span class="note">{ql(albedo, "P1013")}: {qv(albedo, "P1013") or ""}</span>'),
      (pl(synodic), amount(synodic)),
  ])}
</div>
<div class="gallery two">
  <figure><a href="{e(media_url(night))}"><img src="{e(commons(night["value"], 640))}" alt="" loading="lazy"></a>
    {vmenu(night["plabel"])}<figcaption><b>{pl(night)}</b></figcaption></figure>
  <figure class="symbol"><img src="{e(commons(symbol["value"], 200))}" alt="" loading="lazy">
    {vmenu(symbol["plabel"])}<figcaption><b>{pl(symbol)}</b></figcaption></figure>
</div>
"""
    return panel(6, "visibility", "Q1153471", None, body)


# ---------------------------------------------------------------- surface

# Shared with compose_v2.py.
VIEWER_JS = (Path(__file__).resolve().parent / "viewer3d.js").read_text()


def section_surface():
    model = one("P4896")
    texture = one("P4640")
    point = one("P625")
    located = one("P376")
    highest, lowest = one("P610"), one("P1589")
    parts = P("P527")
    traits = P("P1552")

    info = COMMONS["File:" + file_name(model)]
    meta = info["extmetadata"]
    author = re.sub(r"<[^>]+>", "", meta["Artist"]["value"])
    description = re.sub(r"<[^>]+>", "", meta["ImageDescription"]["value"]).split(" using ")[0].split(" by ")[0]
    src = info["url"].split("?")[0]

    # The coordinate is on the Moon's own globe. An OpenStreetMap tile would
    # be a map of the Earth, so the map here is the equirectangular texture
    # the item itself names, and the point goes on it by longitude and
    # latitude.
    globe, lon, lat = re.search(r"entity/(Q\d+)> Point\(([-\d.]+) ([-\d.]+)\)", point["value"]).groups()
    x = (float(lon) + 180) / 360 * 100
    y = (90 - float(lat)) / 180 * 100

    body = f"""
<div class="cols-map">
  <figure class="model3d" data-src="{e(src)}">
    {vmenu(model["plabel"])}
    <div class="model3d-stage">
      <img src="{e(commons(model["value"], 1000))}" alt="{e(file_name(model))}" loading="lazy">
    </div>
    <button class="model3d-load more" type="button" hidden>{icon("3d_rotation")}View in 3D · {megabytes(info["size"])}</button>
    <figcaption><b>{pl(model)}</b> · {e(file_name(model))}<br>
      <span lang="en">{e(description)}</span> · {e(author)} · {e(meta["LicenseShortName"]["value"])}
      <span class="note">· description from the Commons file page</span></figcaption>
  </figure>
  <div class="stack">
    <figure class="moon-map">
      {vmenu(point["plabel"])}
      <div class="moon-map-frame">
        <img src="{e(commons(texture["value"], 1200))}" alt="{e(texture["plabel"])}" loading="lazy">
        <span class="pin" style="left: {x:.2f}%; top: {y:.2f}%"
              title="{e(point["plabel"])}: {lat}, {lon}"></span>
      </div>
      <figcaption>{pl(texture)} · {pl(point)} {lat}, {lon} · {pl(located)}: {e(item()["label"] if globe == QID else globe)}</figcaption>
    </figure>
    <div class="tile">
      {facts_rows([
          (pl(highest), item_link(highest)),
          (pl(lowest), item_link(lowest)),
      ])}
    </div>
  </div>
</div>
<div class="cols">
  <div class="tile">
    <h3>{pl(parts[0])} ({len(parts)}){vmenu(parts[0]["plabel"])}</h3>
    <div class="chips">{"".join(item_chip(p) for p in parts)}</div>
  </div>
  <div class="tile">
    <h3>{pl(traits[0])} ({len(traits)}){vmenu(traits[0]["plabel"])}</h3>
    <div class="chips">{"".join(item_chip(p) for p in traits)}</div>
  </div>
</div>
"""
    return panel(12, "3d_rotation", "Q7201252", None, body)


# ------------------------------------------------------------ exploration

def section_exploration():
    craft = P("P398")
    visitors = one("P3872")
    studied = P("P2579")
    use = one("P366")
    history = one("P2184")
    inception = one("P571")

    # Four spacecraft. The two with a start date come first, newest first;
    # the item number breaks ties, because it is the same in every language.
    def order(st):
        start = year_of(qual(st, "P580") or "")
        qid = value_qid(st) or "Q0"
        return (start is None, -(start or 0), int(qid[1:]))

    tiles = "".join(
        thumb(label_of(s), s["vimg"], note=timespan(s) or None,
              link=wd(value_qid(s)) if value_qid(s) else None)
        for s in sorted(craft, key=order))
    body = f"""
<div>
  <h3>{pl(craft[0])} ({len(craft)}){vmenu(craft[0]["plabel"])}</h3>
  <div class="thumbs wide-tiles">{tiles}</div>
</div>
<div class="cols">
    <div class="tile">
      <h3>{pl(visitors)}{vmenu(visitors["plabel"])}</h3>
      <div class="bignum">{num(visitors["value"])}</div>
      <div class="note">{ql(visitors, "P2348")}: {qv(visitors, "P2348") or ""}</div>
    </div>
    <div class="tile">
      {facts_rows([
          (pl(studied[0]), ", ".join(item_link(s) for s in studied)),
          (pl(use), f'{item_link(use)} <span class="note">{qv(use, "P5102") or ""}</span>'),
          (pl(history), item_link(history)),
          (pl(inception), f'{signed_year(inception)} <span class="note">{qv(inception, "P1480") or ""}</span>'),
      ])}
    </div>
</div>
"""
    return panel(12, "event", "Q1064739", f"{len(craft)}", body)


# ------------------------------------------------------------ name, sound

def section_names():
    named = one("P138")
    glyphs = P("P487")
    audio = P("P443") + P("P989")
    depicted = one("P1299")

    # Pronunciation recordings exist in three languages and a spoken text
    # in a fourth. None is Igbo. Say so, and ask for one.
    tiles = "".join(
        f'<div class="audio-row"><span>{qv(a, "P407") or ""}'
        + (f' <span class="note">{pl(a)}</span>' if a["pid"] != audio[0]["pid"] else "")
        + '</span>'
        f'<audio controls preload="none" src="{e(media_url(a))}"></audio>{vmenu(a["plabel"])}</div>'
        for a in audio)
    audio_langs = [qual(a, "P407") for a in audio]
    has_igbo = any(l and l.lower() in ("igbo", "asụ̀sụ̀ ìgbò") for l in audio_langs)
    invite = "" if has_igbo else (
        f'<p class="order-note">None of the {len(audio)} recordings is in Igbo.</p>'
        f'<button class="invite-button" type="button">{icon("volume_up")}'
        f'Record “{e(item()["label"])}” in Igbo</button>')
    body = f"""
<div class="tile">
  {facts_rows([
      (pl(named), f'{item_link(named)} <span class="note">{ql(named, "P407")}: {qv(named, "P407") or ""}</span>'),
      (pl(glyphs[0]), f'<span class="glyphs big">{" ".join(e(g["value"]) for g in glyphs)}</span>'),
      (pl(depicted), item_link(depicted)),
  ])}
</div>
<div class="tile">
  <h3>{pl(audio[0])}{vmenu(audio[0]["plabel"])}</h3>
  <div class="audios">{tiles}</div>
</div>
{invite}
"""
    return panel(6, "volume_up", "Q82799", None, body)


def section_pictures():
    stills = P("P18") + [one("P948"), one("P4640")]
    video = one("P10")
    info = COMMONS["File:" + file_name(video)]
    figures = "".join(
        f'<figure><a href="{e(media_url(st))}">'
        f'<img src="{e(commons(st["value"], 640))}" alt="" loading="lazy"></a>'
        f'{vmenu(st["plabel"])}'
        f'<figcaption><b>{pl(st)}</b><span>{e(file_name(st))}</span></figcaption></figure>'
        for st in stills)
    body = f"""
<figure class="video">
  {vmenu(video["plabel"])}
  <video controls preload="none" poster="{e(commons(video["value"], 800))}" src="{e(media_url(video))}"></video>
  <figcaption><b>{pl(video)}</b> · {e(file_name(video))} · {megabytes(info["size"])}</figcaption>
</figure>
<div class="gallery">{figures}</div>
"""
    return panel(6, "photo_library", "Q478798", f"{len(stills) + 1}", body)


# ----------------------------------------------------------------- the rest

def section_more():
    """Statements that belong to no section above. Every page has some, and
    a page that forces them into a section it names says something false."""
    diocese = one("P708")
    different = one("P1889")
    duplicate = one("P2959")
    body = f"""
<div class="tile">
  {facts_rows([
      (pl(diocese), item_chip(diocese)),
      (pl(different), item_link(different)),
      (pl(duplicate), item_link(duplicate)),
  ])}
</div>
"""
    return section(12, "info", "More", None, body, tint="grey")


def coverage():
    """How much of this page arrived in Igbo, counted over the strings a
    reader reads: property labels, item values and units. External
    identifiers and curation metadata are left out."""
    shown = [s for s in statements()
             if s["ptype"] != "ExternalId" and s["rank"] != "Deprecated"
             and s["pid"] not in FOOTNOTE_PIDS]
    props = {s["pid"]: s["plabel_lang"] for s in shown}
    values = {value_qid(s): (label_of(s), s["vlabel_lang"]) for s in shown if value_qid(s)}
    units = {s["unit"]: s["unit_lang"] for s in shown if s["unit"] and unit(s)}
    return props, values, units


def section_igbo():
    props, values, units = coverage()

    def bar(have, total):
        return (f'<div class="amount-row coverage"><span class="amount-track"><span class="amount-fill" '
                f'style="inline-size: {have / total * 100:.1f}%"></span></span>'
                f'<span class="amount-value">{have} / {total}</span></div>')

    covered = (LANG, "mul")
    p_have = sum(1 for l in props.values() if l in covered)
    v_have = sum(1 for _, l in values.values() if l in covered)
    u_have = sum(1 for l in units.values() if l in covered)
    v_mul = sum(1 for _, l in values.values() if l == "mul")
    missing = sorted(((q, lbl) for q, (lbl, l) in values.items() if l not in covered),
                     key=lambda x: int(x[0][1:]))
    chips = "".join(f'<span class="chip"><a href="{wd(q)}" lang="en">{e(lbl)}</a>'
                    f'<span class="note">{q}</span></span>' for q, lbl in missing)
    have_values = ", ".join(e(lbl) for q, (lbl, l) in values.items() if l == LANG)
    body = f"""
<div class="cols">
  <div class="tile">
    <h3>property labels</h3><div class="amounts">{bar(p_have, len(props))}</div>
    <h3 style="margin-block-start: var( --spacing-100 );">values</h3><div class="amounts">{bar(v_have, len(values))}</div>
    <h3 style="margin-block-start: var( --spacing-100 );">units</h3><div class="amounts">{bar(u_have, len(units))}</div>
    <p class="order-note" style="margin-block-start: var( --spacing-100 );">Values in Igbo: {have_values}.
       {v_mul} more are names that are the same in every language (mul).
       Strings with a dotted underline on this page are not yet in Igbo.
       Counted without external identifiers and Wikimedia curation
       statements.</p>
  </div>
  <div class="tile">
    <h3>values without an Igbo label ({len(missing)})</h3>
    <div class="chips">{chips}</div>
  </div>
</div>
<button class="invite-button" type="button">{icon("language")}Add Igbo labels</button>
"""
    return panel(12, "language", "Q33578", f"{p_have + v_have + u_have} / {len(props) + len(values) + len(units)}", body)


def section_wikimedia():
    commons_cat, commons_gal = one("P373"), one("P935")
    category, portal, outline = one("P910"), one("P1151"), one("P5125")
    templates = P("P1424")
    project = one("P6104")
    focus = P("P5008")
    sources = P("P1343")
    ids = [s for s in statements() if s["ptype"] == "ExternalId" and s["rank"] != "Deprecated"]

    id_rows = "".join(
        f'<div><span class="k">{pl(s)}</span><span class="v">{e(s["value"])}</span></div>'
        for s in sorted(ids, key=lambda s: s["plabel"].lower()))
    add_dialog("dlg-ids", f"External identifiers ({len(ids)})", f'<div class="idlist">{id_rows}</div>')
    def source_note(st):
        note = qv(st, "P805") or qv(st, "P1810") or ""
        bare = re.fullmatch(r"(?:<span[^>]*>)?(Q\d+)(?:</span>)?", note)
        return f'<a href="{wd(bare.group(1))}">{bare.group(1)}</a>' if bare else note

    source_rows = "".join(
        f'<tr><td>{vl(s)}</td><td>{source_note(s)}</td></tr>'
        for s in sorted(sources, key=lambda s: int((value_qid(s) or "Q0")[1:])))
    add_dialog("dlg-sources", f'{sources[0]["plabel"]} ({len(sources)})',
               f'<table class="vtable"><tbody>{source_rows}</tbody></table>')

    # Sister projects, from the sitelinks. Wikipedia has an Igbo article;
    # no other project has an Igbo page about the Moon.
    families = {}
    for site in SITELINKS:
        m = re.match(r"^(.+?)(wiki|wikiquote|wikivoyage|wikinews|wikisource|wikibooks|wikiversity|wiktionary)$", site)
        if not m or site in ("commonswiki", "specieswiki", "wikidatawiki", "metawiki", "mediawikiwiki"):
            continue
        family = "wikipedia" if m.group(2) == "wiki" else m.group(2)
        families.setdefault(family, []).append(m.group(1))
    family_chips = "".join(
        f'<span class="chip">{e(f)} <b>{len(langs)}</b>'
        + (f'<span class="note">ig</span>' if "ig" in langs else "")
        + "</span>"
        for f, langs in sorted(families.items(), key=lambda kv: -len(kv[1])))
    nearby = "".join(
        f'<span class="chip"><a href="https://{code}.wikipedia.org/wiki/{e(SITELINKS[code + "wiki"]["title"])}" '
        f'lang="{code}" class="native">{e(SITELINKS[code + "wiki"]["title"])}</a><span class="note">{code}</span></span>'
        for code in ("ig", "yo", "ha", "en") if code + "wiki" in SITELINKS)

    body = f"""
<div class="cols">
  <div class="tile">
    <h3>{pl(commons_cat)} · {pl(commons_gal)}</h3>
    <div class="chips">
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/Category:{e(commons_cat["value"])}">{e(commons_cat["value"])}</a></span>
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/{e(commons_gal["value"])}">{e(commons_gal["value"])}</a></span>
    </div>
    <h3 style="margin-block-start: var( --spacing-100 );">Wikipedia · ig, yo, ha, en</h3>
    <div class="chips">{nearby}</div>
  </div>
  <div class="tile">
    <h3>{sum(len(v) for v in families.values())} sitelinks</h3>
    <div class="chips">{family_chips}</div>
  </div>
  <div class="tile">
    {facts_rows([
        (pl(category), vl(category)),
        (pl(portal), vl(portal)),
        (pl(outline), vl(outline)),
        (pl(templates[0]), ", ".join(vl(x) for x in templates)),
        (pl(project), vl(project)),
        (pl(focus[0]), "<br>".join(vl(x) for x in focus)),
    ])}
  </div>
</div>
<div class="chips">
  {more_button("dlg-sources", f"{sources[0]['plabel']} ({len(sources)})", "local_library")}
  {more_button("dlg-ids", f"External identifiers ({len(ids)})", "tag")}
</div>
"""
    return panel(12, "sell", "Q14827288", None, body, tint="grey")


LEAD = {
    "text": "The Moon is the only natural satellite of Earth. It orbits Earth "
            "at an average distance of 384,399 kilometers (238,854 mi). "
            "Because the Moon follows an elliptical path, its distance from "
            "Earth varies over the year, ranging from 357,000–407,000 "
            "kilometers (222,000–253,000 mi), a distance roughly 30 times the "
            "width of Earth. It completes an orbit in relation to Earth and "
            "the Sun (synodically) every 29.5 days.",
    "title": "Moon",
    "url": "https://en.wikipedia.org/wiki/Moon",
    "project": "English Wikipedia",
}

FOOTER = (
    'Prototype. Every label, number, date, image and link on this page comes '
    f'from the Wikidata item <a href="{wd(QID)}">{QID}</a>, as the qjson SPARQL '
    f'query returns it with the label language "{LANG}, mul, en" '
    '(<code>prototype/v1/data/q405-ig.sparql.json</code>). Four side tables, each '
    'from one API call, add what that query does not fetch: the section names, '
    'which are Wikidata items resolved in Igbo where they have a label '
    '(<code>q405-section-labels.json</code>); the sitelinks '
    '(<code>q405-sitelinks.json</code>); the text of the Igbo article '
    '(<code>q405-igwiki.json</code>); and the size, author, licence and '
    'description of the 3D model and the video from Commons '
    '(<code>q405-commons.json</code>). The English lead paragraph is from the '
    '<a href="https://en.wikipedia.org/wiki/Moon">English Wikipedia</a> and '
    'says so on the page. Pictures, sound, video and the 3D model come from '
    'Wikimedia Commons. The words "View in 3D", "Record", "Add Igbo labels", '
    '"More", the coverage headings and the notes on scale are page chrome, '
    'not data, and they are in English. The Wikipedia header is a mock-up, '
    'with words from the Igbo Wikipedia interface.'
)


def build():
    sections = [
        section_hero(),
        section_lead(),
        section_keyfacts(),
        section_orbit(),
        section_body(),
        section_sky(),
        section_surface(),
        section_exploration(),
        section_names(),
        section_pictures(),
        section_igbo(),
        section_more(),
        section_wikimedia(),
    ]
    render_page(OUT, QID, LANG, sections, WORDS, FOOTER)
    page = OUT.read_text().replace("</body>", f"<script>{VIEWER_JS}</script>\n</body>")
    OUT.write_text(page)


if __name__ == "__main__":
    build()
