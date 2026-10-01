"""Wikidata values as identifiers and raw values: the helpers that the
composer and the primitives share. No labels, no text."""

import re
import urllib.parse

ENTITY = "http://www.wikidata.org/entity/"


def entity_id(value):
    if (
        isinstance(value, str)
        and value.startswith(ENTITY)
        and re.fullmatch(r"[PQ]\d+", value[len(ENTITY) :])
    ):
        return value[len(ENTITY) :]
    return None


def year_of(iso):
    m = re.match(r"^([+-]?)0*(\d+)-", iso or "")
    if not m:
        return None
    return -int(m.group(2)) if m.group(1) == "-" else int(m.group(2))


def qual_year(st, pid):
    return next(
        (year_of(q["value"]) for q in st["qualifiers"] if q["pid"] == pid), None
    )


def value_of(st):
    """A statement value, as identifiers and raw values."""
    v, kind = st["value"], st["type"]
    if kind == "WikibaseItem":
        return {"item": entity_id(v)}
    if kind == "Quantity":
        return {
            "amount": float(v),
            "unit": None if st["unit"] in (None, "Q199") else st["unit"],
        }
    if kind == "Time":
        return {"time": v, "precision": st["precision"]}
    if kind == "Monolingualtext":
        return {"text": v, "lang": st["value_lang"]}
    if kind == "CommonsMedia":
        return {"file": urllib.parse.unquote(v.rsplit("/", 1)[-1])}
    if kind == "GlobeCoordinate":
        m = re.search(r"(?:entity/(Q\d+)> )?Point\(([-\d.]+) ([-\d.]+)\)", v)
        return {
            "lat": float(m.group(3)),
            "lon": float(m.group(2)),
            "globe": m.group(1) or "Q2",
        }
    if kind == "Url":
        return {"url": v}
    return {"string": v}


def qualifier_of(q):
    v = q["value"]
    if entity_id(v):
        return {"pid": q["pid"], "item": entity_id(v)}
    if re.match(r"^[+-]?\d{4,}-\d\d-\d\dT", v):
        return {"pid": q["pid"], "year": year_of(v)}
    if v.startswith("http://www.wikidata.org/.well-known/genid"):
        return None  # "some value": nothing to show
    if q.get("value_lang"):
        return {"pid": q["pid"], "text": v, "lang": q["value_lang"]}
    try:
        return {"pid": q["pid"], "amount": float(v)}
    except ValueError:
        return {"pid": q["pid"], "string": v}
