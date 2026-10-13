"""The score primitive: review scores, one card each, with the logo of the
source (IMDb, Rotten Tomatoes, Metacritic, Letterboxd).

A casual reader looks for these first, and a logo is known at a glance in
every language, so a score is large and its source is a picture. The
qualifiers are the ones of the review score property (P444) itself.

localize: nothing of its own. The source and the method are items, the
count is an amount with its property, and the generic walk of localize.py
gives each its label or its number.
"""

from values import entity_id, year_of

SOURCE = "P447"  # review score by
METHOD = "P459"  # determination method or standard
COUNT = "P7887"  # number of reviews/ratings
DATE = "P585"  # point in time


def build(c, pid, sts):
    values = []
    for s in sts:
        q = {x["pid"]: x["value"] for x in s["qualifiers"]}
        v = {
            "statement": s["id"],
            "pid": pid,
            "string": s["value"],
            "provenance": c.provenance(s),
        }
        source = entity_id(q.get(SOURCE))
        if source:
            logo = c.c.get("logos", {}).get(source)
            v["source"] = {"item": source, "image": c.picture(logo, logo=True) if logo else None}
        if entity_id(q.get(METHOD)):
            v["method"] = {"item": entity_id(q[METHOD])}
        if q.get(COUNT):
            v["count"] = {"pid": COUNT, "amount": float(q[COUNT])}
        if q.get(DATE):
            v["year"] = year_of(q[DATE])
        values.append(v)
    # The most reviewed first: an order that is the same in every language.
    values.sort(key=lambda v: -(v.get("count") or {}).get("amount", 0))
    return {
        "primitive": "score",
        "pid": pid,
        "values": values,
        "priority": c.priority(pid, sts),
    }


def html(b, ui):
    cards = []
    for v in b["values"]:
        src = v.get("source") or {}
        image = src.get("image")
        if image:
            mark = (
                f'<img class="logo" src="{ui.FILEPATH}{ui.urllib.parse.quote(image["file"])}?width=240" '
                f'alt="{ui.e(src["display"]["text"])}" loading="lazy">'
            )
        else:
            mark = f'<span class="source">{ui.part(src["display"])}</span>' if src else ""
        method = f'<span class="method">{ui.part(v["method"]["display"])}</span>' if v.get("method") else ""
        count = (
            f'<span class="count" title="{ui.e(v["count"]["property"]["text"])}">{ui.e(v["count"]["amount_display"])}</span>'
            if v.get("count")
            else ""
        )
        year = f'<span class="year">{v["year"]}</span>' if v.get("year") else ""
        cards.append(
            # The score as Wikidata has it ("94%", "7.9/10"): the generic
            # display of this value would take its year.
            f'<div class="card">{mark}<b class="value">{ui.e(v["string"])}</b>'
            f'{method}<span class="meta">{count}{year}</span>'
            f'<span class="tools">{ui.prov(v.get("provenance"))}{ui.edit(v)}</span></div>'
        )
    return f'<div class="block scores">{"".join(cards)}</div>'
