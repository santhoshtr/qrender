#!/usr/bin/env python3
"""Build the static Nelson Mandela (Q8023) prototype page.

The first person archetype. A person is not a place: there is no map and no
outline, but there are dated spans -- marriages, party membership, prison,
office -- that run along one life, and there are 90 awards, which is more
values of one property than any page has held so far.

Usage: python3 prototype/v1/build_q8023.py
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from factoid_kit import *  # noqa: F403

QID = "Q8023"

# The same composition, two label languages. Only the data file and the
# chrome change; the page is built by the same code.
LANGUAGES = {
    "en": {
        "fixture": "q8023-en.sparql.json",
        "out": "q8023.html",
        "lead_action": "Edit",
        "words": {
            "wordmark": "WIKIPEDIA",
            "tagline": "The Free Encyclopedia",
            "search": "Search Wikipedia",
            "donate": "Donate",
            "talk": "Talk",
            "user": "Sthottingal",
        },
    },
    "tcy": {
        "fixture": "q8023-tcy.sparql.json",
        "out": "q8023-tcy.html",
        "lead_action": "Translate",
        "words": {
            "wordmark": "ವಿಕಿಪೀಡಿಯ",
            "tagline": '<img src="https://tcy.wikipedia.org/static/images/mobile/copyright/wikipedia-tagline-tcy.svg" alt="" height="14">',
            "search": "ನಾಡ್‍ಲೆ",
            "donate": "ದಾನೊ",
            "talk": "ಚರ್ಚೆ",
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
    """Mark a string that did not come back in the page language."""
    return (f'<span class="lang-tag">{e(value_lang)}</span>'
            if value_lang and value_lang != LANG else "")


# ---------------------------------------------------------------- sections

def section_profile():
    """The head of a person page: the portrait holds a quarter of the width,
    the name and the cards that a reader asks for first hold the rest."""
    portrait = one("P18")
    signature = one("P109")
    birth, death = one("P569"), one("P570")
    bplace, dplace = one("P19"), one("P20")
    father, mother = one("P22"), one("P25")
    citizen = one("P27")
    occupations = P("P106")
    languages = P("P1412")
    native = one("P103")
    religion = one("P140")
    gender = one("P21")
    birth_name = one("P1477")
    native_name = one("P1559")

    portrait_note = fmt_date(qual(portrait, "P585")) if qual(portrait, "P585") else ""
    signature_row = (
        f'<img class="signature" src="{e(commons(signature["value"], 400))}" '
        f'alt="{e(signature["plabel"])}">'
    )

    details = section(None, "person", "Basic details", None, f"""
<div class="tile">
  {facts([
      (e(birth["plabel"]), f'{fmt_date(birth["value"], birth["precision"])} '
                           f'<span class="note">{e(bplace["plabel"])}: '
                           f'<a href="{wd(value_qid(bplace))}">{e(label_of(bplace))}</a></span>'),
      (e(death["plabel"]), f'{fmt_date(death["value"], death["precision"])} '
                           f'<span class="note">{e(dplace["plabel"])}: '
                           f'<a href="{wd(value_qid(dplace))}">{e(label_of(dplace))}</a></span>'),
      (e(father["plabel"]), e(label_of(father))),
      (e(mother["plabel"]), e(label_of(mother))),
      (e(signature["plabel"]), signature_row),
  ])}
</div>
""")

    identity = section(None, "globe", "Citizenship and work", None, f"""
<div class="tile">
  {facts([
      (e(citizen["plabel"]), f'<a href="{wd(value_qid(citizen))}">{e(label_of(citizen))}</a>'),
      (e(occupations[0]["plabel"]), ", ".join(e(label_of(o)) for o in occupations)),
      (e(languages[0]["plabel"]), ", ".join(e(label_of(x)) for x in languages)),
      (e(native["plabel"]), e(label_of(native))),
      (e(religion["plabel"]), e(label_of(religion))),
      (e(gender["plabel"]), e(label_of(gender))),
  ])}
</div>
""")

    return f"""<section class="profile span-12">
  <figure class="profile-portrait">
    {vmenu(portrait["plabel"])}
    <img src="{e(commons(portrait["value"], 800))}" alt="{e(item()["label"])}">
    <figcaption>{e(portrait["plabel"])}{" · " + e(portrait_note) if portrait_note else ""}</figcaption>
  </figure>
  <div class="profile-body">
    <header class="profile-head">
      <h1>{e(item()["label"])} {lang_tag(item()["label_lang"])}</h1>
      <p class="profile-desc">{e(item()["description"])} {lang_tag(item()["description_lang"])}</p>
      <div class="chips">
        <a class="qid-badge" href="{wd(QID)}">{QID}</a>
        <span class="chip"><span class="note">{e(birth_name["plabel"])}</span>{e(birth_name["value"])}</span>
        <span class="chip"><span class="note">{e(native_name["plabel"])}</span>{e(native_name["value"])}</span>
      </div>
    </header>
    <div class="profile-cards">{details}{identity}</div>
  </div>
</section>"""


# The only text on this page that is not Wikidata. It shows what pulling a
# sister project in could look like. Three problems come with it, and none is
# solved here:
#   1. Choosing one quote out of many is editorial work that we are not
#      entitled to do.
#   2. Not every item has a Wikiquote page, so nothing general follows.
#   3. The quote is in English. A page in another language needs that
#      language's Wikiquote, which may not have the item at all.
# From the English Wikipedia article. The page says so,
# and on a page that is not in English the action on it is "Translate".
LEAD = {
    "text": "Nelson Rolihlahla Mandela was a South African anti-apartheid "
            "revolutionary, politician, and philanthropist, who served as "
            "President of South Africa from 1994 to 1999. He was the country's "
            "first black head of state and the first elected in a fully "
            "representative democratic election. His government focused on "
            "dismantling the legacy of apartheid by tackling institutionalised "
            "racism and fostering racial reconciliation. Ideologically an "
            "African nationalist and socialist, he served as President of the "
            "African National Congress (ANC) party from 1991 to 1997.",
    "title": "Nelson Mandela",
    "url": "https://en.wikipedia.org/wiki/Nelson_Mandela",
    "project": "English Wikipedia",
}


WIKIQUOTE = {
    "text": "When a man is denied the right to live the life he believes in, "
            "he has no choice but to become an outlaw.",
    "page": "https://en.wikiquote.org/wiki/Nelson_Mandela",
    "project": "Wikiquote",
    "logo": "https://commons.wikimedia.org/wiki/Special:FilePath/Wikiquote-logo.svg?width=60",
}


def section_quote():
    q = WIKIQUOTE
    return f"""<section class="quote-band span-12">
  <blockquote>
    <p>{e(q["text"])}</p>
    <footer>
      <img src="{e(q["logo"])}" alt="">
      <a href="{e(q["page"])}">{e(q["project"])}</a>
      <span class="note">from a sister project</span>
    </footer>
  </blockquote>
</section>"""


# ------------------------------------------------------------- the life line

def life_lanes():
    """One life, four properties that run along it as dated spans."""
    birth = year_of(one("P569")["value"])
    death = year_of(one("P570")["value"])
    span = death - birth

    def lane(pid, note_fn=None):
        rows = []
        for st in sorted(P(pid), key=lambda s: year_of(qual(s, "P580") or "") or 9999):
            start, end = year_of(qual(st, "P580")), year_of(qual(st, "P582"))
            if not start:
                continue
            end = end or death
            left = (start - birth) / span * 100
            width = max((end - start) / span * 100, 1.2)
            note = note_fn(st) if note_fn else None
            rows.append(
                f'<div class="band"><span><a href="{wd(value_qid(st))}">{e(label_of(st))}</a>'
                + (f' <span class="note">{e(note)}</span>' if note else "")
                + f'</span><span class="band-track"><span class="band-fill" '
                f'style="inset-inline-start: {left:.2f}%; inline-size: {width:.2f}%;" '
                f'title="{e(timespan(st))}"></span></span>'
                f'<span class="band-years">{e(timespan(st))}</span>{vmenu(label_of(st))}</div>'
            )
        return rows

    lanes = [
        ("P39", lambda s: qual(s, "P1365") and f'after {qual(s, "P1365")}'),
        ("P2632", lambda s: qual(s, "P1350") or (f'prisoner number {qual(s, "P14559")}'
                                                 if qual(s, "P14559") else None)),
        ("P102", None),
        ("P26", lambda s: qual(s, "P1534")),
    ]
    blocks = []
    for pid, note_fn in lanes:
        rows = lane(pid, note_fn)
        if not rows:
            continue
        blocks.append(f'<h3>{e(P(pid)[0]["plabel"])}{vmenu(P(pid)[0]["plabel"])}</h3>'
                      f'<div class="bands">{"".join(rows)}</div>')
    axis = (f'<div class="bands"><div class="band"><span></span>'
            f'<span class="band-axis"><span>{birth}</span><span>{death}</span></span>'
            f'<span></span></div></div>')
    return "".join(blocks) + axis


def section_life():
    birth, death = one("P569"), one("P570")
    bplace, dplace = one("P19"), one("P20")
    cause, manner = one("P509"), one("P1196")
    burial = one("P119")
    detention = P("P2632")
    prisoner_no = next((qual(d, "P14559") for d in detention if qual(d, "P14559")), None)

    born_row = ", ".join(
        e(q["value"]) for q in bplace["quals"] if q["pid"] in ("P131", "P17"))
    body = f"""
<div class="tile">
  {life_lanes()}
</div>
<div class="cols">
  <div class="tile">
    <h3>{e(birth["plabel"])}</h3>
    {facts([
      (e(birth["plabel"]), fmt_date(birth["value"], birth["precision"])),
      (e(bplace["plabel"]), f'<a href="{wd(value_qid(bplace))}">{e(label_of(bplace))}</a>'
                            + (f' <span class="note">{born_row}</span>' if born_row else "")),
      (e(one("P22")["plabel"]), e(label_of(one("P22")))),
      (e(one("P25")["plabel"]), e(label_of(one("P25")))),
    ])}
  </div>
  <div class="tile">
    <h3>{e(detention[0]["plabel"])}</h3>
    {facts([(e(d["plabel"]), f'<a href="{wd(value_qid(d))}">{e(label_of(d))}</a> '
                             f'<span class="note">{e(timespan(d))}</span>') for d in detention])}
    <div class="note">{e(next(q["plabel"] for d in detention for q in d["quals"] if q["pid"] == "P14559"))}: {e(prisoner_no)}</div>
  </div>
  <div class="tile">
    <h3>{e(death["plabel"])}</h3>
    {facts([
      (e(death["plabel"]), fmt_date(death["value"], death["precision"])),
      (e(dplace["plabel"]), e(label_of(dplace))),
      (e(cause["plabel"]), e(label_of(cause))),
      (e(manner["plabel"]), e(label_of(manner))),
      (e(burial["plabel"]), e(label_of(burial))),
    ])}
  </div>
</div>
"""
    return section(12, "event", "Life", f'{year_of(birth["value"])}–{year_of(death["value"])}', body)


def section_family():
    spouses = P("P26")
    children = P("P40")
    father, mother = one("P22"), one("P25")
    relative = one("P1038")

    spouse_tiles = "".join(
        thumb(label_of(s), s["vimg"],
              note=" · ".join(x for x in [timespan(s), qual(s, "P1534")] if x),
              link=wd(value_qid(s)))
        for s in sorted(spouses, key=lambda s: year_of(qual(s, "P580") or "") or 0)
    )
    child_tiles = "".join(
        thumb(label_of(c), c["vimg"], link=wd(value_qid(c)))
        for c in sorted(children, key=lambda c: label_of(c))
    )
    body = f"""
<div>
  <h3>{e(spouses[0]["plabel"])} ({len(spouses)}){vmenu(spouses[0]["plabel"])}</h3>
  <div class="thumbs">{spouse_tiles}</div>
</div>
<div>
  <h3>{e(children[0]["plabel"])} ({len(children)}){vmenu(children[0]["plabel"])}</h3>
  <div class="thumbs">{child_tiles}</div>
</div>
<div class="tile">
  {facts([
      (e(father["plabel"]), e(label_of(father))),
      (e(mother["plabel"]), e(label_of(mother))),
      (e(relative["plabel"]), f'{e(label_of(relative))} <span class="note">{e(qual(relative, "P1039") or "")}</span>'),
  ])}
</div>
"""
    return section(6, "group", "Family", None, body)


def section_career():
    occupations = P("P106")
    education = P("P69")
    positions = P("P39")
    parties = P("P102")
    member = P("P463")
    work = one("P800")
    languages = P("P1412")

    def position_row(st):
        bits = [timespan(st)]
        if qual(st, "P1545"):
            bits.append(f'no. {qual(st, "P1545")}')
        if qual(st, "P1365"):
            bits.append(f'{next(q["plabel"] for q in st["quals"] if q["pid"] == "P1365")}: {qual(st, "P1365")}')
        if qual(st, "P1366"):
            bits.append(f'{next(q["plabel"] for q in st["quals"] if q["pid"] == "P1366")}: {qual(st, "P1366")}')
        return " · ".join(b for b in bits if b)

    body = f"""
<div class="tile">
  <h3>{e(positions[0]["plabel"])}{vmenu(positions[0]["plabel"])}</h3>
  {facts([(f'<a href="{wd(value_qid(p))}">{e(label_of(p))}</a>', f'<span class="note">{e(position_row(p))}</span>')
          for p in sorted(positions, key=lambda p: year_of(qual(p, "P580") or "") or 9999)])}
</div>
<div class="tile">
  <h3>{e(occupations[0]["plabel"])}{vmenu(occupations[0]["plabel"])}</h3>
  {chips_of(occupations)}
</div>
<div class="tile">
  <h3>{e(education[0]["plabel"])}{vmenu(education[0]["plabel"])}</h3>
  {chips_of(education, note_fn=lambda s: qual(s, "P512"))}
</div>
<div class="tile">
  {facts([
      (e(parties[0]["plabel"]), " · ".join(
          f'{e(label_of(p))} <span class="note">{e(timespan(p))}</span>' for p in parties)),
      (e(member[0]["plabel"]), " · ".join(
          f'{e(label_of(m))} <span class="note">{e(timespan(m))}</span>' for m in member)),
      (e(work["plabel"]), e(label_of(work))),
      (e(languages[0]["plabel"]), " · ".join(e(label_of(x)) for x in languages)),
      (e(one("P937")["plabel"]), e(label_of(one("P937")))),
      (e(one("P3602")["plabel"]), e(label_of(one("P3602")))),
  ])}
</div>
"""
    return section(6, "school", "Career", None, body)


def section_awards():
    awards = P("P166")
    years = [year_of(qual(a, "P585")) for a in awards if qual(a, "P585")]

    # Feature the statement that Wikidata documents most: the one with the
    # most qualifiers. That is a property of the data, not our opinion.
    feature = max(awards, key=lambda a: len(a["quals"]))
    rationale = quals(feature, "P6208", lang="en") or quals(feature, "P6208")
    fbits = []
    for q in feature["quals"]:
        if q["pid"] in ("P585",):
            fbits.append(year_str(q["value"]))
        elif q["pid"] in ("P1706", "P2121"):
            fbits.append(f'{q["plabel"]}: {q["value"]}')
    feature_html = (
        f'<div class="tile feature-award">{vmenu(label_of(feature))}'
        + (f'<img src="{e(commons(feature["vimg"], 400))}" alt="" loading="lazy">'
           if feature["vimg"] else "")
        + f'<div><h3>{e(feature["plabel"])}</h3>'
        f'<div class="bignum" style="font-size: var( --font-size-xx-large );">'
        f'<a href="{wd(value_qid(feature))}">{e(label_of(feature))}</a></div>'
        f'<div class="note">{e(" · ".join(fbits))}</div>'
        + (f'<p class="note" style="margin-block: 8px 0;">{e(rationale[0])}</p>'
           if rationale else "")
        + "</div></div>"
    )

    # Six of the illustrated ones, then every award in one table. The row is
    # a window on the 89, not a list of its own, so the counts agree.
    illustrated = sorted((a for a in awards if a["vimg"] and a is not feature),
                         key=lambda a: year_of(qual(a, "P585") or "") or 9999)
    shown = illustrated[:6]
    strip = ('<div class="thumbs row-7">'
             + "".join(thumb(label_of(a), a["vimg"],
                             note=year_str(qual(a, "P585") or "") or None,
                             link=wd(value_qid(a))) for a in shown)
             + more_tile("dlg-awards", len(awards) - len(shown))
             + "</div>")

    rows = "".join(
        f'<tr><td>{e(year_str(qual(a, "P585") or "") or "—")}</td>'
        f'<td><a href="{wd(value_qid(a))}">{e(label_of(a))}</a></td>'
        f'<td>{e(" · ".join(q["plabel"] + ": " + q["value"] for q in a["quals"] if q["pid"] != "P585"))}</td></tr>'
        for a in sorted(awards, key=lambda a: year_of(qual(a, "P585") or "") or 9999)
    )
    add_dialog("dlg-awards", f'{awards[0]["plabel"]} ({len(awards)})',
               '<table class="vtable"><thead><tr><th>year</th><th>award</th>'
               f'<th>qualifiers</th></tr></thead><tbody>{rows}</tbody></table>')

    body = f"""
<div class="cols-map">
  {feature_html}
  <div class="tile">
    {facts([
      (e(one("P2354")["plabel"]), f'<a href="{wd(value_qid(one("P2354")))}">{e(label_of(one("P2354")))}</a>'),
      (e(one("P463")["plabel"]), " · ".join(e(label_of(m)) for m in P("P463"))),
    ])}
    {more_button("dlg-awards", f'See all {len(awards)}, {min(years)}–{max(years)}', "trophy")}
  </div>
</div>
<div>
  <h3>{e(awards[0]["plabel"])} ({len(awards)}){vmenu(awards[0]["plabel"])}</h3>
  {strip}
</div>
"""
    return section(12, "trophy", "Awards", f"{len(awards)} · {min(years)}–{max(years)}", body)


def section_names():
    rows = [
        ("P1477", one("P1477")["value"]),
        ("P1559", one("P1559")["value"]),
        ("P735", label_of(one("P735"))),
        ("P734", label_of(one("P734"))),
        ("P2572", one("P2572")["value"]),
    ]
    site = one("P856")
    body = f"""
<div class="tile">
  {facts([(e(one(pid)["plabel"]), e(value)) for pid, value in rows])}
</div>
<div class="tile">
  {facts([(e(site["plabel"]), f'<a href="{e(site["value"])}">{e(site["value"])}</a>')])}
</div>
"""
    return section(6, "article", "Names and identity", None, body)


def section_views():
    portrait = one("P18")
    signature = one("P109")
    plaques = P("P1801")
    figures = []
    for st in [portrait] + plaques:
        notes = []
        if qual(st, "P585"):
            notes.append(fmt_date(qual(st, "P585")))
        figures.append(
            f'<figure><a href="{e(commons(st["value"]))}">'
            f'<img src="{e(commons(st["value"], 640))}" alt="{e(st["plabel"])}" loading="lazy"></a>'
            f'{vmenu(st["plabel"])}'
            f'<figcaption><b>{e(st["plabel"])}</b><span>{e(" · ".join(notes))}</span></figcaption></figure>'
        )
    emblem = (f'<figure title="{e(signature["plabel"])}">'
              f'<img src="{e(commons(signature["value"], 300))}" alt="{e(signature["plabel"])}" loading="lazy">'
              f'{vmenu(signature["plabel"])}'
              f'<figcaption><b>{e(signature["plabel"])}</b></figcaption></figure>')
    body = f"""
<div class="gallery">{"".join(figures)}</div>
<div class="gallery emblems">{emblem}</div>
"""
    return section(6, "photo_library", "Pictures", f"{len(figures) + 1}", body)


def section_wikimedia():
    cats = [s for pid in ("P910",) for s in P(pid)]
    projects = P("P6104")
    focus = P("P5008")
    sources = P("P1343")
    collections = [s for pid in ("P6379", "P9493", "P10527") for s in P(pid)]
    commons_cat = one("P373")
    commons_gal = one("P935")
    creator = one("P1472")

    ids = [s for s in statements() if s["ptype"] == "ExternalId"]
    id_rows = "".join(
        f'<div><span class="k">{e(s["plabel"])}</span><span class="v">{e(s["value"])}</span></div>'
        for s in sorted(ids, key=lambda s: s["plabel"].lower())
    )
    add_dialog("dlg-ids", f"External identifiers ({len(ids)})",
               f'<div class="idlist">{id_rows}</div>')
    project_rows = "".join(
        f'<tr><td>{e(label_of(s))}</td></tr>' for s in sorted(projects, key=label_of))
    add_dialog("dlg-projects", f'{projects[0]["plabel"]} ({len(projects)})',
               f'<table class="vtable"><tbody>{project_rows}</tbody></table>')

    body = f"""
<div class="cols">
  <div class="tile">
    <h3>{e(commons_cat["plabel"])} · {e(commons_gal["plabel"])} · {e(creator["plabel"])}</h3>
    <div class="chips">
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/Category:{e(commons_cat["value"])}">{e(commons_cat["value"])}</a></span>
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/{e(commons_gal["value"])}">{e(commons_gal["value"])}</a></span>
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/Creator:{e(creator["value"])}">{e(creator["value"])}</a></span>
    </div>
    <h3 style="margin-block-start: 16px;">{e(cats[0]["plabel"])}</h3>
    {chips_of(cats, with_images=False)}
  </div>
  <div class="tile">
    <h3>{e(sources[0]["plabel"])}</h3>
    {chips_of(sources, with_images=False, note_fn=lambda s: qual(s, "P478") and f'volume {qual(s, "P478")}')}
    <h3 style="margin-block-start: 16px;">{e(collections[0]["plabel"])}</h3>
    {chips_of(collections, with_images=False, note_fn=lambda s: qual(s, "P1810"))}
  </div>
  <div class="tile">
    <h3>{e(focus[0]["plabel"])}</h3>
    {chips_of(focus, with_images=False)}
    <h3 style="margin-block-start: 16px;">{e(one("P7763")["plabel"])}</h3>
    <div class="note">{e(label_of(one("P7763")))} · {e(qual(one("P7763"), "P1001") or "")}</div>
  </div>
</div>
<div class="chips">
  {more_button("dlg-projects", f"{projects[0]['plabel']} ({len(projects)})", "communities")}
  {more_button("dlg-ids", f"External identifiers ({len(ids)})", "tag")}
</div>
"""
    return section(12, "sell", "On Wikimedia projects", None, body, tint="grey")


FOOTER = (
    'Prototype. Every label, number, date, image and link on this page comes '
    f'from the Wikidata item <a href="{wd(QID)}">{QID}</a>, as the qjson SPARQL '
    f'query returns it with the label language "{LANG}, mul, en" '
    f'(<code>prototype/v1/data/{CFG["fixture"]}</code>). '
    'Pictures come from Wikimedia Commons. The awards card shows the award '
    'statement that carries the most qualifiers, which is a property of the '
    'data, not a judgement of ours. Section names and the words "from", '
    '"until" and "See all" are page chrome, not data. The Wikipedia header is '
    'a mock-up. Two blocks are not Wikidata and say so on the page: the lead '
    'paragraph comes from the '
    '<a href="https://en.wikipedia.org/wiki/Nelson_Mandela">English Wikipedia</a> '
    'article, and the quotation comes from '
    '<a href="https://en.wikiquote.org/wiki/Nelson_Mandela">Wikiquote</a>, is '
    'marked as such on the page, and shows what pulling in a sister project '
    'could look like. Choosing one quote out of many is editorial work we are '
    'not entitled to do, not every item has a Wikiquote page, and the quote is '
    'in one language. None of that is solved here.'
)


def build():
    sections = [
        section_profile(),
        lead_section(LEAD["text"], LEAD["title"], LEAD["url"], LEAD["project"],
                     CFG["lead_action"]),
        section_quote(),
        section_life(),
        section_family(),
        section_career(),
        section_awards(),
        section_views(),
        section_names(),
        section_wikimedia(),
    ]
    render_page(OUT, QID, LANG, sections, CFG["words"], FOOTER)


if __name__ == "__main__":
    build()
