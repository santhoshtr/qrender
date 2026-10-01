#!/usr/bin/env python3
"""Build the static One Battle After Another (Q117085614) prototype page.

The film archetype. A film has no birth and no boundary; it has people who
made it, people who are in it, scores, awards, a release that happens country
by country, and a rating in every jurisdiction that classifies films. It also
has a hole where the plot should be, because Wikidata does not hold prose, so
the page asks for one instead of pretending it has one.

Usage: python3 prototype/v1/build_q117085614.py
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from factoid_kit import *  # noqa: F403

QID = "Q117085614"

# The same composition in two label languages. Wikidata has 53 sitelinks for
# this film and none of them is an Indic Wikipedia, so the Malayalam page has
# a Malayalam structure and an English article behind it.
LANGUAGES = {
    "en": {
        "fixture": "q117085614-en.sparql.json",
        "out": "q117085614.html",
        "lead_action": "Edit",
        "lead_missing": None,
        "words": {
            "wordmark": "WIKIPEDIA",
            "tagline": "The Free Encyclopedia",
            "search": "Search Wikipedia",
            "donate": "Donate",
            "talk": "Talk",
            "user": "Sthottingal",
        },
    },
    "ml": {
        "fixture": "q117085614-ml.sparql.json",
        "out": "q117085614-ml.html",
        "lead_action": "Translate",
        "lead_missing": "No Malayalam article exists for this film. The "
                        "paragraph below is the lead of the English article.",
        "words": {
            "wordmark": "വിക്കിപീഡിയ",
            "tagline": '<img src="https://ml.wikipedia.org/static/images/mobile/copyright/wikipedia-tagline-ml.svg" alt="" height="14">',
            "search": "തിരയൂ",
            "donate": "ധനസമാഹരണം",
            "talk": "സംവാദം",
            "user": "Sthottingal",
        },
    },
}

LANG = sys.argv[1] if len(sys.argv) > 1 else "en"
CFG = LANGUAGES[LANG]
OUT = ROOT / "prototype" / "v1" / CFG["out"]
FIXTURE = ROOT / "prototype" / "v1" / "data" / CFG["fixture"]
REFERENCES = ROOT / "prototype" / "v1" / "data" / "q117085614-refs.json"

load(FIXTURE)
load_reference_counts(REFERENCES)


def lang_tag(value_lang):
    """Mark a string that did not come back in the page language."""
    return (f'<span class="lang-tag">{e(value_lang)}</span>'
            if value_lang and value_lang != LANG else "")

# Every jurisdiction that classifies films uses its own property. The list is
# written out here so the page says which ones it shows.
RATING_PIDS = (
    "P1657", "P2629", "P1981", "P6452", "P2684", "P3156", "P2756", "P2758",
    "P2363", "P2637", "P3216", "P3306", "P3428", "P3650", "P3818", "P3834",
    "P4437", "P5150", "P5201", "P6658", "P7327", "P7573",
)


def fmt_duration(seconds, unit):
    """9691 second reads as a running time. The source value stays in the
    title attribute."""
    total = int(float(seconds))
    hours, rest = divmod(total, 3600)
    minutes, secs = divmod(rest, 60)
    parts = [f"{hours} h", f"{minutes} min"] + ([f"{secs} s"] if secs else [])
    return f'<span title="{num(seconds)} {e(unit)}">{" ".join(parts)}</span>'


def role_of(st):
    """The character an actor plays, named either way round in the data."""
    return qual(st, "P4633") or qual(st, "P453")


# ---------------------------------------------------------------- sections

def section_hero():
    marquee = one("P18")
    logo = one("P154")
    title = one("P1476")
    rating = one("P1657")
    duration = one("P2047")
    premiere = next((s for s in P("P577") if qual(s, "P1552") == "premiere"), None)
    genres = P("P136")

    chips = [f'<a class="qid-badge" href="{wd(QID)}">{QID}</a>']
    if rating:
        chips.append(f'<span class="hero-chip"><span class="lang">{e(rating["plabel"])}'
                     f'</span>{e(label_of(rating))}</span>')
    if duration:
        chips.append(f'<span class="hero-chip"><span class="lang">{e(duration["plabel"])}'
                     f'</span>{fmt_duration(duration["value"], duration["unit"])}</span>')
    if premiere:
        chips.append(f'<span class="hero-chip"><span class="lang">'
                     f'{e(qual(premiere, "P1552"))}</span>'
                     f'{fmt_date(premiere["value"], premiere["precision"])}</span>')
    genre_line = ", ".join(e(label_of(g)) for g in genres)

    return f"""<section class="hero film span-12">
  <img class="hero-banner" src="{e(commons(marquee["value"], 2000))}" alt="">
  <div class="hero-body">
    <div>
      <h1>{e(item()["label"])} {lang_tag(item()["label_lang"])}</h1>
      <p class="hero-desc">{e(item()["description"])} {lang_tag(item()["description_lang"])}</p>
      <div class="hero-meta">{"".join(chips)}</div>
      <p class="hero-genres">{genre_line}</p>
    </div>
    <div class="hero-logo">{vmenu(logo["plabel"])}
      <img src="{e(commons(logo["value"], 600))}" alt="{e(title["value"])}">
    </div>
  </div>
</section>"""


def section_plot():
    """Wikidata holds no prose. Say so, and ask for it."""
    return """<section class="invite span-12">
  <h2>Plot</h2>
  <p>Wikidata holds statements, not prose, so this page has no plot summary.
     A plot belongs in the Wikipedia article.</p>
  <button class="invite-button" type="button">Write the plot</button>
</section>"""


def section_reception():
    scores = P("P444")
    won = P("P166")
    nominated = P("P1411")

    def score_card(st):
        source = qual(st, "P447") or ""
        method = qual(st, "P459") or ""
        reviews = qual(st, "P7887")
        when = qual(st, "P585")
        note = " · ".join(x for x in [
            f'{next((q["plabel"] for q in st["quals"] if q["pid"] == "P7887"), "")}: {num(reviews)}' if reviews else "",
            year_str(when) if when else "",
        ] if x)
        return (f'<div class="score {next_tint()}">{vmenu(source or st["plabel"])}'
                f'<span class="score-source">{e(source)}</span>'
                f'<span class="score-value">{e(st["value"])}</span>'
                f'<span class="score-method">{e(method)}</span>'
                f'<span class="score-note">{e(note)}</span></div>')

    def award_rows(statements, people_pid, did, title, limit=6):
        def row(st):
            people = [q["value"] for q in st["quals"] if q["pid"] == people_pid]
            event = qual(st, "P805")
            note = " · ".join(x for x in [", ".join(people), event or ""] if x)
            return (f'<div><span class="k"><a href="{wd(value_qid(st))}">{e(label_of(st))}</a></span>'
                    f'<span class="v"><span class="note">{e(note)}</span></span>'
                    f'{vmenu(label_of(st))}</div>')

        ordered = sorted(statements, key=lambda s: label_of(s))
        block = f'<div class="facts">{"".join(row(s) for s in ordered[:limit])}</div>'
        if len(ordered) > limit:
            add_dialog(did, title, f'<div class="facts">{"".join(row(s) for s in ordered)}</div>')
            block += more_button(did, f"See all {len(ordered)}", "trophy")
        return block

    body = f"""
<div class="scores">{"".join(score_card(s) for s in sorted(scores, key=lambda s: qual(s, "P447") or ""))}</div>
<div class="cols">
  <div class="tile">
    <h3>{e(won[0]["plabel"])} ({len(won)}){vmenu(won[0]["plabel"])}</h3>
    {award_rows(won, "P1346", "dlg-won", f'{won[0]["plabel"]} ({len(won)})')}
  </div>
  <div class="tile">
    <h3>{e(nominated[0]["plabel"])} ({len(nominated)}){vmenu(nominated[0]["plabel"])}</h3>
    {award_rows(nominated, "P2453", "dlg-nominated", f'{nominated[0]["plabel"]} ({len(nominated)})')}
  </div>
</div>
"""
    return section(12, "trophy", "Reception", f"{len(won)} won · {len(nominated)} nominated", body)


def section_cast():
    cast = P("P161")
    # Wikidata does not say who is billed first. The count of references on a
    # statement is the nearest thing it offers: the six best referenced here
    # are exactly the six the article names.
    #
    # Ties must not break on the label. Sorting the rest alphabetically made
    # the strip start with a different actor in Malayalam than in English,
    # because the scripts sort differently. The item number is arbitrary but
    # it is the same in every language.
    def order_key(st):
        qid = value_qid(st) or ""
        number = int(qid[1:]) if qid[1:].isdigit() else 10 ** 12
        return (-refs(st), st["vimg"] is None, number)

    ordered = sorted(cast, key=order_key)
    shown = ordered[:6]
    strip = ('<div class="thumbs row-7 portraits">'
             + "".join(thumb(label_of(c), c["vimg"], note=role_of(c),
                             link=wd(value_qid(c))) for c in shown)
             + more_tile("dlg-cast", len(cast) - len(shown))
             + "</div>")
    add_dialog("dlg-cast", f'{cast[0]["plabel"]} ({len(cast)})',
               '<div class="thumbs portraits">'
               + "".join(thumb(label_of(c), c["vimg"], note=role_of(c),
                               link=wd(value_qid(c))) for c in ordered)
               + "</div>")
    body = f"""
<div>
  <h3>{e(cast[0]["plabel"])} ({len(cast)}) · {e(next(q["plabel"] for c in cast for q in c["quals"] if q["pid"] in ("P4633", "P453")))}{vmenu(cast[0]["plabel"])}</h3>
  <p class="order-note">In order of how many references support each
     statement, from {refs(ordered[0])} down to {refs(ordered[-1])}. Wikidata
     records no billing order.</p>
  {strip}
</div>
"""
    return section(12, "group", "Cast", f"{len(cast)}", body)


# The people who made it, and the companies behind it. One card each: a
# person who did three jobs is one card with three roles, not three rows.
CREW_PIDS = ("P57", "P58", "P162", "P86", "P344", "P1040", "P3092")
COMPANY_PIDS = ("P272", "P750")


def clubbed(pids, role_note=None):
    """Group statements of several properties by their value, so each value
    appears once and carries every property label that pointed at it."""
    people = {}
    for pid in pids:
        for st in P(pid):
            key = value_qid(st) or label_of(st)
            entry = people.setdefault(key, {
                "label": label_of(st), "img": st["vimg"],
                "qid": value_qid(st), "roles": [], "notes": [],
            })
            entry["img"] = entry["img"] or st["vimg"]
            role = st["plabel"]
            if role not in entry["roles"]:
                entry["roles"].append(role)
            if role_note:
                note = role_note(st)
                if note and note not in entry["notes"]:
                    entry["notes"].append(note)
    return list(people.values())


def crew_card(entry):
    note = ", ".join(entry["roles"])
    if entry["notes"]:
        note += " · " + ", ".join(entry["notes"])
    return thumb(entry["label"], entry["img"], note=note,
                 link=wd(entry["qid"]) if entry["qid"] else None)


def section_credits():
    crew = clubbed(CREW_PIDS, role_note=lambda st: qual(st, "P3831"))
    based_on = one("P144")
    after = one("P1877")
    director = one("P57")

    body = f"""
<div>
  <h3>{e(director["plabel"])}, {e(one("P58")["plabel"])}, {e(P("P162")[0]["plabel"])}{vmenu(director["plabel"])}</h3>
  <div class="thumbs portraits">{"".join(crew_card(c) for c in crew)}</div>
</div>
<div class="tile">
  {facts([
      (e(based_on["plabel"]), f'<a href="{wd(value_qid(based_on))}">{e(label_of(based_on))}</a>'),
      (e(after["plabel"]), chip(label_of(after), img=after["vimg"], link=wd(value_qid(after)), menu=False)),
  ])}
</div>
"""
    return section(12, "person", "Credits", label_of(director), body)


def section_production():
    companies = clubbed(COMPANY_PIDS, role_note=lambda st: qual(st, "P17"))
    country = one("P495")
    language = one("P364")
    formats = P("P3803")
    ratios = P("P2061")
    camera = one("P4082")
    colour = one("P462")
    reel = one("P2043")
    titles = P("P1638")

    ratio_line = ", ".join(
        f'{e(qual(r, "P1932") or label_of(r))}'
        + (f' <span class="note">{e(", ".join(q["value"] for q in r["quals"] if q["pid"] == "P518"))}</span>'
           if any(q["pid"] == "P518" for q in r["quals"]) else "")
        for r in ratios
    )
    body = f"""
<div class="cols-map">
  <div class="tile">
    {facts([
        (e(country["plabel"]), e(label_of(country))),
        (e(language["plabel"]), e(label_of(language))),
        (e(formats[0]["plabel"]), ", ".join(e(label_of(f)) for f in formats)),
        (e(ratios[0]["plabel"]), ratio_line),
        (e(camera["plabel"]), f'{e(label_of(camera))} <span class="note">'
                              f'{e(next(q["plabel"] for q in camera["quals"] if q["pid"] == "P127"))}: '
                              f'{e(qual(camera, "P127"))}</span>'),
        (e(colour["plabel"]), e(label_of(colour))),
        (e(reel["plabel"]), f'{num(reel["value"])} {e(reel["unit"])} '
                            f'<span class="note">{e(qual(reel, "P518") or "")}</span>'),
        (e(titles[0]["plabel"]), ", ".join(e(t["value"]) for t in titles)),
    ])}
  </div>
  <div>
    <h3>{e(one("P272")["plabel"])}, {e(P("P750")[0]["plabel"])}{vmenu(one("P272")["plabel"])}</h3>
    <div class="thumbs">{"".join(crew_card(c) for c in companies)}</div>
  </div>
</div>
"""
    return section(12, "photo_library", "Production", None, body)


def section_money():
    budget = one("P2130")
    gross = one("P2142")
    top = max(float(budget["value"]), float(gross["value"]))
    bars = "".join(
        f'<div class="amount-row">'
        f'<span class="amount-label">{e(st["plabel"])}</span>'
        f'<span class="amount-track"><span class="amount-fill" '
        f'style="inline-size: {float(st["value"]) / top * 100:.1f}%"></span></span>'
        f'<span class="amount-value">{num(st["value"])}</span>'
        f'{vmenu(st["plabel"])}</div>'
        for st in (budget, gross)
    )
    dates = sorted(P("P577"), key=lambda s: s["value"])
    rows = []
    for st in dates:
        places = [q["value"] for q in st["quals"] if q["pid"] == "P291"]
        characteristic = qual(st, "P1552")
        rows.append(
            f'<div class="event"><span class="event-year">'
            f'{fmt_date(st["value"], st["precision"])}</span>'
            f'<span>{e(characteristic or "")}'
            + (f'<span class="note"> {e(", ".join(places))}</span>' if places else "")
            + f'</span>{vmenu(st["plabel"])}</div>'
        )
    body = f"""
<div class="cols-map">
  <div class="tile">
    <h3>{e(budget["plabel"])} · {e(gross["plabel"])} <span class="note">{e(gross["unit"])}</span>{vmenu(gross["plabel"])}</h3>
    <div class="amounts">{bars}</div>
  </div>
  <div class="tile">
    <h3>{e(dates[0]["plabel"])} ({len(dates)}){vmenu(dates[0]["plabel"])}</h3>
    <div class="events">{"".join(rows)}</div>
  </div>
</div>
"""
    return section(12, "policy", "Money and release", None, body)


def section_ratings():
    rated = [s for pid in RATING_PIDS for s in P(pid)]
    rated.sort(key=lambda s: s["plabel"])

    def badge(st):
        pic = (f'<img src="{e(commons(st["vimg"], 200))}" alt="">'
               if st["vimg"] else f'<span class="badge-text">{e(label_of(st))}</span>')
        descriptors = [q["value"] for q in st["quals"] if q["pid"] == "P7367"]
        return (f'<div class="badge">{vmenu(st["plabel"])}{pic}'
                f'<span class="badge-name">{e(st["plabel"])}</span>'
                # With a symbol the words repeat it, so show them only without.
                + (f'<span class="note">{e(label_of(st))}</span>' if st["vimg"] else "")
                + (f'<span class="note">{e(", ".join(descriptors))}</span>' if descriptors else "")
                + "</div>")

    shown, hidden = rated[:8], rated[8:]
    rows = "".join(
        f'<tr><td>{e(s["plabel"])}</td><td>{e(label_of(s))}</td>'
        f'<td>{e(", ".join(q["value"] for q in s["quals"] if q["pid"] == "P7367"))}</td>'
        f'<td>{e(qual(s, "P2676") or "")}</td></tr>'
        for s in rated
    )
    add_dialog("dlg-ratings", f"Film ratings ({len(rated)})",
               '<table class="vtable"><thead><tr><th>system</th><th>rating</th>'
               '<th>content descriptor</th><th>certificate</th></tr></thead>'
               f'<tbody>{rows}</tbody></table>')
    body = f"""
<div class="badges">{"".join(badge(s) for s in shown)}
  {more_tile("dlg-ratings", len(hidden)) if hidden else ""}</div>
"""
    return section(12, "policy", "Ratings by country", f"{len(rated)} systems", body)


def section_places():
    locations = P("P915")
    periods = P("P2408")
    soundtrack = one("P406")
    composer = one("P86")
    quotes = one("P6166")
    event = one("P793")

    loc_strip = thumb_row(sorted(locations, key=lambda s: label_of(s)),
                          "dlg-locations", f'{locations[0]["plabel"]} ({len(locations)})',
                          note_fn=lambda s: None, shown=6)
    body = f"""
<div>
  <h3>{e(locations[0]["plabel"])} ({len(locations)}){vmenu(locations[0]["plabel"])}</h3>
  {loc_strip}
</div>
<div class="tile">
  {facts([
      (e(periods[0]["plabel"]), ", ".join(e(label_of(p)) for p in periods)),
      (e(soundtrack["plabel"]), e(label_of(soundtrack)) + f' <span class="note">{e(composer["plabel"])}: {e(label_of(composer))}</span>'),
      (e(quotes["plabel"]), e(label_of(quotes))),
      (e(event["plabel"]), f'{e(label_of(event))} <span class="note">'
                           f'{e(qual(event, "P276") or "")} · {e(year_str(qual(event, "P585") or ""))}</span>'),
  ])}
</div>
"""
    return section(12, "map", "Places and sound", None, body)


def section_wikimedia():
    commons_cat = one("P373")
    site = one("P856")
    described = P("P973")
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
    <h3>{e(commons_cat["plabel"])}</h3>
    <div class="chips">
      <span class="chip"><a href="https://commons.wikimedia.org/wiki/Category:{e(commons_cat["value"])}">{e(commons_cat["value"])}</a></span>
    </div>
  </div>
  <div class="tile">
    {facts([
        (e(site["plabel"]), f'<a href="{e(site["value"])}">{e(qual(site, "P1476") or site["value"])}</a>'),
    ] + [(e(d["plabel"]), f'<a href="{e(d["value"])}">{e(d["value"])}</a>') for d in described])}
  </div>
  <div class="tile">
    {more_button("dlg-ids", f"External identifiers ({len(ids)})", "tag")}
  </div>
</div>
"""
    return section(12, "sell", "On Wikimedia projects", None, body, tint="grey")


LEAD = {
    "text": "One Battle After Another is a 2025 American action-comedy "
            "thriller film written, directed, and produced by Paul Thomas "
            "Anderson. Inspired by the 1990 novel Vineland by Thomas Pynchon, "
            "the film's ensemble cast includes Leonardo DiCaprio, Sean Penn, "
            "Benicio del Toro, Regina Hall, Teyana Taylor, and Chase Infiniti, "
            "in her film debut. It follows a washed-up former revolutionary "
            "(DiCaprio) who is brought back into conflict when his enemy "
            "(Penn) resurfaces after sixteen years.",
    "title": "One Battle After Another",
    "url": "https://en.wikipedia.org/wiki/One_Battle_After_Another",
    "project": "English Wikipedia",
}

FOOTER = (
    'Prototype. Every label, number, date, image and link on this page comes '
    f'from the Wikidata item <a href="{wd(QID)}">{QID}</a>, as the qjson SPARQL '
    f'query returns it with the label language "{LANG}, mul, en" '
    f'(<code>prototype/v1/data/{CFG["fixture"]}</code>). '
    'The running time is the duration statement of 9691 second, written as '
    'hours and minutes. Pictures come from Wikimedia Commons. The lead '
    'paragraph is not Wikidata: it comes from the '
    '<a href="https://en.wikipedia.org/wiki/One_Battle_After_Another">English '
    'Wikipedia</a> article and says so on the page. There is no plot summary '
    'because Wikidata holds statements, not prose. Section names and the words '
    '"See all" and "Write the plot" are page chrome, not data. The Wikipedia '
    'header is a mock-up. The cast strip is ordered by the number of '
    'references on each statement, which the qjson query does not fetch: that '
    'count comes from a second query, in '
    '<code>prototype/v1/data/q117085614-refs.json</code>.'
)


def build():
    sections = [
        section_hero(),
        lead_section(LEAD["text"], LEAD["title"], LEAD["url"], LEAD["project"],
                     CFG["lead_action"], CFG["lead_missing"]),
        section_plot(),
        section_reception(),
        section_cast(),
        section_credits(),
        section_production(),
        section_money(),
        section_ratings(),
        section_places(),
        section_wikimedia(),
    ]
    render_page(OUT, QID, LANG, sections, CFG["words"], FOOTER)


if __name__ == "__main__":
    build()
