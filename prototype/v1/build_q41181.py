#!/usr/bin/env python3
"""Build the static golden eagle (Q41181) prototype page.

The taxon archetype, and the clearest case of a gap so far. Wikidata has 110
sitelinks for this bird, among them Malayalam, Tamil, Bengali and Urdu, but no
Hindi article. It has 39 common names in 34 languages, and not one of them is
Hindi, although India is in the bird's range. The Hindi page therefore reads
in Hindi where Wikidata has Hindi, and says what is missing where it does not.

A taxon also has no life story and no credits. What it has is a name with an
author, a rank in a tree, a conservation status, a range, a voice, and a place
in other people's heraldry.

Usage: python3 prototype/v1/build_q41181.py [hi]
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from factoid_kit import *  # noqa: F403

QID = "Q41181"

LANGUAGES = {
    "en": {
        "fixture": "q41181-en.sparql.json",
        "out": "q41181.html",
        "lead_action": "Edit",
        "lead_missing": None,
        "language_name": "English",
        "words": {
            "wordmark": "WIKIPEDIA",
            "tagline": "The Free Encyclopedia",
            "search": "Search Wikipedia",
            "donate": "Donate",
            "talk": "Talk",
            "user": "Sthottingal",
        },
    },
    "hi": {
        "fixture": "q41181-hi.sparql.json",
        "out": "q41181-hi.html",
        "lead_action": "Translate",
        "lead_missing": "No Hindi article exists for this species, although "
                        "Malayalam, Tamil, Bengali and Urdu all have one. The "
                        "paragraph below is the lead of the English article.",
        "language_name": "Hindi",
        "words": {
            "wordmark": "विकिपीडिया",
            "tagline": '<img src="https://hi.wikipedia.org/static/images/mobile/copyright/wikipedia-tagline-hi.svg" alt="" height="14">',
            "search": "खोजें",
            "donate": "दान करें",
            "talk": "संवाद",
            "user": "Sthottingal",
        },
    },
}

LANG = sys.argv[1] if len(sys.argv) > 1 else "en"
CFG = LANGUAGES[LANG]
OUT = ROOT / "prototype" / "v1" / CFG["out"]
FIXTURE = ROOT / "prototype" / "v1" / "data" / CFG["fixture"]

load(FIXTURE)


def lang_tag(value_lang):
    return (f'<span class="lang-tag">{e(value_lang)}</span>'
            if value_lang and value_lang != LANG else "")


# ---------------------------------------------------------------- sections

def section_hero():
    photo = one("P18")
    rank = one("P105")
    iucn = one("P141")
    wingspan = one("P2050")
    voice = one("P51")
    short = one("P1813")

    chips = [f'<a class="qid-badge" href="{wd(QID)}">{QID}</a>']
    for st, value in ((rank, label_of(rank)), (iucn, label_of(iucn)),
                      (wingspan, f'{num(wingspan["value"])} {wingspan["unit"]}')):
        chips.append(f'<span class="hero-chip"><span class="lang">{e(st["plabel"])}'
                     f'</span>{e(value)}</span>')
    chips.append(f'<span class="hero-chip"><span class="lang">{e(short["plabel"])}'
                 f'</span>{e(short["value"])}</span>')
    voice_html = (f'<span class="hero-chip"><span class="lang">{e(voice["plabel"])}</span></span>'
                  f'<audio controls preload="none" src="{e(commons(voice["value"]))}"></audio>')
    depicts = qual(photo, "P180")
    return f"""<section class="hero photo span-12">
  <img class="hero-backdrop" src="{e(commons(photo["value"], 1200))}" alt="" aria-hidden="true">
  <figure class="hero-figure">
    {vmenu(photo["plabel"])}
    <img src="{e(commons(photo["value"], 900))}" alt="{e(item()["label"])}">
    <figcaption>{e(photo["plabel"])}{" · " + e(depicts) if depicts else ""}</figcaption>
  </figure>
  <div class="hero-body">
    <div>
      <h1>{e(item()["label"])} {lang_tag(item()["label_lang"])}</h1>
      <p class="hero-desc">{e(item()["description"])} {lang_tag(item()["description_lang"])}</p>
      <div class="hero-meta">{"".join(chips)}{voice_html}</div>
    </div>
  </div>
</section>"""


def section_taxonomy():
    name = one("P225")
    rank = one("P105")
    parent = one("P171")
    original = one("P1403")
    synonym = one("P1420")
    instance = one("P31")

    author = qual(name, "P405")
    year = qual(name, "P574")
    author_label = next((q["plabel"] for q in name["quals"] if q["pid"] == "P405"), "")
    name_note = " · ".join(x for x in [
        f'{author_label}: {author}' if author else "",
        year_str(year) if year else "",
        qual(name, "P3831") or "",
    ] if x)
    body = f"""
<div class="tile">
  {facts([
      (e(name["plabel"]), f'<i>{e(name["value"])}</i> <span class="note">{e(name_note)}</span>'),
      (e(rank["plabel"]), chip(label_of(rank), img=rank["vimg"], link=wd(value_qid(rank)), menu=False)),
      (e(parent["plabel"]), chip(label_of(parent), img=parent["vimg"], link=wd(value_qid(parent)), menu=False)),
      (e(original["plabel"]), f'<i>{e(label_of(original))}</i>'),
      (e(synonym["plabel"]), f'<i>{e(label_of(synonym))}</i>'),
      (e(instance["plabel"]), e(label_of(instance))),
  ])}
</div>
"""
    return section(6, "article", "Taxonomy", name["value"], body)


def section_conservation():
    iucn = one("P141")
    cites = one("P7603")
    regional = P("P14254")
    czech = one("P5841")

    badge = ""
    if iucn["vimg"]:
        badge = (f'<figure class="status-badge">{vmenu(iucn["plabel"])}'
                 f'<img src="{e(commons(iucn["vimg"], 400))}" alt="{e(label_of(iucn))}">'
                 f'<figcaption><b>{e(label_of(iucn))}</b>{e(iucn["plabel"])}</figcaption></figure>')
    rows = []
    for st in regional:
        where = qual(st, "P1001") or qual(st, "P3005") or ""
        source = qual(st, "P3680") or qual(st, "P585") or ""
        rows.append((f'{e(label_of(st))} <span class="note">{e(where)}</span>',
                     f'<span class="note">{e(source)}</span>'))
    body = f"""
<div class="cols-map">
  {badge}
  <div class="tile">
    {facts([
        (e(cites["plabel"]), e(label_of(cites))),
        (e(regional[0]["plabel"]), "<br>".join(f'{k} {v}' for k, v in rows)),
        (e(czech["plabel"]), e(label_of(czech))),
    ])}
  </div>
</div>
"""
    return section(6, "policy", "Conservation", label_of(iucn), body)


def section_names():
    names = P("P1843")
    have_page_language = [n for n in names if n["vlang"] == LANG]
    languages = sorted({n["vlang"] for n in names if n["vlang"]})

    # The order of a list of names must not depend on the reading language.
    ordered = sorted(names, key=lambda n: (n["vlang"] or "", n["value"]))
    shown = ordered[:10]
    chips_html = "".join(
        chip(n["value"], lang=n["vlang"], note="preferred" if n["rank"] == "Preferred" else None)
        for n in shown
    )
    rows = "".join(
        f'<tr><td>{e(n["vlang"] or "")}</td><td>{e(n["value"])}</td></tr>' for n in ordered)
    add_dialog("dlg-names", f'{names[0]["plabel"]} ({len(names)})',
               '<table class="vtable"><thead><tr><th>language</th><th>name</th>'
               f'</tr></thead><tbody>{rows}</tbody></table>')

    missing = ""
    if not have_page_language:
        missing = (f'<p class="order-note">None of the {len(names)} common names is in '
                   f'{e(CFG["language_name"])}, although the range below includes India.</p>'
                   f'<button class="invite-button" type="button">'
                   f'{icon("language")}Add a name in {e(CFG["language_name"])}</button>')
    body = f"""
<div class="tile">
  <h3>{e(names[0]["plabel"])} ({len(names)}) · {len(languages)} languages{vmenu(names[0]["plabel"])}</h3>
  <div class="chips">{chips_html}</div>
  {more_button("dlg-names", f"See all {len(names)}", "language")}
</div>
{missing}
"""
    return section(6, "language", "Common names", f"{len(languages)} languages", body)


def section_body():
    wingspan = one("P2050")
    masses = P("P2067")
    top = max(float(m["value"]) for m in masses)
    bars = "".join(
        f'<div class="amount-row">'
        f'<span class="amount-label">{e(qual(m, "P21") or "")}</span>'
        f'<span class="amount-track"><span class="amount-fill" '
        f'style="inline-size: {float(m["value"]) / top * 100:.1f}%"></span></span>'
        f'<span class="amount-value">{num(m["value"])}</span>{vmenu(m["plabel"])}</div>'
        for m in sorted(masses, key=lambda m: float(m["value"]))
    )
    role = next((q["value"] for m in masses for q in m["quals"] if q["pid"] == "P3831"), "")
    body = f"""
<div class="tile">
  <h3>{e(masses[0]["plabel"])} <span class="note">{e(masses[0]["unit"])} · {e(role)}</span>{vmenu(masses[0]["plabel"])}</h3>
  <div class="amounts">{bars}</div>
</div>
<div class="tile">
  {facts([
      (e(wingspan["plabel"]), f'{num(wingspan["value"])} {e(wingspan["unit"])}'),
  ])}
</div>
"""
    return section(6, "straighten", "Size", f'{num(wingspan["value"])} {wingspan["unit"]}', body)


def section_range():
    countries = P("P9714")
    range_map = one("P181")
    iucn = one("P141")

    def note_of(st):
        bits = [q["value"] for q in st["quals"]
                if q["pid"] in ("P1264", "P706", "P131")]
        return ", ".join(bits)

    # No ranking here: the map is the range, and the list below it is complete.
    chips_html = "".join(
        chip(label_of(c), note=note_of(c) or None, link=wd(value_qid(c)), menu=True)
        for c in sorted(countries, key=lambda c: label_of(c))
    )
    body = f"""
<div class="cols-map">
  <figure class="range-map">
    {vmenu(range_map["plabel"])}
    <img src="{e(commons(range_map["value"], 1200))}" alt="{e(range_map["plabel"])}">
    <figcaption>{e(range_map["plabel"])}</figcaption>
  </figure>
  <div class="tile">
    {facts([
        (e(countries[0]["plabel"]), f'{len(countries)}'),
        (e(iucn["plabel"]), e(label_of(iucn))),
        (e(one("P6800")["plabel"]), f'<a href="{e(one("P6800")["value"])}">{e(one("P6800")["value"])}</a>'),
    ])}
  </div>
</div>
<div class="tile">
  <h3>{e(countries[0]["plabel"])} ({len(countries)}){vmenu(countries[0]["plabel"])}</h3>
  <div class="chips">{chips_html}</div>
</div>
"""
    return section(12, "map", "Range", f"{len(countries)}", body)


def section_symbol():
    represents = P("P1268")
    official = one("P2238")
    award = one("P166")
    body = f"""
<div class="tile">
  <h3>{e(represents[0]["plabel"])} ({len(represents)}){vmenu(represents[0]["plabel"])}</h3>
  {chips_of(represents)}
</div>
<div class="tile">
  {facts([
      (e(official["plabel"]), chip(label_of(official), img=official["vimg"], link=wd(value_qid(official)), menu=False)),
      (e(award["plabel"]), f'<a href="{wd(value_qid(award))}">{e(label_of(award))}</a>'
                           f' <span class="note">{e(year_str(qual(award, "P585") or ""))}</span>'),
  ])}
</div>
"""
    return section(6, "trophy", "As a symbol", None, body)


def section_media():
    photo = one("P18")
    range_map = one("P181")
    voice = one("P51")
    figures = "".join(
        f'<figure><a href="{e(commons(st["value"]))}">'
        f'<img src="{e(commons(st["value"], 640))}" alt="{e(st["plabel"])}" loading="lazy"></a>'
        f'{vmenu(st["plabel"])}'
        f'<figcaption><b>{e(st["plabel"])}</b><span>{e(qual(st, "P180") or "")}</span></figcaption></figure>'
        for st in (photo, range_map)
    )
    body = f"""
<div class="gallery">{figures}</div>
<div class="tile">
  <h3>{e(voice["plabel"])}{vmenu(voice["plabel"])}</h3>
  <audio controls preload="none" src="{e(commons(voice["value"]))}"></audio>
</div>
"""
    return section(6, "photo_library", "Pictures and voice", None, body)


def section_wikimedia():
    commons_cat = one("P373")
    commons_gal = one("P935")
    category = one("P910")
    source = one("P1343")
    project = one("P6104")
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
  </div>
  <div class="tile">
    {facts([
        (e(category["plabel"]), e(label_of(category))),
        (e(source["plabel"]), e(label_of(source))),
        (e(project["plabel"]), e(label_of(project))),
    ])}
  </div>
  <div class="tile">
    {more_button("dlg-ids", f"External identifiers ({len(ids)})", "tag")}
  </div>
</div>
"""
    return section(12, "sell", "On Wikimedia projects", None, body, tint="grey")


LEAD = {
    "text": "The golden eagle (Aquila chrysaetos) is a bird of prey living in "
            "the Northern Hemisphere. It is the most widely distributed "
            "species of eagle. Like all eagles, it belongs to the family "
            "Accipitridae. They are one of the best-known birds of prey in "
            "the Northern Hemisphere. These birds are dark brown, with "
            "lighter golden-brown plumage on their napes.",
    "title": "Golden eagle",
    "url": "https://en.wikipedia.org/wiki/Golden_eagle",
    "project": "English Wikipedia",
}

FOOTER = (
    'Prototype. Every label, number, date, image and link on this page comes '
    f'from the Wikidata item <a href="{wd(QID)}">{QID}</a>, as the qjson SPARQL '
    f'query returns it with the label language "{LANG}, mul, en" '
    f'(<code>prototype/v1/data/{CFG["fixture"]}</code>). The title is the '
    'language-independent label, which for a taxon is the scientific name. '
    'Pictures and the recording come from Wikimedia Commons. The lead '
    'paragraph is not Wikidata: it comes from the '
    '<a href="https://en.wikipedia.org/wiki/Golden_eagle">English Wikipedia</a> '
    'article and says so on the page. Section names and the words "See all" '
    'and "Add a name" are page chrome, not data. The Wikipedia header is a '
    'mock-up.'
)


def build():
    sections = [
        section_hero(),
        lead_section(LEAD["text"], LEAD["title"], LEAD["url"], LEAD["project"],
                     CFG["lead_action"], CFG["lead_missing"]),
        section_taxonomy(),
        section_conservation(),
        section_names(),
        section_body(),
        section_range(),
        section_symbol(),
        section_media(),
        section_wikimedia(),
    ]
    render_page(OUT, QID, LANG, sections, CFG["words"], FOOTER)


if __name__ == "__main__":
    build()
