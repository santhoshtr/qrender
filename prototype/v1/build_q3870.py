#!/usr/bin/env python3
"""Build the static Nairobi (Q3870) prototype page, with Gujarati labels.

The data comes from the same qjson SPARQL query, asked with the label
language "gu, mul, en". Gujarati labels appear where Wikidata has them and
English ones where it does not, which is what a reader in Gujarati sees today.

Nairobi holds 58 statements against the 435 of Paris, so the page is not the
Paris page with other values in it. Sections that have no data are simply not
there, and the sections that remain take the room.

Usage: python3 prototype/v1/build_q3870.py
"""

import json
import math
import re
import sys
import urllib.parse
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from factoid_kit import *  # noqa: F403

QID = "Q3870"
OUT = ROOT / "prototype" / "v1" / "q3870.html"
FIXTURE = ROOT / "prototype" / "v1" / "data" / "q3870-gu.sparql.json"
BOUNDARY = ROOT / "prototype" / "v1" / "data" / "q3870-boundary.geojson"

load(FIXTURE)


# ---------------------------------------------------------------- sections

def section_hero():
    banner = one("P948")
    arms = one("P94")
    flag = one("P41")
    audio = one("P443")
    desc_lang = item()["description_lang"]

    emblems = "".join(
        f'<img src="{e(commons(s["value"], 220))}" alt="{e(s["plabel"])}">'
        for s in (arms, flag) if s
    )
    # The description has no Gujarati value, so the label service falls back.
    # Say which language the reader is getting.
    lang_tag = f'<span class="lang">{e(desc_lang)}</span>' if desc_lang else ""
    audio_html = ""
    if audio:
        audio_html = (f'<span class="hero-chip"><span class="lang">'
                      f'{e(qual(audio, "P407") or "")}</span>{e(audio["plabel"])}</span>'
                      f'<audio controls preload="none" src="{e(commons(audio["value"]))}"></audio>')
    return f"""<section class="hero span-12">
  <img class="hero-banner" src="{e(commons(banner["value"], 2000))}" alt="">
  <div class="hero-body">
    <div>
      <h1>{e(item()["label"])}</h1>
      <p class="hero-desc">{e(item()["description"])} {lang_tag}</p>
      <div class="hero-meta">
        <a class="qid-badge" href="{wd(QID)}">{QID}</a>
        {audio_html}
      </div>
    </div>
    <div class="hero-emblems">{emblems}</div>
  </div>
</section>"""


def section_keyfacts():
    pop = one("P1082")
    area = one("P2046")
    elev = one("P2044")
    inception = one("P571")
    postal = one("P281")
    dialing = one("P473")
    tiles = [
        stat("groups", pop["plabel"], num(pop["value"]),
             note=year_str(qual(pop, "P585"))),
        stat("straighten", area["plabel"], num(area["value"]), unit=area["unit"]),
        stat("altitude", elev["plabel"], num(elev["value"]), unit=elev["unit"]),
        stat("calendar_month", inception["plabel"],
             fmt_date(inception["value"], inception["precision"])),
        stat("home", postal["plabel"], postal["value"]),
        stat("call", dialing["plabel"], dialing["value"]),
    ]
    return '<section class="keyfacts span-12">' + "".join(tiles) + "</section>"


# ------------------------------------------------------------------ the map

MAP_W, MAP_H = 800, 460


def boundary_rings():
    geo = json.loads(BOUNDARY.read_text())
    for feature in geo["features"]:
        for polygon in feature["geometry"]["coordinates"]:
            for ring in polygon:
                yield ring


def fit_zoom(centre, rings):
    """Largest zoom that still holds the whole outline in the picture."""
    for zoom in range(14, 6, -1):
        cx, cy = mercator(centre[0], centre[1], zoom)
        if all(
            abs(mercator(lat, lon, zoom)[0] - cx) < MAP_W / 2 - 12
            and abs(mercator(lat, lon, zoom)[1] - cy) < MAP_H / 2 - 12
            for ring in rings for lon, lat in ring
        ):
            return zoom
    return 9


def section_geography():
    point = parse_point(one("P625")["value"])
    rings = list(boundary_rings())
    # Frame the picture on the outline, and mark the coordinate inside it.
    lats = [lat for ring in rings for _, lat in ring]
    lons = [lon for ring in rings for lon, _ in ring]
    centre = ((min(lats) + max(lats)) / 2, (min(lons) + max(lons)) / 2)
    zoom = fit_zoom(centre, rings)
    cx, cy = mercator(centre[0], centre[1], zoom)

    def to_px(lat, lon):
        x, y = mercator(lat, lon, zoom)
        return x - cx + MAP_W / 2, y - cy + MAP_H / 2

    paths = []
    for ring in rings:
        pts = [to_px(lat, lon) for lon, lat in ring]
        paths.append("M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in pts) + " Z")
    ccx, ccy = to_px(*point)

    osm = one("P402")
    locator = one("P242")
    coord = one("P625")
    map_url = (f"https://maps.wikimedia.org/img/osm-intl,{zoom},"
               f"{centre[0]},{centre[1]},{MAP_W}x{MAP_H}@2x.png")
    map_html = f"""<figure class="map">
  {vmenu(coord["plabel"])}
  <img src="{map_url}" width="{MAP_W}" height="{MAP_H}" alt="{e(item()['label'])}">
  <svg viewBox="0 0 {MAP_W} {MAP_H}" aria-hidden="true">
    <path d="{' '.join(paths)}" fill="var( --color-progressive )" fill-opacity="0.12"
          stroke="var( --color-progressive )" stroke-width="2"/>
    <circle cx="{ccx:.1f}" cy="{ccy:.1f}" r="7" fill="var( --color-progressive )"
            stroke="#fff" stroke-width="2"><title>{e(coord["plabel"])}: {point[0]}, {point[1]}</title></circle>
  </svg>
  <figcaption>{e(coord["plabel"])} {point[0]}, {point[1]} ·
    {e(osm["plabel"])} {e(osm["value"])}</figcaption>
</figure>"""

    country = one("P17")
    admin = one("P131")
    part = one("P361")
    area = one("P2046")
    elev = one("P2044")
    body = f"""
<div class="cols-map">
  {map_html}
  <div>
    <div class="tile">
      {facts([
        (e(country["plabel"]), chip(label_of(country), img=country["vimg"], link=wd(value_qid(country)), menu=False)),
        (e(admin["plabel"]), chip(label_of(admin), img=admin["vimg"], link=wd(value_qid(admin)), menu=False)),
        (e(part["plabel"]), chip(label_of(part), img=part["vimg"], link=wd(value_qid(part)), menu=False)),
        (e(area["plabel"]), f'{num(area["value"])} {e(area["unit"])}'),
        (e(elev["plabel"]), f'{num(elev["value"])} {e(elev["unit"])}'),
      ])}
    </div>
    <figure class="inset-map">
      {vmenu(locator["plabel"])}
      <img src="{e(commons(locator["value"], 600))}" alt="{e(locator["plabel"])}" loading="lazy">
      <figcaption>{e(locator["plabel"])}</figcaption>
    </figure>
  </div>
</div>
"""
    return section(12, "map", "Geography", None, body)


# ----------------------------------------------------------------- the chart

def year_bars(series, unit_note=None):
    """Three census counts, years apart. Bars from zero, not a line: the
    values are separate counts, not a measured curve."""
    top = max(v for _, v in series)
    cols = "".join(
        f'<div class="bar-col"><span class="bar-value">{num(v)}</span>'
        f'<span class="bar" style="block-size: {v / top * 100:.1f}%"></span>'
        f'<span class="bar-year">{y}</span></div>'
        for y, v in series
    )
    note = f'<div class="note">{e(unit_note)}</div>' if unit_note else ""
    return f'<div class="bars">{cols}</div>{note}'


def section_demographics():
    pops = sorted(
        ((year_of(qual(s, "P585")), int(float(s["value"]))) for s in P("P1082")),
        key=lambda t: t[0],
    )
    plabel = P("P1082")[0]["plabel"]
    body = f"""
<div class="tile">
  <h3>{e(plabel)}{vmenu(plabel)}</h3>
  {year_bars(pops)}
</div>
"""
    return section(6, "groups", "Demographics", f"{num(pops[-1][1])} · {pops[-1][0]}", body)


def section_names():
    official = one("P1448")
    native = one("P1705")
    demonyms = P("P1549")
    hashtag = one("P2572")

    demo_chips = "".join(
        chip(d["value"], lang=d["vlang"], note=qual(d, "P518") or None)
        for d in demonyms
    )
    body = f"""
<div class="tile">
  {facts([
      (e(official["plabel"]), e(official["value"])),
      (e(native["plabel"]), e(native["value"])),
      (e(hashtag["plabel"]), e(hashtag["value"])),
  ])}
</div>
<div class="tile">
  <h3>{e(demonyms[0]["plabel"])} ({len(demonyms)}){vmenu(demonyms[0]["plabel"])}</h3>
  <div class="chips">{demo_chips}</div>
</div>
"""
    return section(6, "article", "Names", None, body)


def section_government():
    mayor = one("P6")
    site = one("P856")
    portal = one("P8402")
    languages = P("P2936")
    body = f"""
<div class="tile">
  <h3>{e(mayor["plabel"])}{vmenu(mayor["plabel"])}</h3>
  <div class="thumb feature">
    <img src="{e(commons(mayor["vimg"], 700))}" alt="" loading="lazy">
    <div class="thumb-body">
      <span class="thumb-title"><a href="{wd(value_qid(mayor))}">{e(label_of(mayor))}</a></span>
      <span class="note">{e(timespan(mayor))}</span>
    </div>
  </div>
</div>
<div class="tile">
  <h3>{e(languages[0]["plabel"])}{vmenu(languages[0]["plabel"])}</h3>
  {chips_of(languages)}
</div>
<div class="tile">
  {facts([
      (e(site["plabel"]), f'<a href="{e(site["value"])}">{e(site["value"])}</a>'),
      (e(portal["plabel"]), e(label_of(portal))),
  ])}
</div>
"""
    return section(6, "how_to_vote", "Government", label_of(mayor), body)


def section_history():
    inception = one("P571")
    capital = P("P1376")
    history = one("P2184")
    source = one("P1343")

    years = [y for s in capital for y in
             (year_of(qual(s, "P580")), year_of(qual(s, "P582"))) if y]
    years.append(year_of(inception["value"]))
    axis_start, axis_end = min(years), max(years + [2026])
    total = axis_end - axis_start

    bands = []
    for s in sorted(capital, key=lambda s: year_of(qual(s, "P580") or "") or 9999):
        start = year_of(qual(s, "P580")) or axis_start
        end = year_of(qual(s, "P582")) or axis_end
        left = (start - axis_start) / total * 100
        width = max((end - start) / total * 100, 1.5)
        span = timespan(s)
        bands.append(
            f'<div class="band"><span><a href="{wd(value_qid(s))}">{e(label_of(s))}</a></span>'
            f'<span class="band-track"><span class="band-fill" style="inset-inline-start: {left:.2f}%; '
            f'inline-size: {width:.2f}%;" title="{e(span)}"></span></span>'
            f'<span class="band-years">{e(span)}</span>{vmenu(label_of(s))}</div>'
        )
    bands.append(f'<div class="band"><span></span>'
                 f'<span class="band-axis"><span>{axis_start}</span><span>{axis_end}</span></span>'
                 f'<span></span></div>')

    body = f"""
<div class="tile">
  {facts([
      (e(inception["plabel"]), fmt_date(inception["value"], inception["precision"])),
      (e(history["plabel"]), f'<a href="{wd(value_qid(history))}">{e(label_of(history))}</a>'),
      (e(source["plabel"]), e(label_of(source)) + f' <span class="note">{e(qual(source, "P805") or "")}</span>'),
  ])}
</div>
<div class="tile">
  <h3>{e(capital[0]["plabel"])} ({len(capital)}){vmenu(capital[0]["plabel"])}</h3>
  <div class="bands">{"".join(bands)}</div>
</div>
"""
    return section(6, "event", "History", None, body)


def section_connections():
    twins = P("P190")
    strip = thumb_row(
        sorted(twins, key=lambda s: (year_of(qual(s, "P580") or "") or 9999, label_of(s))),
        "dlg-twins", f'{twins[0]["plabel"]} ({len(twins)})', shown=7)
    body = f"""
<div>
  <h3>{e(twins[0]["plabel"])} ({len(twins)}){vmenu(twins[0]["plabel"])}</h3>
  {strip}
</div>
"""
    return section(12, "communities", "Connections", None, body)


def section_views():
    video = one("P10")
    audio = one("P443")
    media_pids = ("P18", "P2716", "P8592", "P3451", "P948")
    figures = []
    for pid in media_pids:
        for st in P(pid):
            figures.append(
                f'<figure><a href="{e(commons(st["value"]))}">'
                f'<img src="{e(commons(st["value"], 640))}" alt="{e(st["plabel"])}" loading="lazy"></a>'
                f'{vmenu(st["plabel"])}'
                f'<figcaption><b>{e(st["plabel"])}</b></figcaption></figure>'
            )
    # Drawings, not photographs: they keep their own strip, uncropped.
    emblems = [
        f'<figure title="{e(st["plabel"])}">'
        f'<img src="{e(commons(st["value"], 300))}" alt="{e(st["plabel"])}" loading="lazy">'
        f'{vmenu(st["plabel"])}'
        f'<figcaption><b>{e(st["plabel"])}</b></figcaption></figure>'
        for pid in ("P94", "P41") for st in P(pid)
    ]
    body = f"""
<div class="cols-map">
  <figure class="video">
    {vmenu(video["plabel"])}
    <video controls preload="metadata" src="{e(commons(video["value"]))}"></video>
    <figcaption>{e(video["plabel"])} · {e(urllib.parse.unquote(local_name(video["value"])))}</figcaption>
  </figure>
  <div class="tile">
    <h3>{e(audio["plabel"])}{vmenu(audio["plabel"])}</h3>
    <audio controls preload="none" src="{e(commons(audio["value"]))}"></audio>
    <div class="note">{e(qual(audio, "P407") or "")}</div>
  </div>
</div>
<div class="gallery">{"".join(figures)}</div>
<div class="gallery emblems">{"".join(emblems)}</div>
"""
    return section(12, "photo_library", "Views and sound",
                   f"{len(figures) + len(emblems) + 2}", body)


def section_wikimedia():
    cats = [s for pid in ("P910", "P1464", "P1465", "P1792", "P7867", "P8989") for s in P(pid)]
    focus = P("P5008")
    economy = one("P8744")
    commons_cat = one("P373")
    commons_gal = one("P935")

    ids = [s for s in statements() if s["ptype"] == "ExternalId"]
    id_rows = "".join(
        f'<div><span class="k">{e(s["plabel"])}</span><span class="v">{e(s["value"])}</span></div>'
        for s in sorted(ids, key=lambda s: s["plabel"].lower())
    )
    add_dialog("dlg-ids", f"External identifiers ({len(ids)})",
               f'<div class="idlist">{id_rows}</div>')

    body = f"""
<div class="cols">
  <div class="tile">
    <h3>{e(commons_cat["plabel"])} · {e(commons_gal["plabel"])}</h3>
    <div class="chips">
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/Category:{e(commons_cat["value"])}">{e(commons_cat["value"])}</a></span>
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/{e(commons_gal["value"])}">{e(commons_gal["value"])}</a></span>
    </div>
    <h3 style="margin-block-start: 16px;">{e(economy["plabel"])}</h3>
    {chips_of([one("P8744")], with_images=False)}
  </div>
  <div class="tile">
    <h3>{e(cats[0]["plabel"])}</h3>
    {chips_of(cats, with_images=False)}
  </div>
  <div class="tile">
    <h3>{e(focus[0]["plabel"])}</h3>
    {chips_of(focus, with_images=False)}
  </div>
</div>
<div class="chips">
  {more_button("dlg-ids", f"External identifiers ({len(ids)})", "tag")}
</div>
"""
    return section(12, "sell", "On Wikimedia projects", None, body, tint="grey")


# From the English Wikipedia article. This page is in
# Gujarati, so the action on the paragraph is "Translate".
LEAD = {
    "text": "Nairobi, is the capital and largest city of Kenya, located in "
            "the south-central part of the country. As of 2026, it has a "
            "population of 5.6 million and a metropolitan population of 15.95 "
            "million, making it the 8th most populous city in Africa and the "
            "4th most populous metropolitan area. The city goes beyond its "
            "borders and has expanded into neighbouring counties, making it "
            "the 4th largest metropolitan area in Africa.",
    "title": "Nairobi",
    "url": "https://en.wikipedia.org/wiki/Nairobi",
    "project": "English Wikipedia",
}

WORDS = {
    "wordmark": "વિકિપીડિયા",
    "tagline": '<img src="https://gu.wikipedia.org/static/images/mobile/copyright/wikipedia-tagline-gu.svg" alt="" height="14">',
    "search": "શોધો",
    "donate": "દાન આપો",
    "talk": "ચર્ચા",
    "user": "Sthottingal",
}

FOOTER = (
    'Prototype. Every label, number, date, image and link on this page comes '
    f'from the Wikidata item <a href="{wd(QID)}">{QID}</a>, as the qjson SPARQL '
    'query returns it with the label language "gu, mul, en" '
    '(<code>prototype/v1/data/q3870-gu.sparql.json</code>). English strings are '
    'the fallback of that chain: Wikidata has no Gujarati label for them. '
    'Images, audio and video come from Wikimedia Commons. Map tiles come from '
    'maps.wikimedia.org, and the city outline comes from the OpenStreetMap '
    'relation that property P402 names. Section names and the words "from", '
    '"until" and "See all" are page chrome, not data. The Wikipedia header is '
    'a mock-up; its words come from the Gujarati Wikipedia interface. The lead '
    'paragraph is not Wikidata either: it comes from the '
    '<a href="https://en.wikipedia.org/wiki/Nairobi">English Wikipedia</a> '
    'article, in English, which is why the action on it is "Translate".'
)


def build():
    sections = [
        section_hero(),
        lead_section(LEAD["text"], LEAD["title"], LEAD["url"], LEAD["project"],
                     "Translate"),
        section_keyfacts(),
        section_geography(),
        section_demographics(),
        section_names(),
        section_government(),
        section_history(),
        section_connections(),
        section_views(),
        section_wikimedia(),
    ]
    render_page(OUT, QID, "gu", sections, WORDS, FOOTER)


if __name__ == "__main__":
    build()
