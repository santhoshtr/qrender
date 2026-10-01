#!/usr/bin/env python3
"""Layer 1, content: fetch everything the composer and the renderer read.

Two files come out, because two kinds of content change for different
reasons:

- data/content/<qid>.json is the same for every language. It holds the
  statements with identifiers only (no labels), the references, the
  property classes, the container and the peers with their normalized
  quantities, the Commons metadata of every picture, and the sitelinks.
- data/text/<qid>.<lang>.json is the text of one language. It holds the label
  of every identifier that the content uses (with the language each label
  came from), the description, the Wikipedia lead as HTML with a map of its
  links, the section titles of the article, and the page images of items
  without P18 (a page image comes from an article, so it has a language).

Usage: python3 prototype/v3/content.py Q3870 en
"""

import html
import json
import re
import sys
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
CONFIG = tomllib.loads((HERE / "layouts.toml").read_text())
UA = "qrender-prototype/0.3 (sthottingal@wikimedia.org)"
ENTITY = "http://www.wikidata.org/entity/"

# The property that names what contains the item, in order of preference.
# The country comes before the district: a district is often the same place
# as the city, and a comparison with itself says nothing.
CONTAINERS = ("P397", "P171", "P17", "P131", "P361")
PEERS_MIN, PEERS_MAX = 5, 150
ASSESSMENTS = (
    "Featured pictures on Wikimedia Commons",
    "Quality images",
    "Valued images",
)
DISPLAY_IMAGE = "https://wiki-display-image.toolforge.org/json/wikidata/"


# ---------------------------------------------------------------- transport


def get(url, params):
    request = urllib.request.Request(
        url + "?" + urllib.parse.urlencode(params),
        headers={"User-Agent": UA, "Accept": "application/json"},
    )
    for attempt in range(3):
        try:
            return json.loads(urllib.request.urlopen(request, timeout=120).read())
        except urllib.error.HTTPError as error:
            if error.code not in (429, 503, 504) or attempt == 2:
                raise
            time.sleep(10)


def sparql(query):
    time.sleep(1)
    return get("https://query.wikidata.org/sparql", {"query": query, "format": "json"})[
        "results"
    ]["bindings"]


def api(host, **params):
    params.setdefault("format", "json")
    params.setdefault("formatversion", "2")
    return get(f"https://{host}/w/api.php", params)


def local(uri):
    return uri.rsplit("/", 1)[-1]


def entity_id(value):
    """The Q or P identifier of a value, or None when it is not an entity."""
    if value and value.startswith(ENTITY):
        return local(value)
    return None


# --------------------------------------------------------------- statements


def statements(qid):
    """Every statement of the item, grouped by statement node, with
    identifiers and raw values only. No label service: labels are text, and
    text is fetched per language."""
    rows = sparql(f"""
    SELECT ?property ?propertyType ?statement ?value ?valueImage ?valueLogo ?rank
           ?qualifierProperty ?qualifierValue ?unit ?precision WHERE {{
        VALUES ?item {{ wd:{qid} }}
        ?item ?claim ?statement .
        ?property wikibase:claim ?claim ; wikibase:statementProperty ?ps ;
                  wikibase:propertyType ?propertyType ; wikibase:statementValue ?psv .
        ?statement ?ps ?value ; wikibase:rank ?rank .
        OPTIONAL {{ ?value wdt:P18 ?valueImage . }}
        OPTIONAL {{ ?value wdt:P154 ?valueLogo . }}
        OPTIONAL {{ ?statement ?pq ?qualifierValue . ?qualifierProperty wikibase:qualifier ?pq . }}
        OPTIONAL {{ ?statement ?psv ?node . ?node wikibase:quantityUnit ?unit . }}
        OPTIONAL {{ ?statement ?psv ?tnode . ?tnode wikibase:timePrecision ?precision . }}
    }}""")
    out = {}
    for r in rows:
        sid = r["statement"]["value"]
        st = out.setdefault(
            sid,
            {
                "id": local(sid),
                "pid": local(r["property"]["value"]),
                "type": local(r["propertyType"]["value"]).split("#")[-1],
                "value": r["value"]["value"],
                "value_lang": r["value"].get("xml:lang"),
                "rank": local(r["rank"]["value"])
                .split("#")[-1]
                .replace("Rank", "")
                .lower(),
                "unit": entity_id(r.get("unit", {}).get("value")),
                "precision": int(r["precision"]["value"]) if "precision" in r else None,
                "value_image": None,
                "qualifiers": [],
            },
        )
        if "valueImage" in r and not st["value_image"]:
            st["value_image"] = file_name(r["valueImage"]["value"])
        if "valueLogo" in r and not st.get("value_logo"):
            st["value_logo"] = file_name(r["valueLogo"]["value"])
        if "qualifierProperty" in r:
            q = {
                "pid": local(r["qualifierProperty"]["value"]),
                "value": r["qualifierValue"]["value"],
                "value_lang": r["qualifierValue"].get("xml:lang"),
            }
            if q not in st["qualifiers"]:
                st["qualifiers"].append(q)
    # The picture of a value: its image (P18), else its logo (P154). A party,
    # a company or an institution often has only a logo. A logo is marked:
    # it is shown whole, never cropped like a photo.
    for st in out.values():
        logo = st.pop("value_logo", None)
        if logo and not st["value_image"]:
            st["value_image"], st["value_image_from"] = logo, "P154"
    return [s for s in out.values() if s["rank"] != "deprecated"]


def current(sts):
    """The statement that holds now: preferred rank first, then one without
    an end date, then the latest start. A city has had many countries; the
    container is the one it is in today."""

    def year(s, pid):
        v = next((q["value"] for q in s["qualifiers"] if q["pid"] == pid), None)
        m = re.match(r"^([+-]?)0*(\d+)-", v or "")
        return (-1 if m.group(1) == "-" else 1) * int(m.group(2)) if m else None

    return sorted(
        sts,
        key=lambda s: (
            s["rank"] != "preferred",
            year(s, "P582") is not None,
            -(year(s, "P580") or -(10**6)),
        ),
    )[0]


def file_name(url):
    return urllib.parse.unquote(url.rsplit("/", 1)[-1])


# ---------------------------------------------------------------- side data


def classes_among(qids):
    """The value items that are classes: they have a subclass-of (P279)
    statement. "Lawyer", "species", "big city" are classes; their picture
    shows one example of the class, not the thing the statement is about."""
    out = set()
    qids = sorted(qids)
    for i in range(0, len(qids), 200):
        rows = sparql(f"""SELECT DISTINCT ?v WHERE {{
            VALUES ?v {{ {" ".join("wd:" + q for q in qids[i : i + 200])} }} ?v wdt:P279 [] . }}""")
        out |= {local(r["v"]["value"]) for r in rows}
    return sorted(out)


def property_classes(pids):
    rows = sparql(f"""SELECT ?p ?c WHERE {{
        VALUES ?p {{ {" ".join("wd:" + p for p in pids)} }} ?p wdt:P31 ?c . }}""")
    out = {}
    for r in rows:
        out.setdefault(local(r["p"]["value"]), []).append(local(r["c"]["value"]))
    return out


def references(qid):
    """For each statement, the property set of each of its references."""
    rows = sparql(f"""SELECT ?st ?ref ?rp WHERE {{
        wd:{qid} ?claim ?st . ?st prov:wasDerivedFrom ?ref . ?ref ?rp [] .
        FILTER(STRSTARTS(STR(?rp), "http://www.wikidata.org/prop/reference/P")) }}""")
    out = {}
    for r in rows:
        refs = out.setdefault(local(r["st"]["value"]), {})
        refs.setdefault(r["ref"]["value"], []).append(local(r["rp"]["value"]))
    return {st: list(refs.values()) for st, refs in out.items()}


def quantities(qids, pids):
    """Normalized quantities (SI units) of some properties on some items, and
    the picture of each item. A count has no unit, so its raw value is used."""
    rows = sparql(f"""SELECT ?item ?img ?p ?amount ?unit ?rank WHERE {{
        VALUES ?item {{ {" ".join("wd:" + q for q in qids)} }}
        VALUES ?p {{ {" ".join("wd:" + p for p in pids)} }}
        ?p wikibase:claim ?claim ; wikibase:statementValue ?psv ;
           wikibase:statementValueNormalized ?psn .
        ?item ?claim ?st . ?st wikibase:rank ?rank .
        FILTER(?rank != wikibase:DeprecatedRank)
        {{ ?st ?psn ?node . }} UNION {{ ?st ?psv ?node . ?node wikibase:quantityUnit wd:Q199 . }}
        ?node wikibase:quantityAmount ?amount ; wikibase:quantityUnit ?unit .
        OPTIONAL {{ ?item wdt:P18 ?img }} }}""")
    out = {}
    for r in rows:
        q = local(r["item"]["value"])
        rec = out.setdefault(
            q,
            {
                "image": file_name(r["img"]["value"]) if "img" in r else None,
                "values": {},
            },
        )
        p = local(r["p"]["value"])
        preferred = r["rank"]["value"].endswith("PreferredRank")
        old = rec["values"].get(p)
        if old is None or (preferred and not old["preferred"]):
            rec["values"][p] = {
                "amount": float(r["amount"]["value"]),
                "unit": local(r["unit"]["value"]),
                "preferred": preferred,
            }
    return out


def class_layout(sts):
    """The class layout of an item: the first whose match_p31 has one of the
    item's P31 values. The same rule as the composer's."""
    kinds = {entity_id(s["value"]) for s in sts if s["pid"] == "P31"}
    for name, cl in CONFIG["class_layouts"].items():
        if kinds & set(cl["match_p31"]):
            return name
    return "generic"


def reverse_statements(qid, pids):
    """Statements on other items that point to this one: the works of a
    person (author, director, composer, ...). Each with its picture and its
    earliest publication or inception year, for ordering."""
    if not pids:
        return []
    rows = sparql(f"""SELECT ?work ?p ?img ?date WHERE {{
        VALUES ?p {{ {" ".join("wdt:" + p for p in pids)} }}
        ?work ?p wd:{qid} .
        OPTIONAL {{ ?work wdt:P18 ?img }}
        OPTIONAL {{ ?work wdt:P577|wdt:P571 ?date }} }} LIMIT 500""")
    out = {}
    for r in rows:
        w = local(r["work"]["value"])
        rec = out.setdefault(w, {"item": w, "pids": [], "image": None, "year": None})
        pid = local(r["p"]["value"])
        if pid not in rec["pids"]:
            rec["pids"].append(pid)
        if "img" in r and not rec["image"]:
            rec["image"] = file_name(r["img"]["value"])
        if "date" in r:
            m = re.match(r"^([+-]?)0*(\d+)-", r["date"]["value"])
            if m:
                y = int(m.group(2)) * (-1 if m.group(1) == "-" else 1)
                rec["year"] = y if rec["year"] is None else min(rec["year"], y)
    return sorted(out.values(), key=lambda r: (r["year"] is None, r["year"] or 0))


def tabular(url):
    """A Commons data file (a .tab page): its fields and rows, with its
    licence and description."""
    title = urllib.parse.unquote(url.rsplit("/data/main/", 1)[-1]).replace("+", " ")
    data = api(
        "commons.wikimedia.org",
        action="query",
        prop="revisions",
        titles=title,
        rvprop="content",
        rvslots="main",
    )
    page = data["query"]["pages"][0]
    if "revisions" not in page:
        return None
    doc = json.loads(page["revisions"][0]["slots"]["main"]["content"])
    fields = [f["name"] for f in doc.get("schema", {}).get("fields", [])]
    return {
        "title": title,
        "fields": fields,
        "rows": doc.get("data", [])[:1000],
        "license": doc.get("license", ""),
        "description": doc.get("description", {}),
    }


def class_size(cls, cpid=None, container=None):
    """The number of members, counted only up to one past the largest useful
    peer set. A class like "human" has millions; counting them all would
    time out, and past PEERS_MAX the exact number does not matter."""
    extra = f"?m wdt:{cpid} wd:{container} ." if container else ""
    return len(
        sparql(
            f"SELECT DISTINCT ?m WHERE {{ ?m wdt:P31 wd:{cls} . {extra} }} LIMIT {PEERS_MAX + 1}"
        )
    )


def peers(qid, classes, cpid, container, quantity_pids):
    """The most specific useful peer set: class and container together when
    that gives enough members, else the smallest class that does."""
    options = []
    for cls in classes:
        if container:
            n = class_size(cls, cpid, container)
            if PEERS_MIN <= n <= PEERS_MAX:
                options.append((False, n, cls, cpid, container))
                continue
        n = class_size(cls)
        if PEERS_MIN <= n <= PEERS_MAX:
            options.append((True, n, cls, None, None))
    if not options:
        return None
    _, n, cls, cpid, cont = sorted(options)[0]
    extra = f"?m wdt:{cpid} wd:{cont} ." if cont else ""
    members = [
        local(r["m"]["value"])
        for r in sparql(f"SELECT DISTINCT ?m WHERE {{ ?m wdt:P31 wd:{cls} . {extra} }}")
    ]
    frequency = {
        local(r["p"]["value"]): int(r["n"]["value"])
        for r in sparql(f"""
        SELECT ?p (COUNT(DISTINCT ?m) AS ?n) WHERE {{
            ?m wdt:P31 wd:{cls} . {extra}
            ?m ?claim [] . ?p wikibase:claim ?claim ; wikibase:propertyType ?t .
            FILTER(?t != wikibase:ExternalId) }} GROUP BY ?p""")
    }
    enough = max(5, 0.3 * len(members))
    shared = [p for p in quantity_pids if frequency.get(p, 0) >= enough]
    return {
        "class": cls,
        "container_pid": cpid,
        "container": cont,
        "members": members,
        "frequency": frequency,
        "quantities": quantities(members, shared) if shared else {},
    }


def datatypes(pids):
    out = {}
    pids = sorted(pids)
    for i in range(0, len(pids), 50):
        data = api(
            "www.wikidata.org",
            action="wbgetentities",
            ids="|".join(pids[i : i + 50]),
            props="datatype",
        )
        out.update({p: ent.get("datatype") for p, ent in data["entities"].items()})
    return out


def commons_info(files):
    """Size, type, author, licence and community assessment of each file."""
    out = {}
    files = sorted(files)
    for i in range(0, len(files), 50):
        data = api(
            "commons.wikimedia.org",
            action="query",
            prop="imageinfo|categories",
            titles="|".join("File:" + f for f in files[i : i + 50]),
            iiprop="size|mime|url|extmetadata",
            iiextmetadatafilter="Artist|LicenseShortName",
            clcategories="|".join("Category:" + c for c in ASSESSMENTS),
            cllimit="max",
        )
        back = {n["to"]: n["from"] for n in data["query"].get("normalized", [])}
        for page in data["query"]["pages"]:
            if "imageinfo" not in page:
                continue
            info = page["imageinfo"][0]
            meta = info.get("extmetadata", {})
            out[back.get(page["title"], page["title"])[5:]] = {
                "width": info.get("width"),
                "height": info.get("height"),
                "bytes": info["size"],
                "mime": info["mime"],
                "url": info["url"].split("?")[0],
                # Commons gives the author as HTML: drop the tags, decode the
                # entities, so the text is plain text like every other value.
                "artist": html.unescape(
                    re.sub(r"<[^>]+>", "", meta.get("Artist", {}).get("value", ""))
                ).strip(),
                "license": meta.get("LicenseShortName", {}).get("value", ""),
                "assessments": [c["title"][9:] for c in page.get("categories", [])],
            }
    return out


# -------------------------------------------------------------------- text


def labels(ids, lang):
    """The label of every identifier, by the chain {lang}, mul, en, with the
    language each label came from."""
    out = {}
    ids = sorted(ids)
    chain = list(dict.fromkeys((lang, "mul", "en")))
    for i in range(0, len(ids), 50):
        data = api(
            "www.wikidata.org",
            action="wbgetentities",
            ids="|".join(ids[i : i + 50]),
            props="labels|descriptions",
            languages="|".join(chain),
        )
        for q, ent in data["entities"].items():
            for code in chain:
                if code in ent.get("labels", {}):
                    out[q] = {"label": ent["labels"][code]["value"], "lang": code}
                    break
            for code in chain:
                if code in ent.get("descriptions", {}):
                    out.setdefault(q, {})["description"] = ent["descriptions"][code][
                        "value"
                    ]
                    out[q]["description_lang"] = code
                    break
    return out


def lead(host, title, lang):
    """The lead of an article as HTML paragraphs, the map of its links, and
    the titles of its sections."""
    data = api(
        host,
        action="parse",
        page=title,
        section="0",
        prop="text",
        disableeditsection="1",
        disablelimitreport="1",
        redirects="1",
    )
    html = data["parse"]["text"]
    html = re.sub(r"<table.*?</table>", "", html, flags=re.S)
    html = re.sub(
        r'<sup[^>]*class="[^"]*reference[^"]*".*?</sup>', "", html, flags=re.S
    )
    html = re.sub(r"<style.*?</style>", "", html, flags=re.S)
    paragraphs = [
        p
        for p in re.findall(r"<p>(.*?)</p>", html, flags=re.S)
        if re.sub(r"<[^>]+>|\s", "", p)
    ]
    sections = api(host, action="parse", page=title, prop="sections", redirects="1")[
        "parse"
    ]["sections"]
    return {
        "title": title,
        "paragraphs": paragraphs,
        "links": resolve_links(host, paragraphs),
        "sections": [
            {
                "title": re.sub(r"<[^>]+>", "", s["line"]),
                "anchor": s["anchor"],
                "index": s["index"],
            }
            for s in sections
            if s["toclevel"] == 1
        ],
    }


def resolve_links(host, paragraphs):
    """The Wikidata item of every article a paragraph links to."""
    titles = sorted(
        {
            urllib.parse.unquote(m).replace("_", " ")
            for p in paragraphs
            for m in re.findall(r'href="/wiki/([^"#?]+)', p)
            if ":" not in urllib.parse.unquote(m)
        }
    )
    by_title = {}
    for i in range(0, len(titles), 50):
        d = api(
            host,
            action="query",
            prop="pageprops",
            ppprop="wikibase_item",
            titles="|".join(titles[i : i + 50]),
            redirects="1",
        )
        back = {r["to"]: r["from"] for r in d["query"].get("redirects", [])}
        back.update({n["to"]: n["from"] for n in d["query"].get("normalized", [])})
        for page in d["query"]["pages"]:
            q, t = page.get("pageprops", {}).get("wikibase_item"), page["title"]
            while q and t:
                by_title[t] = q
                t = back.get(t)
    return by_title


def concepts(lang):
    """Labels and aliases of the items that title the spreads and name their
    prose sections, in one language."""
    qids = sorted(
        {
            q
            for cl in CONFIG["class_layouts"].values()
            for sp in cl["spreads"]
            for q in [sp.get("concept"), *sp.get("prose", [])]
            if q
        }
    )
    data = api(
        "www.wikidata.org",
        action="wbgetentities",
        ids="|".join(qids),
        props="labels|aliases",
        languages="|".join(dict.fromkeys((lang, "mul", "en"))),
    )
    out = {}
    for q, ent in data["entities"].items():
        for code in dict.fromkeys((lang, "mul", "en")):
            if code in ent.get("labels", {}):
                out[q] = {"label": ent["labels"][code]["value"], "lang": code}
                break
        out.setdefault(q, {"label": q, "lang": None})
        label = (
            {ent["labels"][lang]["value"]} if lang in ent.get("labels", {}) else set()
        )
        out[q]["names"] = sorted(
            label | {a["value"] for a in ent.get("aliases", {}).get(lang, [])}
        )
    return out


def section_text(host, title, index):
    """The first paragraph of one section of an article, as HTML."""
    data = api(
        host,
        action="parse",
        page=title,
        section=str(index),
        prop="text",
        disableeditsection="1",
        disablelimitreport="1",
        redirects="1",
    )
    html_ = data["parse"]["text"]
    html_ = re.sub(r"<table.*?</table>", "", html_, flags=re.S)
    html_ = re.sub(
        r'<sup[^>]*class="[^"]*reference[^"]*".*?</sup>', "", html_, flags=re.S
    )
    html_ = re.sub(r"<style.*?</style>", "", html_, flags=re.S)
    paragraphs = [
        p
        for p in re.findall(r"<p>(.*?)</p>", html_, flags=re.S)
        if re.sub(r"<[^>]+>|\s", "", p)
    ]
    return paragraphs[0] if paragraphs else None


def section_texts(host, article):
    """The first paragraph of every matched section, and the link map of the
    whole article extended with their links."""
    out = {}
    anchors = {a for hits in article.get("concept_sections", {}).values() for a in hits}
    for s in article["sections"]:
        if s["anchor"] in anchors:
            text = section_text(host, article["title"], s["index"])
            if text:
                out[s["anchor"]] = text
    return out


def match_sections(article, names):
    """The sections of an article whose title is a label or an alias of a
    concept, in the language of the article. No other matching: a title that
    names no concept stays unmatched and is listed as it is."""
    out = {}
    for q, c in names.items():
        wanted = {n.casefold() for n in c["names"]}
        hits = [
            s["anchor"] for s in article["sections"] if s["title"].casefold() in wanted
        ]
        if hits:
            out[q] = hits
    return out


def page_images(qids, lang):
    def one(qid):
        for code in dict.fromkeys((lang, "en")):
            try:
                data = get(DISPLAY_IMAGE + qid, {"lang": code, "fallback": "none"})
            except urllib.error.HTTPError:
                continue
            if "file" in data:
                return qid, {
                    "file": data["file"][5:],
                    "width": data["original"]["width"],
                    "height": data["original"]["height"],
                    "artist": data.get("artist", ""),
                    "license": data.get("license", ""),
                    "source": f"{code}.wikipedia.org",
                }
        return qid, None

    with ThreadPoolExecutor(8) as pool:
        return {q: v for q, v in pool.map(one, sorted(qids)) if v}


# -------------------------------------------------------------------- main


def fetch_content(qid):
    sts = statements(qid)
    pids = sorted(
        {s["pid"] for s in sts} | {q["pid"] for s in sts for q in s["qualifiers"]}
    )
    quantity_pids = sorted({s["pid"] for s in sts if s["type"] == "Quantity"})
    cpid = next(
        (
            p
            for p in CONTAINERS
            if any(s["pid"] == p and entity_id(s["value"]) for s in sts)
        ),
        None,
    )
    container = (
        entity_id(current([s for s in sts if s["pid"] == cpid])["value"])
        if cpid
        else None
    )
    classes = [
        entity_id(s["value"])
        for s in sts
        if s["pid"] == "P31" and entity_id(s["value"])
    ]
    peer_set = peers(qid, classes, cpid, container, quantity_pids)
    if peer_set:
        have = {s["pid"] for s in sts}
        missing = [
            p
            for p, n in peer_set["frequency"].items()
            if n >= 0.6 * len(peer_set["members"]) and p not in have
        ]
        peer_set["missing"] = datatypes(missing) if missing else {}
    layout = class_layout(sts)
    reverse_pids = [
        p
        for sp in CONFIG["class_layouts"][layout]["spreads"]
        for p in sp.get("reverse", [])
    ]
    reverse = reverse_statements(qid, reverse_pids)
    tables = {
        s["value"]: tabular(s["value"]) for s in sts if s["type"] == "TabularData"
    }
    media = {file_name(s["value"]) for s in sts if s["type"] == "CommonsMedia"}
    media |= {s["value_image"] for s in sts if s["value_image"]}
    media |= {r["image"] for r in reverse if r["image"]}
    targets = [qid] + ([container] if container else [])
    sitelinks = api(
        "www.wikidata.org", action="wbgetentities", ids=qid, props="sitelinks"
    )["entities"][qid]["sitelinks"]
    return {
        "qid": qid,
        "statements": sts,
        "references": references(qid),
        "property_classes": property_classes(pids),
        "container": {"pid": cpid, "qid": container},
        "quantities": quantities(targets, quantity_pids) if quantity_pids else {},
        "peers": peer_set,
        "commons": commons_info(media),
        "value_classes": classes_among(
            {
                entity_id(s["value"])
                for s in sts
                if s["type"] == "WikibaseItem" and entity_id(s["value"])
            }
        ),
        "sitelinks": {k: v["title"] for k, v in sitelinks.items()},
        "class_layout": layout,
        "reverse": reverse,
        "tabular": {k: v for k, v in tables.items() if v},
    }


def identifiers(content):
    """Every identifier whose label the page may show."""
    ids = {content["qid"]}
    for s in content["statements"]:
        ids.add(s["pid"])
        for v in (s["value"], s["unit"] and ENTITY + s["unit"]):
            if entity_id(v):
                ids.add(entity_id(v))
        for q in s["qualifiers"]:
            ids.add(q["pid"])
            if entity_id(q["value"]):
                ids.add(entity_id(q["value"]))
    peer_set = content["peers"] or {}
    ids |= set(peer_set.get("members", [])) | set(peer_set.get("missing", {}))
    if peer_set.get("class"):
        ids.add(peer_set["class"])
    if content["container"]["qid"]:
        ids.add(content["container"]["qid"])
    for rec in content["quantities"].values():
        ids |= {v["unit"] for v in rec["values"].values()}
    # The page names the kinds of provenance with Wikidata's own reference
    # properties: stated in, reference URL, imported from Wikimedia project.
    ids |= {"P248", "P854", "P143"}
    # The word for an age, from Wikidata's own property "age at event".
    ids.add("P3629")
    for r in content.get("reverse", []):
        ids.add(r["item"])
        ids |= set(r["pids"])
    return {i for i in ids if re.fullmatch(r"[PQ]\d+", i)}


def fetch_text(content, lang):
    ids = identifiers(content)
    text = {"lang": lang, "labels": labels(ids, lang), "concepts": concepts(lang)}
    title = content["sitelinks"].get(f"{lang}wiki")
    text["article"] = lead(f"{lang}.wikipedia.org", title, lang) if title else None
    if text["article"]:
        text["article"]["concept_sections"] = match_sections(
            text["article"], text["concepts"]
        )
        text["article"]["section_text"] = section_texts(
            f"{lang}.wikipedia.org", text["article"]
        )
        text["article"]["links"].update(
            resolve_links(
                f"{lang}.wikipedia.org", list(text["article"]["section_text"].values())
            )
        )
    if lang != "en" and content["sitelinks"].get("enwiki"):
        text["fallback_article"] = lead(
            "en.wikipedia.org", content["sitelinks"]["enwiki"], lang
        )
        text["fallback_article"]["concept_sections"] = match_sections(
            text["fallback_article"], concepts("en")
        )
        text["fallback_article"]["section_text"] = section_texts(
            "en.wikipedia.org", text["fallback_article"]
        )
        text["fallback_article"]["links"].update(
            resolve_links(
                "en.wikipedia.org",
                list(text["fallback_article"]["section_text"].values()),
            )
        )
    unpictured = {
        entity_id(s["value"])
        for s in content["statements"]
        if s["type"] == "WikibaseItem"
        and entity_id(s["value"])
        and not s["value_image"]
    }
    text["page_images"] = page_images(unpictured, lang)
    return text


def main(qid, lang):
    content_path = HERE / "data" / "content" / f"{qid.lower()}.json"
    content_path.parent.mkdir(parents=True, exist_ok=True)
    if content_path.exists() and "--refresh" not in sys.argv:
        content = json.loads(content_path.read_text())
    else:
        content = fetch_content(qid)
        content_path.write_text(json.dumps(content, ensure_ascii=False, indent=1))
    text_path = HERE / "data" / "text" / f"{qid.lower()}.{lang}.json"
    text_path.parent.mkdir(parents=True, exist_ok=True)
    text = fetch_text(content, lang)
    text_path.write_text(json.dumps(text, ensure_ascii=False, indent=1))
    print(
        f"content: {len(content['statements'])} statements, {len(content['commons'])} files, "
        f"peers {content['peers'] and (content['peers']['class'], len(content['peers']['members']))} "
        f"-> {content_path.name} ({content_path.stat().st_size:,} B)"
    )
    print(
        f"text {lang}: {len(text['labels'])} labels, article {bool(text['article'])} "
        f"-> {text_path.name} ({text_path.stat().st_size:,} B)"
    )


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
