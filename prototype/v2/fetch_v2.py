#!/usr/bin/env python3
"""Fetch the side data that the second-generation composer needs.

The qjson query gives the statements of one item. The composer also asks four
questions that the item alone cannot answer, and each has one generic query:

1. What kind of property is each one?  The classes (P31) of each property,
   which Wikidata editors maintain. They decide the dimension of a card.
2. How well is each statement sourced?  The properties of every reference,
   so that a reference "imported from" a Wikipedia is not counted as a source.
3. How big is it, compared with what contains it?  The normalized quantities
   of the container item (parent body, parent taxon, country), and of the
   item itself.
4. Where does it sit among its peers, and what do peers have that it lacks?
   The members of the most specific class that gives a useful set, with their
   quantities, their pictures, and how often each property occurs among them.

It also fetches the lead of the Wikipedia article as HTML, in the page
language when the article exists and in English, with a map from every link
to its Wikidata item, its local article and its label in the page language.

Everything goes into one file, prototype/v2/data/<qid>-<lang>.json.

Usage: python3 prototype/v2/fetch_v2.py Q405 ig
"""

import json
import re
import sys
import time
import tomllib
from concurrent.futures import ThreadPoolExecutor
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "v1"))
from factoid_kit import load, statements, value_qid  # noqa: E402

ROOT = Path(__file__).resolve().parent
V1 = ROOT.parent / "v1"
UA = "qrender-prototype/0.2 (sthottingal@wikimedia.org)"

# The fixture of each item, as the first prototypes fetched it.
FIXTURES = {
    ("Q405", "ig"): "q405-ig.sparql.json",
    ("Q3870", "gu"): "q3870-gu.sparql.json",
    ("Q41181", "hi"): "q41181-hi.sparql.json",
}

# The property that names what contains the item, in order of preference.
# The country comes before the district: a district is often the same place
# as the city, and a comparison with itself says nothing.
CONTAINERS = ("P397", "P171", "P17", "P131", "P361")

# A peer set outside these bounds says nothing: too few to compare, too many
# to show or to query.
PEERS_MIN, PEERS_MAX = 5, 150


def get(url, params):
    request = urllib.request.Request(
        url + "?" + urllib.parse.urlencode(params),
        headers={"User-Agent": UA, "Accept": "application/json"})
    for attempt in range(3):
        try:
            return json.loads(urllib.request.urlopen(request, timeout=120).read())
        except urllib.error.HTTPError as error:
            if error.code != 429 or attempt == 2:
                raise
            time.sleep(10)


def sparql(query):
    time.sleep(1)
    rows = get("https://query.wikidata.org/sparql", {"query": query, "format": "json"})
    return rows["results"]["bindings"]


def entity(uri):
    return uri.rsplit("/", 1)[-1]


def api(host, **params):
    params.setdefault("format", "json")
    params.setdefault("formatversion", "2")
    return get(f"https://{host}/w/api.php", params)


# ------------------------------------------------------------------ queries

def property_classes(pids):
    rows = sparql(f"""SELECT ?p ?c WHERE {{
        VALUES ?p {{ {" ".join("wd:" + p for p in pids)} }} ?p wdt:P31 ?c . }}""")
    out = {}
    for r in rows:
        out.setdefault(entity(r["p"]["value"]), []).append(entity(r["c"]["value"]))
    return out


def references(qid):
    """For each statement, the property set of each of its references."""
    rows = sparql(f"""SELECT ?st ?ref ?rp WHERE {{
        wd:{qid} ?claim ?st . ?st prov:wasDerivedFrom ?ref . ?ref ?rp [] .
        FILTER(STRSTARTS(STR(?rp), "http://www.wikidata.org/prop/reference/P")) }}""")
    out = {}
    for r in rows:
        refs = out.setdefault(r["st"]["value"], {})
        refs.setdefault(r["ref"]["value"], []).append(entity(r["rp"]["value"]))
    return {st: list(refs.values()) for st, refs in out.items()}


def quantities(qids, pids, lang):
    """Normalized quantities (SI units) of some properties on some items,
    preferred and normal rank only. Also the label and picture of each item."""
    rows = sparql(f"""SELECT ?item ?itemLabel ?img ?p ?amount ?unitLabel ?rank WHERE {{
        VALUES ?item {{ {" ".join("wd:" + q for q in qids)} }}
        VALUES ?p {{ {" ".join("wd:" + p for p in pids)} }}
        ?p wikibase:claim ?claim ; wikibase:statementValue ?psv ;
           wikibase:statementValueNormalized ?psn .
        ?item ?claim ?st . ?st wikibase:rank ?rank .
        FILTER(?rank != wikibase:DeprecatedRank)
        # A quantity without a unit (a count) has no normalized value, so
        # take its raw value.
        {{ ?st ?psn ?node . }} UNION {{ ?st ?psv ?node . ?node wikibase:quantityUnit wd:Q199 . }}
        ?node wikibase:quantityAmount ?amount ; wikibase:quantityUnit ?unit .
        OPTIONAL {{ ?item wdt:P18 ?img }}
        SERVICE wikibase:label {{ bd:serviceParam wikibase:language "{lang}, mul, en" .
            ?item rdfs:label ?itemLabel . ?unit rdfs:label ?unitLabel . }} }}""")
    out = {}
    for r in rows:
        q = entity(r["item"]["value"])
        rec = out.setdefault(q, {"label": r["itemLabel"]["value"],
                                 "label_lang": r["itemLabel"].get("xml:lang"),
                                 "image": r.get("img", {}).get("value"), "values": {}})
        p = entity(r["p"]["value"])
        preferred = r["rank"]["value"].endswith("PreferredRank")
        old = rec["values"].get(p)
        # One value per property: preferred first, else the first one seen.
        if old is None or (preferred and not old["preferred"]):
            rec["values"][p] = {"amount": float(r["amount"]["value"]),
                                "unit": r["unitLabel"]["value"], "preferred": preferred}
    return out


def class_size(cls, container_pid=None, container=None):
    extra = f"?m wdt:{container_pid} wd:{container} ." if container else ""
    rows = sparql(f"SELECT (COUNT(DISTINCT ?m) AS ?n) WHERE {{ ?m wdt:P31 wd:{cls} . {extra} }}")
    return int(rows[0]["n"]["value"])


def choose_peers(qid, classes, container_pid, container):
    """The most specific useful peer set: class and container together if
    that gives enough members, else the smallest class that does."""
    options = []
    for cls in classes:
        if container:
            n = class_size(cls, container_pid, container)
            if PEERS_MIN <= n <= PEERS_MAX:
                options.append((n, cls, container_pid, container))
                continue
        n = class_size(cls)
        if PEERS_MIN <= n <= PEERS_MAX:
            options.append((n, cls, None, None))
    if not options:
        return None
    # Container-bound sets first (they share the item's context), then size.
    options.sort(key=lambda o: (o[2] is None, o[0]))
    n, cls, cpid, cont = options[0]
    extra = f"?m wdt:{cpid} wd:{cont} ." if cont else ""
    members = [entity(r["m"]["value"]) for r in sparql(
        f"SELECT DISTINCT ?m WHERE {{ ?m wdt:P31 wd:{cls} . {extra} }}")]
    frequency = {entity(r["p"]["value"]): int(r["n"]["value"]) for r in sparql(f"""
        SELECT ?p (COUNT(DISTINCT ?m) AS ?n) WHERE {{
            ?m wdt:P31 wd:{cls} . {extra}
            ?m ?claim [] . ?p wikibase:claim ?claim ; wikibase:propertyType ?t .
            FILTER(?t != wikibase:ExternalId) }} GROUP BY ?p""")}
    return {"class": cls, "container_pid": cpid, "container": cont,
            "members": members, "frequency": frequency}


def labels(qids, lang):
    out = {}
    qids = list(qids)
    for i in range(0, len(qids), 50):
        data = api("www.wikidata.org", action="wbgetentities", ids="|".join(qids[i:i + 50]),
                   props="labels", languages=f"{lang}|mul|en")
        for q, ent in data["entities"].items():
            for code in (lang, "mul", "en"):
                if code in ent.get("labels", {}):
                    out[q] = {"label": ent["labels"][code]["value"], "lang": code}
                    break
    return out


# --------------------------------------------------------------------- lead

def lead_html(host, title):
    """The lead section of an article as HTML: its paragraphs only, without
    the infobox, the references and the maintenance templates."""
    data = api(host, action="parse", page=title, section="0", prop="text",
               disableeditsection="1", disablelimitreport="1", redirects="1")
    html = data["parse"]["text"]
    html = re.sub(r"<table.*?</table>", "", html, flags=re.S)
    html = re.sub(r'<sup[^>]*class="[^"]*reference[^"]*".*?</sup>', "", html, flags=re.S)
    html = re.sub(r"<style.*?</style>", "", html, flags=re.S)
    paragraphs = [p for p in re.findall(r"<p>(.*?)</p>", html, flags=re.S)
                  if re.sub(r"<[^>]+>|\s", "", p)]
    return paragraphs


def link_targets(paragraphs, wiki, lang):
    """Every link in the lead, resolved to its Wikidata item, the article in
    the page language, and the label in the page language."""
    titles = sorted({urllib.parse.unquote(m).replace("_", " ")
                     for p in paragraphs for m in re.findall(r'href="/wiki/([^"#?]+)', p)
                     if ":" not in urllib.parse.unquote(m)})
    # The wiki knows which item each of its pages belongs to.
    by_title = {}
    for i in range(0, len(titles), 50):
        data = api(f"{wiki[:-4]}.wikipedia.org", action="query", prop="pageprops",
                   ppprop="wikibase_item", titles="|".join(titles[i:i + 50]), redirects="1")
        back = {r["to"]: r["from"] for r in data["query"].get("redirects", [])}
        back.update({n["to"]: n["from"] for n in data["query"].get("normalized", [])})
        for page in data["query"]["pages"]:
            q = page.get("pageprops", {}).get("wikibase_item")
            title = page["title"]
            while q and title:
                by_title[title] = q
                title = back.get(title)
    items = {}
    qids = sorted(set(by_title.values()))
    for i in range(0, len(qids), 50):
        data = api("www.wikidata.org", action="wbgetentities", ids="|".join(qids[i:i + 50]),
                   props="sitelinks|labels", languages=lang, sitefilter=f"{lang}wiki")
        for q, ent in data["entities"].items():
            items[q] = {"local": ent.get("sitelinks", {}).get(f"{lang}wiki", {}).get("title"),
                        "label": ent.get("labels", {}).get(lang, {}).get("value")}
    return {"titles": by_title, "items": items}


def commons_info(files):
    """Size, type, author and licence of each Commons file, so that every
    picture on the page can carry its attribution."""
    out = {}
    files = sorted(files)
    for i in range(0, len(files), 50):
        data = api("commons.wikimedia.org", action="query", prop="imageinfo|categories",
                   titles="|".join("File:" + f for f in files[i:i + 50]),
                   iiprop="size|mime|url|extmetadata",
                   iiextmetadatafilter="Artist|LicenseShortName|ImageDescription",
                   clcategories="|".join("Category:" + c for c in ASSESSMENTS), cllimit="max")
        back = {n["to"]: n["from"] for n in data["query"].get("normalized", [])}
        for page in data["query"]["pages"]:
            if "imageinfo" not in page:
                continue
            info = page["imageinfo"][0]
            meta = info.get("extmetadata", {})
            name = back.get(page["title"], page["title"])[5:]
            out[name] = {
                "size": info["size"], "mime": info["mime"], "url": info["url"].split("?")[0],
                "width": info.get("width"), "height": info.get("height"),
                "artist": re.sub(r"<[^>]+>", "", meta.get("Artist", {}).get("value", "")).strip(),
                "license": meta.get("LicenseShortName", {}).get("value", ""),
                "description": re.sub(r"<[^>]+>", "", meta.get("ImageDescription", {}).get("value", "")).strip()[:300],
                # The community's own judgment of the picture, where it has one.
                "assessments": [c["title"][9:] for c in page.get("categories", [])],
            }
    return out


# Commons categories that record a community assessment of a picture.
ASSESSMENTS = ("Featured pictures on Wikimedia Commons", "Quality images", "Valued images")

DISPLAY_IMAGE = "https://wiki-display-image.toolforge.org/json/wikidata/"


def page_images(qids, lang):
    """A picture for items that have no P18: the page image of their article,
    in the page language first, then in English, from wiki-display-image.
    This is not Wikidata, so the page marks where it came from."""
    def one(qid):
        for code in dict.fromkeys((lang, "en")):
            try:
                data = get(DISPLAY_IMAGE + qid, {"lang": code, "fallback": "none"})
            except urllib.error.HTTPError:
                continue
            if "file" in data:
                return qid, {"file": data["file"][5:], "width": data["original"]["width"],
                             "height": data["original"]["height"], "artist": data.get("artist", ""),
                             "license": data.get("license", ""), "source": f"{code}.wikipedia.org"}
        return qid, None
    with ThreadPoolExecutor(8) as pool:
        return {q: v for q, v in pool.map(one, sorted(qids)) if v}


def datatypes(pids):
    out = {}
    pids = list(pids)
    for i in range(0, len(pids), 50):
        data = api("www.wikidata.org", action="wbgetentities", ids="|".join(pids[i:i + 50]), props="datatype")
        out.update({p: ent.get("datatype") for p, ent in data["entities"].items()})
    return out


def concept_labels(lang):
    """Labels of the items that name the dimensions in v2.toml, and of the
    reference properties that name the kinds of provenance."""
    config = tomllib.loads((ROOT / "v2.toml").read_text())
    reference_pids = ["P248", "P854", "P143", "P887"]
    # Names for the page's own parts: Wikidata property, Wikidata item,
    # Wikimedia project.
    parts = ["Q18616576", "Q16222597", "Q14827288"]
    return labels([d["concept"] for d in config["dimensions"]] + reference_pids + parts, lang)


# --------------------------------------------------------------------- main

def main(qid, lang):
    load(V1 / "data" / FIXTURES[(qid, lang)])
    sts = [s for s in statements() if s["rank"] != "Deprecated"]
    pids = sorted({s["pid"] for s in sts} | {q["pid"] for s in sts for q in s["quals"]})
    quantity_pids = sorted({s["pid"] for s in sts if s["ptype"] == "Quantity"})

    bundle = {"qid": qid, "lang": lang}
    bundle["property_classes"] = property_classes(pids)
    bundle["references"] = references(qid)

    container_pid = next((p for p in CONTAINERS if any(s["pid"] == p and value_qid(s) for s in sts)), None)
    container = next((value_qid(s) for s in sts if s["pid"] == container_pid), None) if container_pid else None
    bundle["container"] = {"pid": container_pid, "qid": container}
    targets = [qid] + ([container] if container else [])
    bundle["quantities"] = quantities(targets, quantity_pids, lang) if quantity_pids else {}

    classes = [value_qid(s) for s in sts if s["pid"] == "P31" and value_qid(s)]
    peers = choose_peers(qid, classes, container_pid, container)
    if peers:
        # Compare on the properties that the item has and most peers have.
        enough = max(5, 0.3 * len(peers["members"]))
        shared = [p for p in quantity_pids if peers["frequency"].get(p, 0) >= enough]
        peers["quantities"] = quantities(peers["members"], shared, lang) if shared else {}
        missing = [p for p, n in peers["frequency"].items()
                   if n >= 0.6 * len(peers["members"]) and p not in {s["pid"] for s in sts}]
        peers["missing_labels"] = labels(missing, lang) if missing else {}
        peers["missing_types"] = datatypes(missing) if missing else {}
        peers["class_label"] = labels([peers["class"]], lang).get(peers["class"])
    bundle["peers"] = peers

    sitelinks = api("www.wikidata.org", action="wbgetentities", ids=qid,
                    props="sitelinks")["entities"][qid]["sitelinks"]
    bundle["sitelinks"] = {k: v["title"] for k, v in sitelinks.items()}
    leads = {}
    for code in dict.fromkeys((lang, "en")):
        title = bundle["sitelinks"].get(f"{code}wiki")
        if title:
            paragraphs = lead_html(f"{code}.wikipedia.org", title)
            leads[code] = {"title": title, "paragraphs": paragraphs,
                           "links": link_targets(paragraphs, f"{code}wiki", lang)}
    bundle["leads"] = leads
    bundle["concepts"] = concept_labels(lang)
    # Every picture the page can show: the item's own files and the P18 of
    # each item it points to. Their shape decides their box.
    media = {urllib.parse.unquote(s["value"].rsplit("/", 1)[-1])
             for s in sts if s["ptype"] == "CommonsMedia"}
    media |= {urllib.parse.unquote(s["vimg"].rsplit("/", 1)[-1]) for s in sts if s.get("vimg")}
    bundle["commons"] = commons_info(media)
    unpictured = {value_qid(s) for s in sts if s["ptype"] == "WikibaseItem"
                  and value_qid(s) and not s.get("vimg")}
    bundle["page_images"] = page_images(unpictured, lang)

    out = ROOT / "data" / f"{qid.lower()}-{lang}.json"
    out.parent.mkdir(exist_ok=True)
    out.write_text(json.dumps(bundle, ensure_ascii=False, indent=1))
    print(f"wrote {out}: {len(bundle['references'])} referenced statements, "
          f"peers {peers and (peers['class'], len(peers['members']))}, "
          f"leads {list(leads)}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
