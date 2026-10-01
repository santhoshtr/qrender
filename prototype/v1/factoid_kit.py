"""Shared parts of the static prototype pages.

Data loading, value formatting, the small set of page components and the page
frame live here. What each item's page looks like -- which sections it has and
what goes in them -- lives in its own build script, because that is the part we
are still designing.
"""

import html
import json
import math
import re
from pathlib import Path

# The repository root: this file is in prototype/v1/.
ROOT = Path(__file__).resolve().parents[2]
ICONS = ROOT / "qrender" / "assets" / "icons"

_STATE = {"item": {}, "statements": [], "refs": {}}

# ---------------------------------------------------------------- data layer

def local_name(uri):
    return uri.rsplit("/", 1)[-1]


def load(fixture):
    """Read one WDQS result file and group its rows by statement."""
    rows = json.loads(Path(fixture).read_text())["results"]["bindings"]
    by_st, order = {}, []
    for r in rows:
        st = r["statement"]["value"]
        if st not in by_st:
            by_st[st] = {
                "id": st,
                "pid": local_name(r["property"]["value"]),
                "plabel": r.get("propertyLabel", {}).get("value", ""),
                "plabel_lang": r.get("propertyLabel", {}).get("xml:lang"),
                "ptype": r["propertyType"]["value"].rsplit("#", 1)[-1],
                "value": r.get("statementValue", {}).get("value"),
                "vlang": r.get("statementValue", {}).get("xml:lang"),
                "vlabel": r.get("statementValueLabel", {}).get("value"),
                "vlabel_lang": r.get("statementValueLabel", {}).get("xml:lang"),
                "vimg": r.get("statementValueImage", {}).get("value"),
                "unit": r.get("unitOfMeasureLabel", {}).get("value"),
                "unit_lang": r.get("unitOfMeasureLabel", {}).get("xml:lang"),
                "precision": r.get("timePrecision", {}).get("value"),
                "rank": r.get("statementRankLabel", {}).get("value", "Normal"),
                "quals": [],
            }
            order.append(st)
        if "qualifierProperty" in r:
            qv = r.get("qualifierValue", {})
            qual = {
                "pid": local_name(r["qualifierProperty"]["value"]),
                "plabel": r.get("qualifierPropertyLabel", {}).get("value", ""),
                "plabel_lang": r.get("qualifierPropertyLabel", {}).get("xml:lang"),
                "value": r.get("qualifierValueLabel", {}).get("value") or qv.get("value"),
                "lang": qv.get("xml:lang"),
                "label_lang": r.get("qualifierValueLabel", {}).get("xml:lang"),
                "qid": local_name(qv["value"]) if qv.get("value", "").startswith(
                    "http://www.wikidata.org/entity/Q") else None,
            }
            if qual not in by_st[st]["quals"]:
                by_st[st]["quals"].append(qual)
    first = rows[0]
    _STATE["item"] = {
        "label": first["itemLabel"]["value"],
        "label_lang": first["itemLabel"].get("xml:lang", ""),
        "description": first.get("itemDescription", {}).get("value", ""),
        "description_lang": first.get("itemDescription", {}).get("xml:lang", ""),
    }
    _STATE["statements"] = [by_st[s] for s in order]


def load_reference_counts(path):
    """How many references support each statement.

    The qjson query does not fetch this, so it arrives as a side table from a
    second query. Wikidata has no field that says which value matters most;
    the number of references an editor bothered to add is the closest signal
    the data offers, and it is the only one we use for ordering."""
    rows = json.loads(Path(path).read_text())["results"]["bindings"]
    _STATE["refs"] = {r["statement"]["value"]: int(r["refs"]["value"]) for r in rows}


def refs(st):
    return _STATE["refs"].get(st["id"], 0)


def item():
    return _STATE["item"]


def statements():
    return _STATE["statements"]


def P(pid, deprecated=False):
    """Statements for one property. Deprecated rank is dropped by default,
    the same rule the qjson transform uses."""
    out = [s for s in _STATE["statements"] if s["pid"] == pid]
    if not deprecated:
        out = [s for s in out if s["rank"] != "Deprecated"]
    return out


def one(pid):
    st = P(pid)
    if not st:
        return None
    for s in st:
        if s["rank"] == "Preferred":
            return s
    return st[0]


def qual(st, pid):
    for q in st["quals"]:
        if q["pid"] == pid:
            return q["value"]
    return None


def quals(st, pid, lang=None):
    out = [q for q in st["quals"] if q["pid"] == pid]
    if lang:
        out = [q for q in out if q["lang"] == lang] or out
    return [q["value"] for q in out]


def label_of(st):
    if st["vlabel"]:
        return st["vlabel"]
    if st["value"] and st["value"].startswith("http://www.wikidata.org/entity/"):
        return local_name(st["value"])
    return st["value"] or ""


def value_qid(st):
    v = st["value"] or ""
    return local_name(v) if v.startswith("http://www.wikidata.org/entity/Q") else None

# ---------------------------------------------------------------- formatting

def e(text):
    return html.escape(str(text), quote=True)


def year_of(iso):
    if not iso:
        return None
    m = re.match(r"^(-?)0*(\d+)-", iso)
    if not m:
        return None
    y = int(m.group(2))
    return -y if m.group(1) else y


def fmt_date(iso, precision=None):
    """Show a time value at the precision the data states."""
    y = year_of(iso)
    if y is None:
        return iso
    p = int(precision) if precision else 11
    if p <= 6:
        return f"{abs(y) // 1000 + 1}. millennium" + (" BC" if y < 0 else "")
    if p == 7:
        century = (abs(y) - 1) // 100 + 1 if y > 0 else (abs(y) - 1) // 100 + 1
        suffix = {1: "st", 2: "nd", 3: "rd"}.get(century % 10 if century % 100 not in (11, 12, 13) else 0, "th")
        return f"{century}{suffix} century" + (" BC" if y < 0 else "")
    if p <= 9:
        return f"{abs(y)} BC" if y < 0 else str(y)
    if p == 10:
        return iso[:7] if y > 0 else str(y)
    return iso.split("T")[0] if y > 0 else str(y)


def year_str(iso):
    y = year_of(iso)
    if y is None:
        return ""
    return f"{abs(y)} BC" if y < 0 else str(y)


def timespan(st, prefix=True):
    """'1977 – 1995', 'from 2026', 'until 1859' from P580 / P582.
    A span inside one year keeps its full dates, so it does not read as
    a whole year."""
    start, end = qual(st, "P580"), qual(st, "P582")
    if start and end:
        if year_of(start) == year_of(end):
            return f'{start.split("T")[0]} – {end.split("T")[0]}'
        return f"{year_str(start)} – {year_str(end)}"
    if start:
        return ("from " if prefix else "") + year_str(start)
    if end:
        return ("until " if prefix else "") + year_str(end)
    return ""


def num(raw):
    try:
        f = float(raw)
    except (TypeError, ValueError):
        return str(raw)
    if f == int(f):
        return f"{int(f):,}"
    return f"{f:,.10g}"


def commons(url, width=None):
    """Commons FilePath URL, optionally as a thumbnail."""
    u = url.replace("http://commons.wikimedia.org", "https://commons.wikimedia.org")
    return f"{u}?width={width}" if width else u


def wd(qid):
    return f"https://www.wikidata.org/wiki/{qid}"

# ---------------------------------------------------------------- components

USED_ICONS = []


CHROME_ICON_NAMES = ("search", "menu", "more-vert", "edit", "add-photo")


def icon(name, cls="icon"):
    if name not in USED_ICONS and name not in CHROME_ICON_NAMES:
        USED_ICONS.append(name)
    return f'<svg class="{cls}" aria-hidden="true"><use href="#i-{name}"></use></svg>'


CHROME_SYMBOLS = (
    '<symbol id="i-search" viewBox="0 -960 960 960"><path d="M784-120 532-372q-30 24-69 38t-83 14q-109 '
    '0-184.5-75.5T120-580q0-109 75.5-184.5T380-840q109 0 184.5 75.5T640-580q0 44-14 83t-38 69l252 252-56 '
    '56ZM380-400q75 0 127.5-52.5T560-580q0-75-52.5-127.5T380-760q-75 0-127.5 52.5T200-580q0 75 52.5 127.5T380-400Z"/></symbol>'
    '<symbol id="i-menu" viewBox="0 -960 960 960"><path d="M120-240v-80h720v80H120Zm0-200v-80h720v80H120Zm0-200v-80h720v80H120Z"/></symbol>'
    '<symbol id="i-more-vert" viewBox="0 -960 960 960"><path d="M480-160q-33 0-56.5-23.5T400-240q0-33 23.5-56.5T480-320q33 0 56.5 23.5T560-240q0 33-23.5 56.5T480-160Zm0-240q-33 0-56.5-23.5T400-480q0-33 23.5-56.5T480-560q33 0 56.5 23.5T560-480q0 33-23.5 56.5T480-400Zm0-240q-33 0-56.5-23.5T400-720q0-33 23.5-56.5T480-800q33 0 56.5 23.5T560-720q0 33-23.5 56.5T480-640Z"/></symbol>'
    '<symbol id="i-add-photo" viewBox="0 -960 960 960"><path d="M440-440ZM120-120q-33 0-56.5-23.5T40-200v-480q0-33 23.5-56.5T120-760h126l74-80h240v80H355l-73 80H120v480h640v-360h80v360q0 33-23.5 56.5T760-120H120Zm640-560v-80h-80v-80h80v-80h80v80h80v80h-80v80h-80ZM440-260q75 0 127.5-52.5T620-440q0-75-52.5-127.5T440-620q-75 0-127.5 52.5T260-440q0 75 52.5 127.5T440-260Zm0-80q-42 0-71-29t-29-71q0-42 29-71t71-29q42 0 71 29t29 71q0 42-29 71t-71 29Z"/></symbol>'
    '<symbol id="i-edit" viewBox="0 -960 960 960"><path d="M200-200h57l391-391-57-57-391 391v57Zm-80 80v-170l528-527q12-11 26.5-17t30.5-6q16 0 31 6t26 18l55 56q12 11 17.5 26t5.5 30q0 16-5.5 30.5T817-647L290-120H120Zm640-584-56-56 56 56Zm-141 85-28-29 57 57-29-28Z"/></symbol>'
)


def sprite():
    parts = [ICONS.joinpath(f"{n}.svg").read_text().strip() for n in USED_ICONS]
    return ('<svg xmlns="http://www.w3.org/2000/svg" style="display:none">'
            + "".join(parts) + CHROME_SYMBOLS + "</svg>")


DIALOGS = []


def more_button(did, text, icon_name="info"):
    return f'<button class="more" command="show-modal" commandfor="{did}">{icon(icon_name)}{e(text)}</button>'


def add_dialog(did, title, body):
    DIALOGS.append(
        f'<dialog id="{did}" closedby="any" aria-labelledby="{did}-t">'
        f'<div class="dialog-head"><h2 id="{did}-t">{e(title)}</h2>'
        f'<button class="close" command="close" commandfor="{did}" aria-label="Close">&times;</button></div>'
        f'<div class="dialog-body">{body}</div></dialog>'
    )


def chip(label, img=None, note=None, lang=None, link=None, menu=True):
    inner = ""
    if img:
        inner += f'<img src="{e(commons(img, 48))}" alt="" loading="lazy">'
    text = e(label)
    if link:
        text = f'<a href="{e(link)}">{text}</a>'
    inner += f"<span>{text}</span>"
    if lang:
        inner += f'<span class="note">{e(lang)}</span>'
    if note:
        inner += f'<span class="note">{e(note)}</span>'
    if menu:
        inner += vmenu(label)
    return f'<span class="chip">{inner}</span>'


def chips_of(statements, note_fn=None, with_images=True, link=True):
    out = []
    for st in statements:
        note = note_fn(st) if note_fn else timespan(st)
        out.append(chip(
            label_of(st),
            img=st["vimg"] if with_images else None,
            note=note or None,
            link=wd(value_qid(st)) if link and value_qid(st) else None,
        ))
    return '<div class="chips">' + "".join(out) + "</div>"


def thumb(title, img, note=None, link=None):
    """A picture tile. Name and note each keep to one line, so the full
    value also goes in a title attribute. A value with no picture asks for
    one instead of showing an empty plate."""
    cls = "thumb" if img else "thumb no-image"
    pic = (f'<img src="{e(commons(img, 400))}" alt="" loading="lazy">' if img else
           f'<button class="add-image" type="button">{icon("add-photo")}Add image</button>')
    name = e(title)
    if link:
        name = f'<a href="{e(link)}">{name}</a>'
    note_html = f'<span class="note" title="{e(note)}">{e(note)}</span>' if note else ""
    return (f'<div class="{cls}">{pic}{vmenu(title)}<div class="thumb-body">'
            f'<span class="thumb-title" title="{e(title)}">{name}</span>'
            f'{note_html}</div></div>')


def facts(rows):
    body = "".join(
        f'<div><span class="k">{k}</span><span class="v">{v}</span>{vmenu(k)}</div>'
        for k, v in rows if v
    )
    return f'<div class="facts">{body}</div>'


# Tints carry no meaning. Sections and stat tiles take them in page order,
# so the page looks rich without claiming that a colour means something.
PALETTE = ("blue", "pink", "green", "purple", "orange", "lime", "yellow")
TINT = {"n": 0}


def next_tint():
    tint = PALETTE[TINT["n"] % len(PALETTE)]
    TINT["n"] += 1
    return tint


def vmenu(label):
    """The action affordance on one statement. Every trigger opens the same
    popover; the browser anchors it to the button that opened it."""
    name = re.sub(r"<[^>]+>", "", str(label)).strip()
    return (f'<button class="value-action" popovertarget="value-menu" '
            f'aria-label="Actions on {e(name)}">{icon("more-vert")}</button>')


VALUE_MENU = ('<div id="value-menu" popover class="card-menu">'
              '<button class="card-menu-item" type="button">'
              + icon("edit") + "Edit</button></div>")


def section(span, icon_name, title, count, body, tint=None):
    """A card. span is the column count on the page grid, or None when the
    card sits inside a layout of its own."""
    tint = tint or next_tint()
    head = (
        f'<div class="panel-head"><span class="panel-icon">{icon(icon_name)}</span>'
        f'<h2>{e(title)}</h2><span class="panel-actions">'
        + (f'<span class="panel-count">{e(count)}</span>' if count else "")
        + "</span></div>"
    )
    span_class = f" span-{span}" if span else ""
    return f'<section class="panel {tint}{span_class}">{head}{body}</section>'


def stat(icon_name, label, value, unit=None, note=None, tight=False):
    unit_html = f'<span class="stat-unit"> {e(unit)}</span>' if unit else ""
    note_html = f'<span class="stat-note">{e(note)}</span>' if note else ""
    cls = "stat-value tight" if tight else "stat-value"
    return (
        f'<div class="stat {next_tint()}">{vmenu(label)}'
        f'<span class="stat-label">{icon(icon_name)}{e(label)}</span>'
        f'<span class="{cls}">{e(value)}{unit_html}</span>{note_html}</div>'
    )


def more_tile(did, hidden):
    return (f'<button class="more-tile" command="show-modal" commandfor="{did}">'
            f'<b>+{hidden}</b><span>more</span></button>')


def thumb_row(statements, did, dialog_title, note_fn=None, shown=6,
              portraits=False, wide=False):
    """One row of picture tiles. The rest of the values open in a dialog."""
    def tile(st):
        return thumb(label_of(st), st["vimg"],
                     note=(note_fn(st) if note_fn else timespan(st)) or None,
                     link=wd(value_qid(st)) if value_qid(st) else None)

    shape = " portraits" if portraits else (" wide-tiles" if wide else "")
    row = "".join(tile(s) for s in statements[:shown])
    hidden = len(statements) - shown
    if hidden > 0:
        row += more_tile(did, hidden)
        add_dialog(did, dialog_title,
                   f'<div class="thumbs{shape}">'
                   + "".join(tile(s) for s in statements) + "</div>")
    cols = " row-5" if wide else " row-7"
    return f'<div class="thumbs{cols}{shape}">{row}</div>'


# ------------------------------------------------------------------ the map

MAP_W, MAP_H, MAP_Z = 800, 460, 12


def mercator(lat, lon, zoom):
    n = 256 * 2 ** zoom
    x = (lon + 180.0) / 360.0 * n
    s = math.sin(math.radians(lat))
    y = (0.5 - math.log((1 + s) / (1 - s)) / (4 * math.pi)) * n
    return x, y


def parse_point(literal):
    m = re.match(r"Point\(([-\d.]+) ([-\d.]+)\)", literal)
    return (float(m.group(2)), float(m.group(1))) if m else None


# ------------------------------------------------------------------- chrome

WIKIPEDIA_LOGO = "https://commons.wikimedia.org/wiki/Special:FilePath/Wikipedia-logo-v2.svg?width=72"


def chrome(words):
    return f"""<header class="chrome">
  <div class="chrome-inner">
    <button class="chrome-burger" aria-label="Main menu">{icon("menu")}</button>
    <a class="chrome-logo" href="#">
      <img src="{WIKIPEDIA_LOGO}" alt="">
      <span><b>{e(words["wordmark"])}</b><small>{words["tagline"]}</small></span>
    </a>
    <form class="chrome-search" role="search" onsubmit="return false">
      {icon("search")}
      <input type="search" name="search" placeholder="{e(words["search"])}" aria-label="{e(words["search"])}">
    </form>
    <nav class="chrome-links">
      <span>{e(words["donate"])}</span><span>{e(words["talk"])}</span>
      <span class="chrome-user">{icon("person")}{e(words["user"])}</span>
    </nav>
  </div>
</header>"""


FALLBACK_JS = """
// Progressive enhancement only: opens the "see all" dialogs in browsers
// without the invoker commands API. The page content does not need it.
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


def lead_section(text, page_title, page_url, project, action="Edit",
                 missing_note=None):
    """The lead paragraph of the Wikipedia article. Not Wikidata: it carries
    its source and an action of its own. Where the page is not in English the
    action is "Translate", because the paragraph is not in the page language.
    missing_note says that the article does not exist in this language."""
    action_icon = "edit" if action == "Edit" else "language"
    return f"""<section class="lead span-12">
  <button class="value-action" popovertarget="lead-menu"
          aria-label="Actions on the lead section">{icon("more-vert")}</button>
  <div id="lead-menu" popover class="card-menu">
    <button class="card-menu-item" type="button">{icon(action_icon)}{e(action)}</button>
  </div>
  {f'<p class="lead-missing">{e(missing_note)}</p>' if missing_note else ""}
  <p>{e(text)}</p>
  <footer>
    <img src="{WIKIPEDIA_LOGO}" alt="">
    <a href="{e(page_url)}">{e(project)}</a>
    <span class="note">{e(page_title)} · lead section, not from Wikidata</span>
  </footer>
</section>"""


def render_page(out, qid, lang, sections, words, footer_note):
    page = f"""<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{e(item()["label"])} ({qid}) — prototype</title>
<link rel="stylesheet" href="codex-tokens-light.css">
<link rel="stylesheet" href="prototype.css">
</head>
<body>
{chrome(words)}
<main class="page">
{"".join(sections)}
<footer class="provenance">{footer_note}</footer>
</main>
{VALUE_MENU}
{"".join(DIALOGS)}
{sprite()}
<script>{FALLBACK_JS}</script>
</body>
</html>
"""
    Path(out).write_text(page)
    print(f"wrote {out} ({len(page):,} bytes)")
