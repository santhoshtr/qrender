#!/usr/bin/env python3
"""Build the static Paris (Q90) prototype page.

Every string, number, date, image and link on the page comes from a row of
qjson/tests/fixtures/Q90.sparql.json. The only text that is not data is page
chrome ("See all", "Close") and the section names.

Usage: python3 prototype/v1/build_q90.py
"""

import json
import math
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from factoid_kit import *  # noqa: F403

QID = "Q90"
OUT = ROOT / "prototype" / "v1" / "q90.html"
FIXTURE = ROOT / "qjson" / "tests" / "fixtures" / "Q90.sparql.json"
BOUNDARY = ROOT / "prototype" / "v1" / "data" / "q90-boundary.geojson"

load(FIXTURE)


# ---------------------------------------------------------------- sections

def section_hero():
    banner = one("P948")
    arms = one("P94")
    flag = one("P41")
    audio = one("P443")
    # One language only while we prototype: the page is built for one label
    # language at a time.
    nicks = [n for n in P("P1449") if n["vlang"] == "en"]

    chips_html = "".join(
        f'<span class="hero-chip">{e(n["value"])}</span>' for n in nicks
    )
    emblems = "".join(
        f'<img src="{e(commons(s["value"], 220))}" alt="{e(s["plabel"])}">'
        for s in (arms, flag) if s
    )
    audio_html = ""
    if audio:
        audio_html = f'<audio controls preload="none" src="{e(commons(audio["value"]))}"></audio>'
    return f"""<section class="hero span-12">
  <img class="hero-banner" src="{e(commons(banner["value"], 2000))}" alt="">
  <div class="hero-body">
    <div>
      <h1>{e(item()["label"])}</h1>
      <p class="hero-desc">{e(item()["description"])}</p>
      <div class="hero-meta">
        <a class="qid-badge" href="{wd(QID)}">{QID}</a>
        {chips_html}{audio_html}
      </div>
    </div>
    <div class="hero-emblems">{emblems}</div>
  </div>
</section>"""


def section_keyfacts():
    pop = one("P1082")
    area = one("P2046")
    elev = one("P2044")
    elevs = {qual(s, "P3831"): s for s in P("P2044")}
    inception = one("P571")
    budget = one("P2769")
    zones = P("P421")

    standard = next((z for z in zones if qual(z, "P1264") == "standard time"), zones[0])
    zone_value = label_of(standard)
    zone_note = " · ".join(
        f'{label_of(z)} {qual(z, "P1264") or ""}'.strip()
        for z in zones if z is not standard)

    low = elevs.get("lowest point")
    high = elevs.get("highest point")
    elev_note = None
    if low and high:
        elev_note = f'{num(low["value"])}–{num(high["value"])} {high["unit"]}'

    tiles = [
        stat("groups", pop["plabel"], num(pop["value"]),
             note=f'{qual(pop, "P459")} {year_str(qual(pop, "P585"))}'),
        stat("straighten", area["plabel"], num(area["value"]), unit=area["unit"]),
        stat("altitude", elev["plabel"], num(elev["value"]), unit=elev["unit"],
             note=elev_note),
        stat("calendar_month", inception["plabel"],
             fmt_date(inception["value"], inception["precision"])),
        stat("policy", budget["plabel"], num(budget["value"]), unit=budget["unit"],
             note=year_str(qual(budget, "P585")), tight=True),
        stat("pace", zones[0]["plabel"], zone_value, note=zone_note),
    ]
    return '<section class="keyfacts span-12">' + "".join(tiles) + "</section>"


def section_names():
    official = one("P1448")
    native = one("P1705")
    nicks = P("P1449")
    demonyms = P("P1549")
    named_after = one("P138")
    replaces = P("P1365")

    rows = "".join(
        f'<tr><td>{e(d["value"])}</td><td>{e(d["vlang"] or "")}</td>'
        f'<td>{e(", ".join(q["plabel"] + ": " + q["value"] for q in d["quals"]))}</td></tr>'
        for d in sorted(demonyms, key=lambda x: (x["vlang"] or ""))
    )
    add_dialog("dlg-demonym", f'{demonyms[0]["plabel"]} ({len(demonyms)})',
               '<table class="vtable"><thead><tr><th>value</th><th>language</th>'
               f'<th>qualifiers</th></tr></thead><tbody>{rows}</tbody></table>')

    english = [d for d in demonyms if d["vlang"] == "en"]
    demo_chips = "".join(
        chip(d["value"], note=qual(d, "P11054") or qual(d, "P518") or None)
        for d in english
    )

    body = f"""
<div class="tile">
  {facts([
      (e(official["plabel"]), e(official["value"])),
      (e(native["plabel"]), e(native["value"])),
      (e(nicks[0]["plabel"]), " · ".join(e(n["value"]) for n in nicks if n["vlang"] == "en")),
      (e(named_after["plabel"]), f'<a href="{wd(value_qid(named_after))}">{e(label_of(named_after))}</a>'),
      (e(replaces[0]["plabel"]), " · ".join(
          f'<a href="{wd(value_qid(r))}">{e(label_of(r))}</a>' + (
              f' <span class="note">{e(timespan(r))}</span>' if timespan(r) else "")
          for r in replaces)),
  ])}
</div>
<div class="tile">
  <h3>{e(demonyms[0]["plabel"])}</h3>
  <div class="chips">{demo_chips}</div>
</div>
{more_button("dlg-demonym", f"See all {len(demonyms)} in {len({d['vlang'] for d in demonyms})} languages", "language")}
"""
    return section(6, "article", "Names", None, body)


def section_symbols():
    media = []
    for pid in ("P94", "P4004", "P41", "P154"):
        for st in P(pid):
            legends = quals(st, "P2096", lang="en")
            media.append((st, legends[0] if legends else None))
    motto_text = one("P1451")
    motto_item = one("P1546")
    patron = one("P417")
    arms_item = one("P237")
    flag_item = one("P163")

    tiles = "".join(
        f'<figure title="{e(legend or st["plabel"])}">'
        f'<img src="{e(commons(st["value"], 300))}" alt="{e(st["plabel"])}" loading="lazy">'
        f'{vmenu(st["plabel"])}'
        f'<figcaption><b>{e(st["plabel"])}</b></figcaption></figure>'
        for st, legend in media
    )
    translations = quals(motto_text, "P2441", lang="en")
    motto_note = " · ".join(x for x in [
        qual(motto_item, "P407"),
        f'from {year_str(qual(motto_item, "P580"))}' if qual(motto_item, "P580") else None,
        qual(motto_item, "P898"),
    ] if x)
    body = f"""
<div class="cols">
  <div class="tile">
    <h3>{e(motto_text["plabel"])}</h3>
    <p style="font-family: var( --font-family-serif ); font-size: var( --font-size-xx-large ); margin: 0;">{e(motto_text["value"])}</p>
    <p class="note" style="margin: 4px 0 0;">{e(translations[0] if translations else "")}</p>
    <p class="note" style="margin: 2px 0 0;">{e(motto_note)}</p>
  </div>
  <div class="tile">
    {facts([
        (e(patron["plabel"]), chip(label_of(patron), img=patron["vimg"], link=wd(value_qid(patron)), menu=False)),
        (e(arms_item["plabel"]), chip(label_of(arms_item), img=arms_item["vimg"], link=wd(value_qid(arms_item)), menu=False)),
        (e(flag_item["plabel"]), chip(label_of(flag_item), img=flag_item["vimg"], link=wd(value_qid(flag_item)), menu=False)),
    ])}
  </div>
</div>
<div class="gallery emblems">{tiles}</div>
"""
    return section(6, "trophy", "Symbols", None, body)


# ----------------------------------------------------------------- the chart

def population_chart(series):
    w, h = 760, 250
    ml, mr, mt, mb = 78, 18, 22, 26
    years = [y for y, _ in series]
    values = [v for _, v in series]
    x0, x1 = min(years), max(years)
    step = 50000
    y0 = math.floor(min(values) / step) * step
    y1 = math.ceil(max(values) / step) * step

    def px(year):
        return ml + (year - x0) / (x1 - x0) * (w - ml - mr)

    def py(value):
        return mt + (1 - (value - y0) / (y1 - y0)) * (h - mt - mb)

    grid, ticks = [], []
    v = y0
    while v <= y1:
        y = py(v)
        grid.append(f'<line class="grid-line" x1="{ml}" y1="{y:.1f}" x2="{w - mr}" y2="{y:.1f}"/>')
        grid.append(f'<text class="axis-text" x="{ml - 8}" y="{y + 4:.1f}" text-anchor="end">{num(v)}</text>')
        v += step
    # Keep year ticks readable: drop a label that would touch its neighbour,
    # but always keep the first and the last year.
    keep, last_x = [], None
    for year, _ in series:
        x = px(year)
        if last_x is None or x - last_x >= 46:
            keep.append(year)
            last_x = x
    if series[-1][0] not in keep:
        if keep and px(series[-1][0]) - px(keep[-1]) < 46:
            keep.pop()
        keep.append(series[-1][0])
    for year in keep:
        ticks.append(f'<text class="axis-text" x="{px(year):.1f}" y="{h - 8}" '
                     f'text-anchor="middle">{year}</text>')

    line = "M" + " L".join(f"{px(y):.1f},{py(v):.1f}" for y, v in series)
    area = (f'{line} L{px(x1):.1f},{py(y0):.1f} L{px(x0):.1f},{py(y0):.1f} Z')
    dots = "".join(
        f'<circle class="series-dot" cx="{px(y):.1f}" cy="{py(v):.1f}" r="4">'
        f'<title>{num(v)} · {y}</title></circle>'
        for y, v in series
    )
    # Label the ends and the peak only, and keep the text inside the frame.
    marked = sorted({series[0], series[-1], max(series, key=lambda s: s[1])})
    labels = ""
    for y, v in marked:
        x = px(y)
        anchor = "start" if x < ml + 30 else ("end" if x > w - mr - 30 else "middle")
        labels += (f'<text class="point-label" x="{x:.1f}" y="{py(v) - 12:.1f}" '
                   f'text-anchor="{anchor}">{num(v)}</text>')
    return (f'<svg class="chart" viewBox="0 0 {w} {h}" role="img" '
            f'aria-label="population by census year">{"".join(grid)}'
            f'<path class="series-area" d="{area}"/><path class="series-line" d="{line}"/>'
            f'{dots}{labels}{"".join(ticks)}</svg>')


def section_demographics():
    pops = sorted(
        ((year_of(qual(s, "P585")), int(float(s["value"]))) for s in P("P1082") if qual(s, "P585")),
        key=lambda t: t[0],
    )
    male, female = one("P1540"), one("P1539")
    weddings = one("P5982")
    latest = pops[-1]
    method = qual(P("P1082")[0], "P459")

    m, f = int(float(male["value"])), int(float(female["value"]))
    total = m + f
    split_year = year_str(qual(male, "P585"))
    split = f"""<div class="splitbar">
  <span style="flex: {m}; background-color: var( --color-option-blue );">{num(m)}</span>
  <span style="flex: {f}; background-color: var( --color-option-purple );">{num(f)}</span>
</div>
<div class="chips" style="margin-block-start: 8px;">
  <span class="chip">{icon("man")} {e(male["plabel"])} <span class="note">{m / total * 100:.1f}%</span></span>
  <span class="chip">{icon("woman")} {e(female["plabel"])} <span class="note">{f / total * 100:.1f}%</span></span>
  <span class="chip"><span class="note">{e(method)} {split_year}</span></span>
</div>"""

    body = f"""
<div class="tile">
  <h3>{e(P("P1082")[0]["plabel"])} · {e(method)} {pops[0][0]}–{pops[-1][0]}{vmenu(P("P1082")[0]["plabel"])}</h3>
  {population_chart(pops)}
</div>
<div class="cols">
  <div class="tile">
    <h3>{e(male["plabel"])} · {e(female["plabel"])}{vmenu(male["plabel"])}</h3>
    {split}
  </div>
  <div class="tile">
    <h3>{e(weddings["plabel"])}{vmenu(weddings["plabel"])}</h3>
    <div class="bignum">{num(weddings["value"])}</div>
    <div class="note">{e(year_str(qual(weddings, "P585")))}</div>
  </div>
</div>
"""
    return section(12, "groups", "Demographics", f'{num(latest[1])} in {latest[0]}', body)


def section_government():
    mayors = P("P6")
    dated = sorted(mayors, key=lambda s: year_of(qual(s, "P580")) or -9999, reverse=True)
    current = next((s for s in mayors if s["rank"] == "Preferred"), dated[0])
    office = one("P1313")
    council = one("P194")
    budget = one("P2769")
    site = one("P856")
    jobs = P("P10311")
    contact = one("P11266")
    portal = one("P8402")
    owns = P("P1830")

    def mayor_note(st):
        return " · ".join(x for x in [timespan(st), qual(st, "P102"),
                                      qual(st, "P1534")] if x)

    earlier = [s for s in dated if s is not current]
    mayor_strip = thumb_row(earlier, "dlg-mayors",
                            f'{mayors[0]["plabel"]} ({len(mayors)})',
                            note_fn=mayor_note, portraits=True)
    own_strip = thumb_row(sorted(owns, key=lambda s: label_of(s)), "dlg-owns",
                          f'{owns[0]["plabel"]} ({len(owns)})',
                          note_fn=lambda s: None)
    links = facts([
        (e(s["plabel"]), f'<a href="{e(s["value"])}">{e(s["value"])}</a>')
        for s in [site] + jobs + [contact] if s
    ])
    body = f"""
<div class="cols">
  <div class="tile">
    <h3>{e(label_of(office))}</h3>
    <div class="thumb feature">
      <img src="{e(commons(current["vimg"], 700))}" alt="" loading="lazy">
      {vmenu(label_of(current))}
      <div class="thumb-body">
        <span class="thumb-title"><a href="{wd(value_qid(current))}">{e(label_of(current))}</a></span>
        <span class="note">{e(timespan(current))}{" · " + e(qual(current, "P102")) if qual(current, "P102") else ""}</span>
      </div>
    </div>
    <div class="note" style="margin-block-start: 8px;">{e(office["plabel"])}: {e(label_of(office))} {e(timespan(office))}</div>
  </div>
  <div class="tile">
    {facts([
      (e(council["plabel"]), e(label_of(council))),
      (e(one("P797")["plabel"]), e(label_of(one("P797")))),
      (e(one("P1454")["plabel"]), e(label_of(one("P1454")))),
      (e(budget["plabel"]), f'{num(budget["value"])} {e(budget["unit"])} <span class="note">{e(year_str(qual(budget, "P585")))}</span>'),
      (e(portal["plabel"]), e(label_of(portal))),
      (e(one("P37")["plabel"]), e(label_of(one("P37")))),
      (e(one("P708")["plabel"]), e(label_of(one("P708")))),
    ])}
  </div>
</div>
<div>
  <h3>{e(mayors[0]["plabel"])} ({len(mayors)})</h3>
  {mayor_strip}
</div>
<div class="tile">
  <h3>{e(site["plabel"])}</h3>
  {links}
</div>
<div>
  <h3>{e(owns[0]["plabel"])} ({len(owns)})</h3>
  {own_strip}
</div>
"""
    return section(12, "how_to_vote", "Government", label_of(current), body)


def all_years():
    years = []
    for s in statements():
        for q in s["quals"]:
            if q["pid"] in ("P580", "P582", "P585", "P577"):
                y = year_of(q["value"])
                if y:
                    years.append(y)
    return years


def section_history():
    countries = P("P17")
    spans = []
    for s in countries:
        start = year_of(qual(s, "P580"))
        end = year_of(qual(s, "P582"))
        spans.append((s, start, end))
    axis_start = min(x for _, x, _ in spans if x)
    axis_end = max(all_years())
    total = axis_end - axis_start

    bands = []
    for s, start, end in sorted(spans, key=lambda t: t[1] or 0):
        a = start if start else axis_start
        b = end if end else axis_end
        left = (a - axis_start) / total * 100
        width = max((b - a) / total * 100, 0.4)
        label = timespan(s) or ""
        bands.append(
            f'<div class="band"><span><a href="{wd(value_qid(s))}">{e(label_of(s))}</a></span>'
            f'<span class="band-track"><span class="band-fill" style="inset-inline-start: {left:.2f}%; '
            f'inline-size: {width:.2f}%;" title="{e(label)}"></span></span>'
            f'<span class="band-years">{e(label)}</span>{vmenu(label_of(s))}</div>'
        )
    band_scale = (f'<div class="band"><span></span>'
                  f'<span class="band-axis"><span>{axis_start}</span><span>{axis_end}</span></span>'
                  f'<span></span></div>')

    events = P("P793")
    dated = sorted(
        [s for s in events if qual(s, "P585")],
        key=lambda s: year_of(qual(s, "P585")),
    )
    event_html = "".join(
        f'<div class="event"><span class="event-year">{year_str(qual(s, "P585"))}</span>'
        + (f'<img src="{e(commons(s["vimg"], 160))}" alt="" loading="lazy">' if s["vimg"] else "")
        + f'<span><a href="{wd(value_qid(s))}">{e(label_of(s))}</a></span>'
        + vmenu(label_of(s)) + "</div>"
        for s in dated
    )
    rows = "".join(
        f'<tr><td>{e(year_str(qual(s, "P585") or "") or "—")}</td>'
        f'<td><a href="{wd(value_qid(s))}">{e(label_of(s))}</a></td></tr>'
        for s in sorted(events, key=lambda s: year_of(qual(s, "P585") or "") or 9999)
    )
    add_dialog("dlg-events", f'{events[0]["plabel"]} ({len(events)})',
               f'<table class="vtable"><tbody>{rows}</tbody></table>')

    capital = P("P1376")
    awards = P("P166")
    inception = one("P571")
    body = f"""
<div class="tile">
  <h3>{e(countries[0]["plabel"])}</h3>
  <div class="bands">{"".join(bands)}{band_scale}</div>
</div>
<div class="cols">
  <div class="tile">
    <h3>{e(events[0]["plabel"])} ({len(dated)} dated of {len(events)})</h3>
    <div class="events">{event_html}</div>
    {more_button("dlg-events", f"See all {len(events)}", "event")}
  </div>
  <div class="tile">
    <h3>{e(capital[0]["plabel"])} ({len(capital)})</h3>
    {chips_of(capital, with_images=False)}
    <h3 style="margin-block-start: 16px;">{e(awards[0]["plabel"])}</h3>
    {chips_of(awards, note_fn=lambda s: year_str(qual(s, "P585") or "") or None)}
    <h3 style="margin-block-start: 16px;">{e(inception["plabel"])}</h3>
    <div>{e(fmt_date(inception["value"], inception["precision"]))}</div>
  </div>
</div>
"""
    return section(12, "event", "History", None, body)


def section_connections():
    twins = P("P190")
    member = P("P463")
    twin_strip = thumb_row(
        sorted(twins, key=lambda s: (year_of(qual(s, "P580") or "") or 9999, label_of(s))),
        "dlg-twins", f'{twins[0]["plabel"]} ({len(twins)})')
    body = f"""
<div>
  <h3>{e(twins[0]["plabel"])} ({len(twins)})</h3>
  {twin_strip}
</div>
<div class="tile">
  <h3>{e(member[0]["plabel"])}</h3>
  {chips_of(member)}
</div>
"""
    return section(12, "communities", "Connections", None, body)


def section_views():
    media_pids = ("P18", "P2716", "P8592", "P3451", "P4291", "P4640", "P1766", "P1943", "P242")
    figures = []
    for pid in media_pids:
        for st in P(pid):
            notes = []
            if qual(st, "P585"):
                notes.append(fmt_date(qual(st, "P585")))
            if quals(st, "P180"):
                notes.append(", ".join(quals(st, "P180")))
            if qual(st, "P7108"):
                notes.append(qual(st, "P7108"))
            legends = quals(st, "P2096", lang="en")
            if legends:
                notes.append(legends[0])
            figures.append(
                f'<figure><a href="{e(commons(st["value"]))}">'
                f'<img src="{e(commons(st["value"], 640))}" alt="{e(st["plabel"])}" loading="lazy"></a>'
                f'{vmenu(st["plabel"])}'
                f'<figcaption><b>{e(st["plabel"])}</b><span>{e(" · ".join(notes))}</span></figcaption></figure>'
            )
    audios = []
    for st in P("P443"):
        audios.append((st["plabel"], st["value"], qual(st, "P407")))
    native = one("P1705")
    for a in quals(native, "P443"):
        audios.append((f'{native["plabel"]}: {native["value"]}', a, native["vlang"]))
    audio_html = "".join(
        f'<div class="tile"><h3>{e(name)}</h3>'
        f'<audio controls preload="none" src="{e(commons(url))}"></audio>'
        + (f'<div class="note">{e(lang)}</div>' if lang else "") + "</div>"
        for name, url, lang in audios
    )
    shown, hidden = figures[:4], len(figures) - 4
    strip = "".join(shown)
    if hidden > 0:
        strip += more_tile("dlg-views", hidden)
        add_dialog("dlg-views", f"Views ({len(figures)})",
                   '<div class="gallery">' + "".join(figures) + "</div>")
    body = f"""
<div class="gallery row-5">{strip}</div>
<div class="cols">{audio_html}</div>
"""
    return section(12, "photo_library", "Views and sound", f"{len(figures)} files", body)


def section_wikimedia():
    cat_pids = ("P910", "P1464", "P1465", "P1791", "P1792", "P1740", "P7867", "P10280", "P1151", "P1424")
    cats = [s for pid in cat_pids for s in P(pid)]
    topics = [s for pid in ("P2184", "P2633", "P9241", "P8744", "P868", "P2936", "P1456", "P1299")
              for s in P(pid)]
    projects = P("P6104")
    focus = P("P5008")
    sources = P("P1343")
    commons_cat = one("P373")
    commons_gal = one("P935")

    different = P("P1889")
    diff_rows = "".join(
        f'<tr><td>{e(label_of(s))}</td><td><a href="{wd(value_qid(s))}">{value_qid(s)}</a></td></tr>'
        for s in different if value_qid(s)
    )
    add_dialog("dlg-different", f'{different[0]["plabel"]} ({len(different)})',
               f'<table class="vtable"><tbody>{diff_rows}</tbody></table>')

    ids = [s for s in statements() if s["ptype"] == "ExternalId"]
    id_rows = "".join(
        f'<div><span class="k">{e(s["plabel"])}</span><span class="v">{e(s["value"])}</span></div>'
        for s in sorted(ids, key=lambda s: s["plabel"].lower())
    )
    add_dialog("dlg-ids", f"External identifiers ({len(ids)})",
               f'<div class="idlist">{id_rows}</div>')

    source_rows = "".join(
        f'<tr><td>{e(label_of(s))}</td><td>{e(qual(s, "P805") or "")}</td></tr>'
        for s in sources
    )
    add_dialog("dlg-sources", f'{sources[0]["plabel"]} ({len(sources)})',
               f'<table class="vtable"><tbody>{source_rows}</tbody></table>')

    body = f"""
<div class="cols">
  <div class="tile">
    <h3>{e(commons_cat["plabel"])} · {e(commons_gal["plabel"])}</h3>
    <div class="chips">
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/Category:{e(commons_cat["value"])}">{e(commons_cat["value"])}</a></span>
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/{e(commons_gal["value"])}">{e(commons_gal["value"])}</a></span>
    </div>
    <h3 style="margin-block-start: 16px;">{e(projects[0]["plabel"])}</h3>
    {chips_of(projects, with_images=False)}
  </div>
  <div class="tile">
    <h3>{e(cats[0]["plabel"])} and related pages</h3>
    {chips_of(cats, with_images=False)}
  </div>
  <div class="tile">
    <h3>{e(topics[0]["plabel"])} and related topics</h3>
    {chips_of(topics, with_images=False)}
    <h3 style="margin-block-start: 16px;">{e(focus[0]["plabel"])}</h3>
    {chips_of(focus, with_images=False)}
  </div>
</div>
<div class="chips">
  {more_button("dlg-sources", f"{sources[0]['plabel']} ({len(sources)})", "local_library")}
  {more_button("dlg-different", f"{different[0]['plabel']} ({len(different)})", "compare_arrows")}
  {more_button("dlg-ids", f"External identifiers ({len(ids)})", "tag")}
</div>
"""
    return section(12, "sell", "On Wikimedia projects", None, body, tint="grey")


def section_geography():
    centre = parse_point(one("P625")["value"])
    cx, cy = mercator(centre[0], centre[1], MAP_Z)

    def to_px(lat, lon):
        x, y = mercator(lat, lon, MAP_Z)
        return x - cx + MAP_W / 2, y - cy + MAP_H / 2

    rings = []
    geo = json.loads(BOUNDARY.read_text())
    for feature in geo["features"]:
        for polygon in feature["geometry"]["coordinates"]:
            for ring in polygon:
                pts = [to_px(lat, lon) for lon, lat in ring]
                rings.append("M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in pts) + " Z")

    markers = []
    for pid, tag in (("P1332", "N"), ("P1333", "S"), ("P1334", "E"), ("P1335", "W")):
        st = one(pid)
        lat, lon = parse_point(st["value"])
        x, y = to_px(lat, lon)
        markers.append(
            f'<g><circle cx="{x:.1f}" cy="{y:.1f}" r="5" fill="#fff" stroke="#72777d" stroke-width="1.5">'
            f'<title>{e(st["plabel"])}: {lat}, {lon}</title></circle>'
            f'<text x="{x:.1f}" y="{y - 10:.1f}" text-anchor="middle" class="axis-text">{tag}</text></g>'
        )
    ccx, ccy = to_px(*centre)
    markers.append(
        f'<circle cx="{ccx:.1f}" cy="{ccy:.1f}" r="7" fill="var( --color-progressive )" '
        f'stroke="#fff" stroke-width="2"><title>{e(one("P625")["plabel"])}: '
        f'{centre[0]}, {centre[1]}</title></circle>'
    )

    osm = one("P402")
    map_url = (f"https://maps.wikimedia.org/img/osm-intl,{MAP_Z},"
               f"{centre[0]},{centre[1]},{MAP_W}x{MAP_H}@2x.png")
    map_html = f"""<figure class="map">
  {vmenu(one("P625")["plabel"])}
  <img src="{map_url}" width="{MAP_W}" height="{MAP_H}" alt="map of {e(item()['label'])}">
  <svg viewBox="0 0 {MAP_W} {MAP_H}" aria-hidden="true">
    <path d="{' '.join(rings)}" fill="var( --color-progressive )" fill-opacity="0.12"
          stroke="var( --color-progressive )" stroke-width="2"/>
    {''.join(markers)}
  </svg>
  <figcaption>{e(one('P625')['plabel'])} {centre[0]}, {centre[1]} ·
    boundary from {e(osm['plabel'])} {e(osm['value'])} · N/S/E/W = {e(one('P1332')['plabel'])} and the three other extreme points</figcaption>
</figure>"""

    area = one("P2046")
    elevs = P("P2044")
    waters = P("P206")
    admin = sorted(P("P131"), key=lambda s: year_of(qual(s, "P580")) or -9999)
    stat_ent = P("P8138")
    borders = P("P47")
    current_borders = [b for b in borders if not qual(b, "P582")]
    parts = P("P150")
    postal = {qual(s, "P518"): s["value"] for s in P("P281") if qual(s, "P518")}

    elev_rows = " · ".join(
        f'{num(s["value"])} {s["unit"]}' + (f' ({qual(s, "P3831")})' if qual(s, "P3831") else "")
        for s in elevs
    )
    admin_rows = "".join(
        f'<div class="band"><span><a href="{wd(value_qid(s))}">{e(label_of(s))}</a></span>'
        f'<span class="band-years">{e(timespan(s) or "")}</span>{vmenu(label_of(s))}</div>'
        for s in admin
    )
    def numeric_first(st):
        m = re.match(r"^(\d+)", label_of(st))
        return (0, int(m.group(1))) if m else (1, 0)

    part_strip = thumb_row(
        sorted(parts, key=lambda s: (numeric_first(s), label_of(s))),
        "dlg-parts", f'{parts[0]["plabel"]} ({len(parts)})',
        note_fn=lambda s: postal.get(label_of(s)))
    border_rows = "".join(
        f'<tr><td>{e(label_of(b))}</td><td>{e(timespan(b))}</td></tr>'
        for b in sorted(borders, key=lambda b: label_of(b))
    )
    add_dialog("dlg-borders", f'{borders[0]["plabel"]} ({len(borders)})',
               f'<table class="vtable"><tbody>{border_rows}</tbody></table>')

    body = f"""
<div class="cols-map">
  {map_html}
  <div class="tile">
    {facts([
      (e(area["plabel"]), f'{num(area["value"])} {e(area["unit"])}'),
      (e(elevs[0]["plabel"]), elev_rows),
      (e(one("P30")["plabel"]), e(label_of(one("P30")))),
      (e(one("P361")["plabel"]), e(label_of(one("P361")))),
      (e(one("P610")["plabel"]), e(label_of(one("P610")))),
      (e(stat_ent[0]["plabel"]), " · ".join(e(label_of(s)) for s in stat_ent)),
      (e(one("P2670")["plabel"]), e(label_of(one("P2670")))),
      (e(one("P3896")["plabel"]), f'<a href="{e(one("P3896")["value"])}">{e(local_name(one("P3896")["value"]))}</a>'),
    ])}
  </div>
</div>
<div class="tile">
  <h3>{e(waters[0]["plabel"])}</h3>
  {chips_of(waters)}
</div>
<div class="tile">
  <h3>{e(admin[0]["plabel"])}</h3>
  <div class="bands">{admin_rows}</div>
</div>
<div>
  <h3>{e(parts[0]["plabel"])} ({len(parts)}) · {e(P("P281")[0]["plabel"])}</h3>
  {part_strip}
</div>
<div class="tile">
  <h3>{e(borders[0]["plabel"])} ({len(borders)})</h3>
  {chips_of(current_borders[:14], with_images=False)}
</div>
{more_button("dlg-borders", f"See all {len(borders)}", "map")}
"""
    return section(12, "map", "Geography", None, body)


# From the English Wikipedia article.
LEAD = {
    "text": "Paris is the capital and largest city of France, with an "
            "estimated city population of 2.04 million in an area of 105.4 "
            "km2 (40.7 sq mi), as of January 2026, and a metropolitan "
            "population of 13.3 million (2023). Located on the river Seine in "
            "the centre of the Île-de-France region, it is the largest "
            "metropolitan area and fourth-most populous city in the European "
            "Union (EU). Paris is nicknamed the \"City of Light\", partly "
            "because of its role in the Age of Enlightenment.",
    "title": "Paris",
    "url": "https://en.wikipedia.org/wiki/Paris",
    "project": "English Wikipedia",
}

WORDS = {
    "wordmark": "WIKIPEDIA",
    "tagline": "The Free Encyclopedia",
    "search": "Search Wikipedia",
    "donate": "Donate",
    "talk": "Talk",
    "user": "Sthottingal",
}

FOOTER = (
    'Prototype. Every label, number, date, image and link on this page comes '
    f'from the Wikidata item <a href="{wd(QID)}">{QID}</a>, as the qjson SPARQL '
    'query returns it (fixture <code>Q90.sparql.json</code>). Images and audio '
    'come from Wikimedia Commons. Map tiles come from maps.wikimedia.org, and '
    'the city outline comes from the OpenStreetMap relation that property P402 '
    'names. Section names and the words "from", "until" and "See all" are page '
    'chrome, not data. The lead paragraph is the one exception: it comes from '
    'the <a href="https://en.wikipedia.org/wiki/Paris">English Wikipedia</a> '
    'article and says so on the page. The Wikipedia header is a mock-up.'
)


def build():
    sections = [
        section_hero(),
        lead_section(LEAD["text"], LEAD["title"], LEAD["url"], LEAD["project"], "Edit"),
        section_keyfacts(),
        section_names(),
        section_symbols(),
        section_geography(),
        section_demographics(),
        section_government(),
        section_history(),
        section_connections(),
        section_views(),
        section_wikimedia(),
    ]
    render_page(OUT, QID, "en", sections, WORDS, FOOTER)


if __name__ == "__main__":
    build()
